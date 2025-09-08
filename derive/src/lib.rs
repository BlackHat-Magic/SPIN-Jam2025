use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

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

        inventory::submit! {
            ComponentRegistration {
                type_id: typeid::ConstTypeId::of::<#name>(),
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

        inventory::submit! {
            ResourceRegistration {
                type_id: typeid::ConstTypeId::of::<#name>(),
                name: #type_name,
            }
        }
    }
    .into()
}
