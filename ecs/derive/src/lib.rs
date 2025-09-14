use proc_macro::TokenStream;
use proc_macro2::{Delimiter, Ident, TokenStream as TokenStream2, TokenTree};
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

#[proc_macro_derive(Component)]
pub fn derive_component(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let type_name = name.to_string();

    quote! {
        impl Component for #name {
            fn get_type_id(&self) -> usize {
                get_component_id::<Self>()
            }

            fn as_any(&self) -> &dyn std::any::Any {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }
        }

        submit! {
            ComponentRegistration {
                type_id: ConstTypeId::of::<#name>(),
                name: #type_name,
            }
        }
    }
    .into()
}

#[proc_macro_derive(Resource)]
pub fn derive_resource(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let type_name = name.to_string();

    quote! {
        impl Resource for #name {
            fn get_type_id(&self) -> usize {
                get_resource_id::<Self>()
            }

            fn as_any(&self) -> &dyn std::any::Any {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }
        }

        submit! {
            ResourceRegistration {
                type_id: ConstTypeId::of::<#name>(),
                name: #type_name,
            }
        }
    }
    .into()
}

/**
 * Systems look like
 *
 * ```ignore
 * #[system]
 * fn my_system(
 *    query: query (&mut Transform, &Velocity),
 *    time: res &Time,
 * ) {
 *    // query is of type Vec<(&mut Transform, &Velocity)>
 *    // time is of type Option<&Time>
 *    for (transform, velocity) in query.iter() {
 *        transform.position += velocity.0 * time.delta_seconds;
 *    }
 * }
 * ```
 */
#[proc_macro]
pub fn system(item: TokenStream) -> TokenStream {
    let item2: TokenStream2 = item.into();

    let (fn_name, args, body) = parse_function(item2);

    let mut shared_components = Vec::new();
    let mut mutable_components = Vec::new();
    let mut shared_resources = Vec::new();
    let mut mutable_resources = Vec::new();

    let (arg_gather_tokens, runs_alone) = generate_arg_gather(
        args,
        &mut shared_components,
        &mut mutable_components,
        &mut shared_resources,
        &mut mutable_resources,
    );

    let component_access = component_access(&shared_components, &mutable_components);
    let resource_access = resource_access(&shared_resources, &mutable_resources);

    let last_run_ident = quote::format_ident!("LAST_RUN{}", fn_name.to_string().to_uppercase());

    let expanded = quote! {
        #[allow(non_camel_case_types)]
        pub struct #fn_name;

        static #last_run_ident: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

        impl System for #fn_name {
            unsafe fn run_unsafe(&mut self, world: *mut World) {
                #arg_gather_tokens
                {
                    #body
                }
            }

            fn name(&self) -> &'static str {
                stringify!(#fn_name)
            }

            fn component_access(&self) -> &'static ComponentAccess {
                lazy_static::lazy_static! {
                    static ref CA: ComponentAccess = #component_access;
                }
                &CA
            }

            fn resource_access(&self) -> &'static ResourceAccess {
                lazy_static::lazy_static! {
                    static ref RA: ResourceAccess = #resource_access;
                }
                &RA
            }

            fn get_last_run(&self) -> Tick {
                #last_run_ident.load(std::sync::atomic::Ordering::SeqCst)
            }

            fn set_last_run(&mut self, tick: Tick) {
                #last_run_ident.store(tick, std::sync::atomic::Ordering::SeqCst);
            }

            fn runs_alone(&self) -> bool {
                #runs_alone
            }
        }
    };

    expanded.into()
}

fn component_access(shared_components: &[Ident], mutable_components: &[Ident]) -> TokenStream2 {
    let mut read_ids = Vec::new();
    let mut write_ids = Vec::new();

    for comp in shared_components {
        read_ids.push(quote! { get_component_id::<#comp>() });
    }

    for comp in mutable_components {
        write_ids.push(quote! { get_component_id::<#comp>() });
    }

    quote! {
        ComponentAccess {
            read: Box::leak(Box::new([#(#read_ids),*])) as &'static [usize],
            write: Box::leak(Box::new([#(#write_ids),*])) as &'static [usize],
        }
    }
}

fn resource_access(shared_resources: &[Ident], mutable_resources: &[Ident]) -> TokenStream2 {
    let mut read_ids = Vec::new();
    let mut write_ids = Vec::new();

    for res in shared_resources {
        read_ids.push(quote! { get_resource_id::<#res>() });
    }

    for res in mutable_resources {
        write_ids.push(quote! { get_resource_id::<#res>() });
    }

    quote! {
        ResourceAccess {
            read: Box::leak(Box::new([#(#read_ids),*])) as &'static [usize],
            write: Box::leak(Box::new([#(#write_ids),*])) as &'static [usize],
        }
    }
}

fn parse_function(item2: TokenStream2) -> (proc_macro2::Ident, Vec<TokenTree>, TokenStream2) {
    let mut tokens = item2.into_iter();
    let mut fn_name = None;
    let mut args = Vec::new();
    let mut body = TokenStream2::new();
    let mut found_fn = false;

    while let Some(tt) = tokens.next() {
        match &tt {
            TokenTree::Ident(ident) if ident == "fn" && !found_fn => {
                found_fn = true;
            }
            TokenTree::Ident(ident) if found_fn && fn_name.is_none() => {
                fn_name = Some(ident.clone());
            }
            TokenTree::Group(group)
                if group.delimiter() == Delimiter::Parenthesis && fn_name.is_some() =>
            {
                args = group.stream().into_iter().collect();
            }
            TokenTree::Group(group) if group.delimiter() == Delimiter::Brace => {
                body = group.stream();
            }
            _ => {}
        }
    }

    let fn_name = fn_name.expect("Could not find function name");
    (fn_name, args, body)
}

fn generate_arg_gather(
    args: Vec<TokenTree>,
    shared_components: &mut Vec<Ident>,
    mutable_components: &mut Vec<Ident>,
    shared_resources: &mut Vec<Ident>,
    mutable_resources: &mut Vec<Ident>,
) -> (TokenStream2, bool) {
    let mut arg_gather = Vec::new();
    let mut arg_iter = args.into_iter().peekable();
    let mut runs_alone = false;

    while let Some(tt) = arg_iter.next() {
        if let TokenTree::Ident(arg_name) = &tt {
            if let Some(TokenTree::Punct(p)) = arg_iter.peek() {
                if p.as_char() == ':' {
                    arg_iter.next();
                    if let Some(TokenTree::Ident(ty_ident)) = arg_iter.next() {
                        let ty_str = ty_ident.to_string();
                        if ty_str == "query" {
                            if let Some(gather_code) = handle_query(
                                &mut arg_iter,
                                arg_name,
                                shared_components,
                                mutable_components,
                            ) {
                                arg_gather.push(gather_code);
                            }
                        } else if ty_str == "res" {
                            if let Some(gather_code) = handle_resource(
                                &mut arg_iter,
                                arg_name,
                                shared_resources,
                                mutable_resources,
                            ) {
                                arg_gather.push(gather_code);
                            }
                        } else if ty_str == "commands" {
                            if runs_alone {
                                panic!("commands can only be specified once");
                            }
                            runs_alone = true;
                            arg_gather.push(quote! {
                                let mut #arg_name = Commands::new(world);
                            });
                        } else {
                            panic!("Unknown argument type: {}", ty_str);
                        }
                    }
                }
            }
        }
    }

    (quote! { #(#arg_gather)* }, runs_alone)
}

fn handle_query(
    arg_iter: &mut std::iter::Peekable<std::vec::IntoIter<TokenTree>>,
    arg_name: &proc_macro2::Ident,
    shared_components: &mut Vec<Ident>,
    mutable_components: &mut Vec<Ident>,
) -> Option<TokenStream2> {
    if let Some(TokenTree::Group(tuple_group)) = arg_iter.next() {
        let mut query_types = Vec::new();
        let mut tuple_iter = tuple_group.stream().into_iter().peekable();
        while let Some(tt) = tuple_iter.next() {
            let TokenTree::Punct(p) = &tt else { continue };
            if p.as_char() != '&' {
                continue;
            }
            let Some(TokenTree::Ident(mut_or_ty)) = tuple_iter.peek() else {
                continue;
            };
            if mut_or_ty == "mut" {
                tuple_iter.next();
                if let Some(TokenTree::Ident(ty)) = tuple_iter.next() {
                    query_types.push((true, quote! { #ty }));

                    if mutable_components.iter().any(|c| c == &ty) {
                        panic!(
                            "Component {} is already requested as mutable in another query, this would require two mutable borrows of the same data, which is undefined in Rust",
                            ty
                        );
                    }

                    if shared_components.iter().any(|c| c == &ty) {
                        panic!(
                            "Component {} is already requested as shared in another query, this would require a mutable and an immutable borrow of the same data, which is undefined in Rust",
                            ty
                        );
                    }

                    mutable_components.push(ty.clone());
                }
            } else {
                if let Some(TokenTree::Ident(ty)) = tuple_iter.next() {
                    query_types.push((false, quote! { #ty }));

                    if shared_components.iter().any(|c| c == &ty) {
                        // already requested as shared, that's fine
                    } else if mutable_components.iter().any(|c| c == &ty) {
                        panic!(
                            "Component {} is already requested as mutable in another query, this would require a mutable and an immutable borrow of the same data, which is undefined in Rust",
                            ty
                        );
                    } else {
                        shared_components.push(ty.clone());
                    }
                }
            }
        }
        let mut gather_code = TokenStream2::new();

        let mut names = Vec::new();
        for (i, (is_mut, ty)) in query_types.iter().enumerate() {
            let var_name = quote::format_ident!("c{}", i);
            names.push(var_name.clone());
            if *is_mut {
                gather_code.extend(quote! {
                    let #var_name = World::get_components_mut::<#ty>(world);
                });
            } else {
                gather_code.extend(quote! {
                    let #var_name = World::get_components::<#ty>(world);
                });
            }
        }
        let n = names.len();
        let mut iter_names = Vec::new();
        let mut curr_names = Vec::new();
        let mut id_exprs = Vec::new();
        let mut val_exprs = Vec::new();
        let mut assignments = Vec::new();
        let mut else_assignments = Vec::new();
        let mut curr_list = Vec::new();
        for i in 0..n {
            let iter_name = quote::format_ident!("iter{}", i);
            let curr_name = quote::format_ident!("curr{}", i);
            iter_names.push(iter_name.clone());
            curr_names.push(curr_name.clone());
            curr_list.push(curr_name.clone());
            let c_name = quote::format_ident!("c{}", i);

            if i == 0 {
                gather_code.extend(quote! {
                    let mut result = Vec::with_capacity(#c_name.len());
                });
            }
            gather_code.extend(quote! {
                let mut #iter_name = #c_name.into_iter();
                let mut #curr_name = #iter_name.next();
            });
            id_exprs.push(quote! { #curr_name.as_ref().unwrap().0 });
            val_exprs.push(quote! { #curr_name.take().unwrap().1 });
            assignments.push(quote! { #curr_name = #iter_name.next(); });
            else_assignments
                .push(quote! { if ids[#i] < max_id { #curr_name = #iter_name.next(); } });
        }

        let somes = curr_list.iter().map(|c| quote! { #c.is_some() });

        gather_code.extend(quote! {
            while #(#somes) && * {
                let ids = [#(#id_exprs),*];
                let min_id = *ids.iter().min().unwrap();
                let max_id = *ids.iter().max().unwrap();
                if min_id == max_id {
                    result.push((#(#val_exprs),*));
                    #(#assignments)* ;
                } else {
                    #(#else_assignments)* ;
                }
            }
            result.shrink_to_fit();
            result
        });

        Some(quote! {
            let #arg_name = {
                #gather_code
            }.into_iter();
        })
    } else {
        None
    }
}

fn handle_resource(
    arg_iter: &mut std::iter::Peekable<std::vec::IntoIter<TokenTree>>,
    arg_name: &proc_macro2::Ident,
    shared_resources: &mut Vec<Ident>,
    mutable_resources: &mut Vec<Ident>,
) -> Option<TokenStream2> {
    if let Some(ref_token) = arg_iter.next() {
        match &ref_token {
            TokenTree::Punct(p) if p.as_char() == '&' => {
                if let Some(TokenTree::Ident(mut_ident)) = arg_iter.peek() {
                    if mut_ident == "mut" {
                        arg_iter.next();
                        if let Some(TokenTree::Ident(res_ty)) = arg_iter.next() {
                            if mutable_resources.iter().any(|r| r == &res_ty) {
                                panic!(
                                    "Resource {} is already requested as mutable in another argument, this would require two mutable borrows of the same data, which is undefined in Rust",
                                    res_ty
                                );
                            }
                            if shared_resources.iter().any(|r| r == &res_ty) {
                                panic!(
                                    "Resource {} is already requested as shared in another argument, this would require a mutable and an immutable borrow of the same data, which is undefined in Rust",
                                    res_ty
                                );
                            }
                            mutable_resources.push(res_ty.clone());
                            return Some(quote! {
                                let #arg_name = World::get_resource_mut::<#res_ty>(world);
                            });
                        }
                    } else {
                        if let Some(TokenTree::Ident(res_ty)) = arg_iter.next() {
                            if shared_resources.iter().any(|r| r == &res_ty) {
                                // already requested as shared, that's fine
                            } else if mutable_resources.iter().any(|r| r == &res_ty) {
                                panic!(
                                    "Resource {} is already requested as mutable in another argument, this would require a mutable and an immutable borrow of the same data, which is undefined in Rust",
                                    res_ty
                                );
                            } else {
                                shared_resources.push(res_ty.clone());
                            }
                            return Some(quote! {
                                let #arg_name = World::get_resource::<#res_ty>(world);
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }
    None
}
