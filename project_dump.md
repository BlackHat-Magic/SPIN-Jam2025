`build.rs`:

```rust
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=resources/");
    println!("cargo:rerun-if-changed=build.rs");

    let base_target_dir = std::env::var("CARGO_TARGET_DIR").unwrap_or(".".to_string());
    let target_dir = std::env::var("CARGO_BUILD_TARGET_DIR").unwrap_or("target".to_string());
    let target_dir = format!("{}/{}", base_target_dir, target_dir);
    let profile = std::env::var("PROFILE").unwrap_or("debug".to_string());
    let target_dir = format!("{}/{}", target_dir, profile);
    let target_dir = format!("{}/resources", target_dir);
    let target_dir = Path::new(target_dir.as_str());

    if target_dir.exists() {
        std::fs::remove_dir_all(target_dir).unwrap();
    }

    std::fs::create_dir(target_dir).unwrap();

    let resource_dir = Path::new("resources");
    compile_and_copy_files(resource_dir, target_dir);
}

fn compile_and_copy_files(from: &Path, to: &Path) {
    let read_dir = std::fs::read_dir(from).unwrap();
    for entry in read_dir {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            let dir_name = path.file_name().unwrap();
            let new_dir = to.join(dir_name);
            std::fs::create_dir(&new_dir).unwrap();
            compile_and_copy_files(&path, &new_dir);
        } else {
            let file_name = path.file_name().unwrap();

            //#[cfg(debug_assertions)]
            //{
            let new_file = to.join(file_name);
            std::fs::copy(&path, &new_file).unwrap();
            //}

            /*#[cfg(not(debug_assertions))]
            {
                let file_extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

                if file_extension != "wgsl" {
                    let new_file = to.join(file_name);
                    std::fs::copy(&path, &new_file).unwrap();
                    continue;
                }

                let wgsl_code = std::fs::read_to_string(&path).unwrap();
                let module = naga::front::wgsl::parse_str(&wgsl_code).unwrap();

                let mut validator = naga::valid::Validator::new(
                    naga::valid::ValidationFlags::all(),
                    naga::valid::Capabilities::all(),
                );
                let module_info = validator.validate(&module).unwrap();

                let options = naga::back::spv::Options::default();

                let mut spv_writer = naga::back::spv::Writer::new(&options).unwrap();
                let mut spv_words = Vec::<u32>::new();

                spv_writer
                    .write(&module, &module_info, None, &None, &mut spv_words)
                    .unwrap();

                let spv_bytes = spv_words
                    .iter()
                    .flat_map(|word| word.to_le_bytes().to_vec())
                    .collect::<Vec<u8>>();

                let new_file = to.join(file_name);
                let new_file = new_file.with_extension("spv");

                std::fs::write(&new_file, &spv_bytes).unwrap();

                std::process::Command::new("spirv-opt")
                    .arg("-O")
                    .arg(&new_file)
                    .arg("-o")
                    .arg(&new_file)
                    .output();
            }*/
        }
    }
}

```

`ecs/derive/src/lib.rs`:

```rust
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
 * ```
 * system!(
 *  fn my_system(
 *      query: query (&mut Transform, &Velocity),
 *      time: res &Time,
 *  ) {
 *      // query is impl Iterator<(&mut Transform, &Velocity)>
 *      // time is Option<&Time>
 *      for (transform, velocity) in query {
 *          transform.position += velocity.0 * time.delta_seconds;
 *      }
 *  }
 * )
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

    let all_send_sync = shared_components
        .iter()
        .chain(mutable_components.iter())
        .chain(shared_resources.iter())
        .chain(mutable_resources.iter())
        .collect::<Vec<_>>();
    let last_run_ident = quote::format_ident!("LAST_RUN_{}", fn_name.to_string().to_uppercase());

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
                #runs_alone #(|| #all_send_sync::is_not_send_sync())*
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
                    let #var_name = unsafe { World::get_components_mut::<#ty>(world) };
                });
            } else {
                gather_code.extend(quote! {
                    let #var_name = unsafe { World::get_components::<#ty>(world) };
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
            let mut #arg_name = {
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
                                let mut #arg_name = unsafe { World::get_resource_mut::<#res_ty>(world) };
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
                                let mut #arg_name = unsafe { World::get_resource::<#res_ty>(world) };
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

```

`ecs/derive/tests/component_pass.rs`:

```rust
use ecs::*;

#[derive(Component, Default, Debug, PartialEq)]
struct Foo(u32);

#[test]
fn component_macro_registers_type_and_allows_access() {
    let mut app = App::new();
    let entity = app.spawn_entity();
    app.add_component(entity, Foo(42)).unwrap();

    let commands: &Commands = &app;
    let world_ptr = commands.world;
    let components = unsafe { World::get_components::<Foo>(world_ptr) };

    assert_eq!(components.len(), 1);
    assert_eq!(components[0].0, entity);
    assert_eq!(*components[0].1, Foo(42));
}

```

`ecs/derive/tests/trybuild.rs`:

```rust
#[test]
fn derive_macros_compile() {
    let t = trybuild::TestCases::new();
    t.pass("tests/component_pass.rs");
    t.pass("tests/resource_pass.rs");
    t.pass("tests/system_pass.rs");
}

```

`ecs/src/lib.rs`:

```rust
#![allow(incomplete_features)]
#![feature(specialization)]

pub mod scheduler;
pub mod system;
pub mod world;

use std::any::Any;
use std::collections::HashMap;
use std::sync::OnceLock;

pub use inventory::submit;
pub use typeid::ConstTypeId;

pub use derive::*;

pub use scheduler::*;
pub use system::*;
pub use world::*;

pub use lazy_static::lazy_static;

pub trait SendSyncCheck {
    fn is_not_send_sync() -> bool;
}

impl<T: Send + Sync + Any> SendSyncCheck for T {
    fn is_not_send_sync() -> bool {
        false
    }
}

impl<T: Any> SendSyncCheck for T {
    default fn is_not_send_sync() -> bool {
        true
    }
}

pub trait Plugin {
    fn build(&self, app: &mut App);
}

#[derive(Default)]
pub struct PluginGroup {
    plugins: Vec<Box<dyn Plugin>>,
}

impl Plugin for PluginGroup {
    fn build(&self, app: &mut App) {
        for plugin in &self.plugins {
            plugin.build(app);
        }
    }
}

impl PluginGroup {
    pub fn add(&mut self, plugin: Box<dyn Plugin>) {
        self.plugins.push(plugin);
    }
}

#[macro_export]
macro_rules! plugin_group {
    ($($plugin:expr),* $(,)?) => {
        {
            let mut group = PluginGroup::default();
            $(
                group.add(Box::new($plugin));
            )*
            group
        }
    };
}

pub trait Component: Any {
    fn get_type_id(&self) -> usize;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

pub trait Resource: Any {
    fn get_type_id(&self) -> usize;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

pub struct ComponentRegistration {
    pub type_id: ConstTypeId,
    pub name: &'static str,
}

pub struct ResourceRegistration {
    pub type_id: ConstTypeId,
    pub name: &'static str,
}

inventory::collect!(ComponentRegistration);
inventory::collect!(ResourceRegistration);

static COMPONENT_IDS: OnceLock<HashMap<ComponentId, usize>> = OnceLock::new();
static RESOURCE_IDS: OnceLock<HashMap<ResourceId, usize>> = OnceLock::new();

pub type ComponentId = ConstTypeId;
pub type ResourceId = ConstTypeId;

fn build_component_ids() -> HashMap<ComponentId, usize> {
    let mut entries: Vec<_> = inventory::iter::<ComponentRegistration>
        .into_iter()
        .collect();
    entries.sort_by_key(|e| e.name);
    entries
        .into_iter()
        .enumerate()
        .map(|(i, r)| (r.type_id, i))
        .collect()
}

fn build_resource_ids() -> HashMap<ResourceId, usize> {
    let mut entries: Vec<_> = inventory::iter::<ResourceRegistration>
        .into_iter()
        .collect();
    entries.sort_by_key(|e| e.name);
    entries
        .into_iter()
        .enumerate()
        .map(|(i, r)| (r.type_id, i))
        .collect()
}

pub fn get_component_id<T>() -> usize {
    *COMPONENT_IDS
        .get_or_init(build_component_ids)
        .get(&ConstTypeId::of::<T>())
        .expect("Component not registered")
}

pub fn get_resource_id<T>() -> usize {
    *RESOURCE_IDS
        .get_or_init(build_resource_ids)
        .get(&ConstTypeId::of::<T>())
        .expect("Resource not registered")
}

#[derive(Component)]
pub struct EntityId {
    id: u32,
}

impl EntityId {
    pub fn get(&self) -> u32 {
        self.id
    }
}

pub struct Entity {
    pub id: u32,
    pub(crate) components: Vec<Option<Box<dyn Component>>>,
}

impl Entity {
    pub fn new(id: u32) -> Self {
        let mut components =
            Vec::with_capacity(COMPONENT_IDS.get_or_init(build_component_ids).len());
        components.resize_with(COMPONENT_IDS.get_or_init(build_component_ids).len(), || {
            None
        });
        let mut result = Self { id, components };
        result.add_component(Box::new(EntityId { id })).unwrap();
        result
    }

    pub fn set_component(&mut self, component: Option<Box<dyn Component>>, id: usize) {
        self.components[id] = component;
    }

    pub fn add_component(&mut self, component: Box<dyn Component>) -> Option<()> {
        let id = component.get_type_id();

        if self.components[id].is_none() {
            self.components[id] = Some(component);
            Some(())
        } else {
            None
        }
    }

    pub fn get_component<T: Component>(&self) -> Option<&T> {
        let id = get_component_id::<T>();
        self.components[id]
            .as_ref()
            .and_then(|c| c.as_any().downcast_ref::<T>())
    }

    pub fn get_component_mut<T: Component>(&mut self) -> Option<&mut T> {
        let id = get_component_id::<T>();
        self.components[id]
            .as_mut()
            .and_then(|c| c.as_any_mut().downcast_mut::<T>())
    }

    pub fn remove_component<T: Component>(&mut self) -> Option<Box<dyn Component>> {
        let id = get_component_id::<T>();
        self.components[id].take()
    }

    pub fn has_component<T: Component>(&self) -> bool {
        let id = get_component_id::<T>();
        self.components[id].is_some()
    }
}

```

`ecs/src/scheduler.rs`:

```rust
use std::ops::Deref;

use crate::*;
use rayon::prelude::*;

pub type Tick = u64;

pub struct Scheduler {
    world: *mut World,

    systems: HashMap<SystemStage, Vec<Vec<*mut dyn System>>>,
}

#[derive(Clone, Copy)]
struct WorldWrapper(*mut World);

unsafe impl Send for WorldWrapper {}
unsafe impl Sync for WorldWrapper {}

impl Deref for WorldWrapper {
    type Target = World;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0 }
    }
}

#[derive(Clone, Copy)]
struct SystemWrapper(*mut dyn System);

unsafe impl Send for SystemWrapper {}
unsafe impl Sync for SystemWrapper {}

impl Scheduler {
    pub(crate) fn new(world: *mut World) -> Self {
        Self {
            world,
            systems: HashMap::new(),
        }
    }

    pub(crate) fn run(scheduler: *mut Scheduler, stage: SystemStage) {
        unsafe {
            let world = (*scheduler).world;
            let Some(systems) = (*scheduler).systems.get(&stage) else {
                return;
            };
            for group in systems {
                if group.len() == 1 {
                    // Run single systems on main thread because they might not be Send + Sync
                    let system = group[0];
                    system.as_mut().unwrap().run_unsafe(world);
                    continue;
                }

                let group: Vec<SystemWrapper> = group.iter().map(|&s| SystemWrapper(s)).collect();
                let world = WorldWrapper(world);

                group.par_iter().for_each(|system| {
                    #[allow(clippy::redundant_locals)] // it's not actually redundant here because
                    // of safety reasons
                    let world = world;
                    let world = world.0;

                    let system = system.0;

                    system.as_mut().unwrap().run_unsafe(world);
                });
            }
        }
    }

    pub(crate) fn add_system(&mut self, system: *mut dyn System, stage: SystemStage) {
        let entry = self.systems.entry(stage).or_default();
        if unsafe { system.as_ref() }.unwrap().runs_alone() || entry.is_empty() {
            entry.push(vec![system]);
            return;
        }

        for group in entry.iter_mut() {
            if unsafe { group[0].as_ref() }.unwrap().runs_alone() {
                continue;
            }
            let mut overlap = false;
            for &existing_system in group.iter() {
                let existing_component_access = unsafe { (*existing_system).component_access() };
                let new_component_access = unsafe { (*system).component_access() };
                if existing_component_access.overlaps(new_component_access) {
                    overlap = true;
                    break;
                }

                let existing_resource_access = unsafe { (*existing_system).resource_access() };
                let new_resource_access = unsafe { (*system).resource_access() };
                if existing_resource_access.overlaps(new_resource_access) {
                    overlap = true;
                    break;
                }
            }
            if !overlap {
                group.push(system);
                return;
            }
        }

        entry.push(vec![system]);
    }
}

```

`ecs/src/system.rs`:

```rust
use crate::*;

pub enum SystemRunCriteria {
    Always,
    Once,
    Never,
    OnChannelReceive(String),
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum SystemStage {
    Init,
    PreUpdate,
    Update,
    PostUpdate,
    Render,
    DeInit,
}

pub struct ComponentAccess {
    pub read: &'static [usize],
    pub write: &'static [usize],
}

impl ComponentAccess {
    pub fn overlaps(&self, other: &ComponentAccess) -> bool {
        for &r in self.read {
            if other.write.contains(&r) {
                return true;
            }
        }

        for &w in self.write {
            if other.read.contains(&w) || other.write.contains(&w) {
                return true;
            }
        }

        false
    }
}

pub struct ResourceAccess {
    pub read: &'static [usize],
    pub write: &'static [usize],
}

impl ResourceAccess {
    pub fn overlaps(&self, other: &ResourceAccess) -> bool {
        for &r in self.read {
            if other.write.contains(&r) {
                return true;
            }
        }

        for &w in self.write {
            if other.read.contains(&w) || other.write.contains(&w) {
                return true;
            }
        }

        false
    }
}

pub trait System: Send + Sync + 'static {
    fn name(&self) -> &'static str;
    fn component_access(&self) -> &'static ComponentAccess;
    fn resource_access(&self) -> &'static ResourceAccess;
    fn get_last_run(&self) -> Tick;
    fn set_last_run(&mut self, tick: Tick);
    fn runs_alone(&self) -> bool;

    /// # Safety
    /// just don't call this outside of the `ecs` crate
    unsafe fn run_unsafe(&mut self, world: *mut World);
}

```

`ecs/src/world.rs`:

```rust
use std::ops::{Deref, DerefMut};

use crate::*;

pub struct App {
    commands: Commands,
}

impl Deref for App {
    type Target = Commands;

    fn deref(&self) -> &Self::Target {
        &self.commands
    }
}

impl DerefMut for App {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.commands
    }
}

impl App {
    // I prefer App:new over App::default for clarity here as it is only supposed to ever
    // initialize to one thing, so it doesn't make snese to me to call it "default"
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let world = Box::into_raw(Box::new(World::new()));
        let scheduler = Box::into_raw(Box::new(Scheduler::new(world)));
        unsafe {
            (*world).scheduler = scheduler;
        }

        Self {
            commands: Commands::new(world),
        }
    }

    pub fn add_plugin(&mut self, plugin: impl Plugin) {
        plugin.build(self);
    }

    pub fn init(&mut self) {
        unsafe {
            let world = self.commands.world;
            let scheduler = (*world).scheduler;

            Scheduler::run(scheduler, SystemStage::Init);
        }
    }

    pub fn de_init(&mut self) {
        unsafe {
            let world = self.commands.world;
            let scheduler = (*world).scheduler;

            Scheduler::run(scheduler, SystemStage::DeInit);
        }
    }

    pub fn run(&mut self) {
        unsafe {
            let world = self.commands.world;
            let scheduler = (*world).scheduler;

            (*world).tick += 1;

            Scheduler::run(scheduler, SystemStage::PreUpdate);
            Scheduler::run(scheduler, SystemStage::Update);
            Scheduler::run(scheduler, SystemStage::PostUpdate);
            Scheduler::run(scheduler, SystemStage::Render);
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        unsafe {
            let _ = Box::from_raw(self.commands.world);
        }
    }
}

pub struct Commands {
    pub world: *mut World,
}

pub type EntityId = u32;

impl Commands {
    pub fn new(world: *mut World) -> Self {
        Self { world }
    }

    pub fn spawn_entity(&mut self) -> EntityId {
        unsafe {
            let world = self.world.as_mut().unwrap();
            let id = world.next_entity_id;
            world.next_entity_id += 1;
            let entity = Entity::new(id);
            world.entities.push(entity);
            id
        }
    }

    pub fn despawn_entity(&mut self, id: EntityId) -> Option<()> {
        unsafe {
            let world = self.world.as_mut().unwrap();
            if (id as usize) < world.entities.len() {
                world.entities.remove(id as usize);
                Some(())
            } else {
                None
            }
        }
    }

    pub fn insert_resource<T: Resource>(&mut self, resource: T) -> Option<()> {
        let id = get_resource_id::<T>();
        unsafe {
            let world = self.world.as_mut().unwrap();
            if world.resources[id].is_none() {
                world.resources[id] = Some(Box::new(resource));
                Some(())
            } else {
                world.resources[id] = Some(Box::new(resource));
                None
            }
        }
    }

    pub fn get_resource<T: Resource>(&mut self) -> Option<&'static T> {
        unsafe { World::get_resource::<T>(self.world) }
    }

    pub fn get_resource_mut<T: Resource>(&mut self) -> Option<&'static mut T> {
        unsafe { World::get_resource_mut::<T>(self.world) }
    }

    pub fn add_system(&mut self, system: impl System, stage: SystemStage) {
        unsafe {
            let world = self.world.as_mut().unwrap();
            let system = Box::into_raw(Box::new(system));
            world.systems.push((stage, system));

            world.scheduler.as_mut().unwrap().add_system(system, stage);
        }
    }

    pub fn add_component<T: Component>(&mut self, entity_id: EntityId, component: T) -> Option<()> {
        unsafe {
            let world = self.world.as_mut().unwrap();
            if let Some(entity) = world.entities.get_mut(entity_id as usize) {
                entity.add_component(Box::new(component))
            } else {
                None
            }
        }
    }

    pub fn remove_component<T: Component>(
        &mut self,
        entity_id: EntityId,
    ) -> Option<Box<dyn Component>> {
        unsafe {
            let world = self.world.as_mut().unwrap();
            if let Some(entity) = world.entities.get_mut(entity_id as usize) {
                entity.remove_component::<T>()
            } else {
                None
            }
        }
    }

    pub fn run_system(&mut self, system: &mut dyn System) {
        unsafe {
            system.run_unsafe(self.world);
        }
    }

    pub fn should_exit(&self) -> bool {
        unsafe {
            let world = self.world.as_ref().unwrap();
            world.should_exit
        }
    }

    pub fn exit(&mut self) {
        unsafe {
            let world = self.world.as_mut().unwrap();
            world.should_exit = true;
        }
    }
}

pub struct World {
    pub(crate) entities: Vec<Entity>,
    pub(crate) resources: Vec<Option<Box<dyn Resource>>>,
    pub(crate) systems: Vec<(SystemStage, *mut dyn System)>,
    pub(crate) tick: Tick,
    pub(crate) next_entity_id: u32,
    pub(crate) scheduler: *mut Scheduler,
    pub(crate) should_exit: bool,
}

impl Drop for World {
    fn drop(&mut self) {
        unsafe {
            if !self.scheduler.is_null() {
                let _ = Box::from_raw(self.scheduler);
            }
        }
    }
}

impl World {
    fn new() -> Self {
        let mut resources =
            Vec::with_capacity(RESOURCE_IDS.get_or_init(crate::build_resource_ids).len());
        resources.resize_with(RESOURCE_IDS.get().unwrap().len(), || None);
        Self {
            entities: Vec::new(),
            resources,
            systems: Vec::new(),
            tick: 0,
            next_entity_id: 0,
            scheduler: std::ptr::null_mut(),
            should_exit: false,
        }
    }

    /// # Safety
    ///
    /// `world` must be non-null and valid
    pub unsafe fn get_resource<T: Resource>(world: *mut World) -> Option<&'static T> {
        let id = get_resource_id::<T>();
        unsafe {
            Some(
                world
                    .as_ref()?
                    .resources
                    .get(id)?
                    .as_ref()?
                    .as_any()
                    .downcast_ref::<T>()
                    .unwrap(),
            )
        }
    }

    /// # Safety
    ///
    /// `world` must be non-null and valid
    pub unsafe fn get_resource_mut<T: Resource>(world: *mut World) -> Option<&'static mut T> {
        let id = get_resource_id::<T>();
        unsafe {
            Some(
                world
                    .as_mut()?
                    .resources
                    .get_mut(id)?
                    .as_mut()?
                    .as_any_mut()
                    .downcast_mut::<T>()
                    .unwrap(),
            )
        }
    }

    /// # Safety
    ///
    /// `world` must be non-null and valid
    pub unsafe fn get_components<T: Component>(world: *mut World) -> Vec<(u32, &'static T)> {
        let id = get_component_id::<T>();
        let mut components = Vec::new();

        unsafe {
            let world = world.as_ref().unwrap();
            for entity in &world.entities {
                if let Some(component) = entity
                    .components
                    .get(id)
                    .and_then(|c| c.as_ref())
                    .and_then(|c| c.as_any().downcast_ref::<T>())
                {
                    components.push((entity.id, component));
                }
            }
        }

        components
    }

    /// # Safety
    ///
    /// `world` must be non-null and valid
    pub unsafe fn get_components_mut<T: Component>(
        world: *mut World,
    ) -> Vec<(u32, &'static mut T)> {
        let id = get_component_id::<T>();
        let mut components = Vec::new();

        unsafe {
            let world = world.as_mut().unwrap();
            for entity in &mut world.entities {
                if let Some(component) = entity
                    .components
                    .get_mut(id)
                    .and_then(|c| c.as_mut())
                    .and_then(|c| c.as_any_mut().downcast_mut::<T>())
                {
                    components.push((entity.id, component));
                }
            }
        }

        components
    }
}

```

`ecs/tests/access.rs`:

```rust
use ecs::{ComponentAccess, ResourceAccess};

const EMPTY: &[usize] = &[];

#[test]
fn component_access_detects_conflicting_writes() {
    static READ: [usize; 1] = [1];
    static WRITE: [usize; 1] = [2];
    static OTHER_WRITE: [usize; 1] = [2];

    let a = ComponentAccess {
        read: &READ,
        write: EMPTY,
    };
    let b = ComponentAccess {
        read: EMPTY,
        write: &OTHER_WRITE,
    };
    let c = ComponentAccess {
        read: EMPTY,
        write: &WRITE,
    };

    assert!(b.overlaps(&c));
    assert!(c.overlaps(&b));
    assert!(!a.overlaps(&b));
}

#[test]
fn component_access_detects_read_write_conflicts() {
    static READ: [usize; 1] = [3];
    static WRITE: [usize; 1] = [3];

    let reader = ComponentAccess {
        read: &READ,
        write: EMPTY,
    };
    let writer = ComponentAccess {
        read: EMPTY,
        write: &WRITE,
    };

    assert!(reader.overlaps(&writer));
    assert!(writer.overlaps(&reader));
}

#[test]
fn resource_access_mirrors_component_access_rules() {
    static READ: [usize; 1] = [7];
    static WRITE: [usize; 1] = [7];

    let read_only = ResourceAccess {
        read: &READ,
        write: EMPTY,
    };
    let write_only = ResourceAccess {
        read: EMPTY,
        write: &WRITE,
    };
    let disjoint_write = ResourceAccess {
        read: EMPTY,
        write: &[9],
    };

    assert!(read_only.overlaps(&write_only));
    assert!(write_only.overlaps(&read_only));
    assert!(!write_only.overlaps(&disjoint_write));
}

```

`ecs/tests/core.rs`:

```rust
use ecs::*;

#[derive(Component, Default, Debug, PartialEq)]
struct Position(f32);

#[derive(Component, Debug, PartialEq)]
struct Velocity(f32);

#[derive(Resource, Default, Debug, PartialEq)]
struct Counter(u32);

#[test]
fn spawn_entity_assigns_ids_and_entity_component() {
    let mut app = App::new();

    let e0 = app.spawn_entity();
    let e1 = app.spawn_entity();

    assert_eq!(e0, 0);
    assert_eq!(e1, 1);

    let commands: &Commands = &app;
    let world = commands.world;

    let entity_ids = unsafe { World::get_components::<EntityId>(world) };
    let collected: Vec<u32> = entity_ids.into_iter().map(|(_, id)| id.get()).collect();
    assert_eq!(collected, vec![0, 1]);
}

#[test]
fn add_get_and_remove_components_round_trip() {
    let mut app = App::new();
    let entity = app.spawn_entity();

    assert!(app.add_component(entity, Position(3.0)).is_some());
    assert!(app.add_component(entity, Velocity(2.71)).is_some());

    // second insertion should fail without replacing
    assert!(app.add_component(entity, Position(1.0)).is_none());

    let commands: &Commands = &app;
    let world = commands.world;

    let positions = unsafe { World::get_components::<Position>(world) };
    assert_eq!(positions.len(), 1);
    assert_eq!(positions[0].0, entity);
    assert_eq!(*positions[0].1, Position(3.0));

    let velocities = unsafe { World::get_components::<Velocity>(world) };
    assert_eq!(velocities.len(), 1);
    assert_eq!(velocities[0].0, entity);
    assert_eq!(*velocities[0].1, Velocity(2.71));

    let removed = app
        .remove_component::<Position>(entity)
        .expect("component missing");
    assert!(removed.as_any().downcast_ref::<Position>().is_some());

    let positions = unsafe { World::get_components::<Position>(world) };
    assert!(positions.is_empty());
}

#[test]
fn insert_resource_overwrites_and_returns_flags() {
    let mut app = App::new();

    assert!(app.insert_resource(Counter(1)).is_some());
    assert!(app.insert_resource(Counter(5)).is_none());

    let commands: &Commands = &app;
    let world = commands.world;

    let counter = unsafe { World::get_resource::<Counter>(world).expect("resource missing") };
    assert_eq!(*counter, Counter(5));
}

system! {
    fn touch_components(query: query (&mut Position, &Velocity)) {
        for (pos, vel) in query {
            pos.0 += vel.0;
        }
    }
}

#[test]
fn systems_modify_components_via_scheduler() {
    let mut app = App::new();
    let entity = app.spawn_entity();
    app.add_component(entity, Position(1.0)).unwrap();
    app.add_component(entity, Velocity(4.0)).unwrap();

    app.add_system(touch_components, SystemStage::Update);
    app.run();

    let commands: &Commands = &app;
    let world = commands.world;

    let positions = unsafe { World::get_components::<Position>(world) };
    assert_eq!(positions[0].1.0, 5.0);
}

#[test]
fn despawn_entity_removes_components() {
    let mut app = App::new();
    let e0 = app.spawn_entity();
    let e1 = app.spawn_entity();

    app.add_component(e0, Position(0.0)).unwrap();
    app.add_component(e1, Position(1.0)).unwrap();

    assert!(app.despawn_entity(e0).is_some());

    let commands: &Commands = &app;
    let world = commands.world;

    let positions = unsafe { World::get_components::<Position>(world) };
    assert_eq!(positions.len(), 1);
    assert_eq!(positions[0].0, e1);
}

```

`ecs/tests/scheduler.rs`:

```rust
use ecs::*;

#[derive(Component, Default)]
struct Position(u32);

#[derive(Resource, Default)]
struct Counter(u32);

system! {
    fn writer(query: query(&mut Position), counter: res &mut Counter) {
        let Some(counter) = counter else { return; };
        let mut wrote = false;
        for pos in query {
            pos.0 += 1;
            wrote = true;
        }
        if wrote {
            counter.0 += 1;
        }
    }
}

system! {
    fn reader(query: query(&Position), counter: res &mut Counter) {
        let Some(counter) = counter else { return; };
        for _pos in query {
            counter.0 += 10;
        }
    }
}

#[test]
fn scheduler_runs_systems_once_per_tick_in_stage_order() {
    let mut app = App::new();
    let entity = app.spawn_entity();
    app.add_component(entity, Position(0)).unwrap();
    app.insert_resource(Counter::default());

    app.add_system(writer, SystemStage::Update);
    app.add_system(reader, SystemStage::PostUpdate);

    app.run();

    let commands: &Commands = &app;
    let world = commands.world;

    let counter = unsafe { ecs::World::get_resource::<Counter>(world).unwrap() };
    assert_eq!(counter.0, 11); // writer + reader contributions

    let mut positions = unsafe { ecs::World::get_components::<Position>(world) };
    assert_eq!(positions.len(), 1);
    assert_eq!(positions.pop().unwrap().1.0, 1);
}

```

`networking/net_derive/src/lib.rs`:

```rust
use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

#[proc_macro_derive(NetSend)]
pub fn derive_net_send(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let type_name = name.to_string();

    quote! {
        impl NetSend for #name {
            fn get_type_id(&self) -> usize {
                get_net_id::<Self>()
            }

            fn get_bytes(&self) -> Vec<u8> {
                bincode::serde::encode_to_vec::<&#name, bincode::config::Configuration>(self, bincode::config::Configuration::default()).unwrap()
            }

            fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
                Ok(bincode::serde::decode_from_slice::<#name, bincode::config::Configuration>(bytes, bincode::config::Configuration::default())?.0)
            }
        }

        submit! {
            NetRegistration {
                type_id: ConstTypeId::of::<#name>(),
                name: #type_name,
                from_bytes: |bytes: &[u8]| -> anyhow::Result<Box<dyn std::any::Any>> { Ok(Box::new(#name::from_bytes(bytes)?)) },
            }
        }
    }
    .into()
}

```

`networking/src/lib.rs`:

```rust
use ecs::*;

mod registry;

pub use registry::*;

use anyhow::Result;
use std::any::Any;
use std::collections::VecDeque;
use std::sync::Mutex;

pub use bincode;
pub use net_derive::*;
pub use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tokio::sync::mpsc::*;

pub trait NetSend: Any + Sized + DeserializeOwned {
    fn get_type_id(&self) -> usize;
    fn get_bytes(&self) -> Vec<u8>;
    fn from_bytes(bytes: &[u8]) -> Result<Self>;
}

pub struct NetworkingPlugin {
    is_server: bool,
}

impl NetworkingPlugin {
    pub fn client() -> Self {
        Self { is_server: false }
    }

    pub fn server() -> Self {
        Self { is_server: true }
    }
}

impl Plugin for NetworkingPlugin {
    fn build(&self, app: &mut App) {
        let (tx_event, rx_event) = channel(256);
        let (tx_request, rx_request) = channel(256);

        tokio::spawn(handle_networking(tx_event, rx_request));

        app.insert_resource(Networking::new(tx_request, rx_event));
        app.add_system(gather_events, SystemStage::PreUpdate);
    }
}

system! {
    fn gather_events(
        networking: res &mut Networking,
    ) {
        let Some(networking) = networking else {
            return;
        };

        networking.gather_recv();
        networking.serialize_recv();
    }
}

type RecvChannel = Mutex<VecDeque<(Target, Box<dyn Any>)>>;

#[derive(Resource)]
pub struct Networking {
    tx_request: Sender<NetworkingRequest>,
    rx_event: Receiver<NetworkingEvent>,

    recv_buffer: Vec<RecvChannel>,
    events: Vec<NetworkingEvent>,
}

impl Networking {
    fn new(tx_request: Sender<NetworkingRequest>, rx_event: Receiver<NetworkingEvent>) -> Self {
        let mut recv_buffer = Vec::new();
        let recv_count = registry::NET_IDS.len();
        for _ in 0..recv_count {
            recv_buffer.push(Mutex::new(VecDeque::new()));
        }

        Self {
            tx_request,
            rx_event,
            recv_buffer,
            events: Vec::new(),
        }
    }

    fn gather_recv(&mut self) {
        let mut events = Vec::new();

        while let Ok(event) = self.rx_event.try_recv() {
            events.push(event);
        }

        self.events = events;
    }

    fn split_off_events(&mut self, cond: fn(&NetworkingEvent) -> bool) -> Vec<NetworkingEvent> {
        let (split_off, events) = self.events.drain(..).partition(cond);

        self.events = events;
        split_off
    }

    fn serialize_recv(&mut self) {
        let split_events = self.split_off_events(|e| matches!(e, NetworkingEvent::RecvData { .. }));

        for event in split_events {
            let NetworkingEvent::RecvData { from, data } = &event else {
                panic!("Event type mismatch in serialize_recv");
            };

            debug_assert!(data.len() > 4);
            let type_id = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;

            debug_assert!(type_id < self.recv_buffer.len());
            let data = &data[4..];

            let Ok(obj) = registry::FROM_BYTES[type_id](data) else {
                println!(
                    "Failed to deserialize network object of type id: {}",
                    type_id,
                );
                continue;
            };
            let mut buffer = self.recv_buffer[type_id].lock().unwrap();
            buffer.push_back((*from, obj));
        }
    }

    pub fn next<T: NetSend>(&self) -> Option<(Target, T)> {
        let type_id = registry::get_net_id::<T>();
        debug_assert!(type_id < self.recv_buffer.len());

        let mut buffer = self.recv_buffer[type_id].lock().unwrap();
        let (target, obj) = buffer.pop_front()?;
        let obj = *obj.downcast::<T>().unwrap();

        Some((target, obj))
    }

    pub fn collect<T: NetSend>(&self) -> Vec<(Target, T)> {
        let type_id = registry::get_net_id::<T>();
        debug_assert!(type_id < self.recv_buffer.len());

        let mut buffer = self.recv_buffer[type_id].lock().unwrap();
        let mut results = Vec::new();

        while let Some((target, obj)) = buffer.pop_front() {
            let obj = *obj.downcast::<T>().unwrap();
            results.push((target, obj));
        }

        results
    }

    pub fn send<T: NetSend>(&self, reliability: Reliability, target: Target, data: T) {
        debug_assert!(target != Target::This, "Cannot send data to 'This' target");

        let mut bytes = Vec::new();
        let type_id = data.get_type_id() as u32;
        bytes.extend_from_slice(&type_id.to_le_bytes());
        bytes.extend_from_slice(&data.get_bytes());

        let request = NetworkingRequest::SendData {
            reliability,
            target,
            data: bytes,
        };

        self.tx_request
            .try_send(request)
            .expect("Networking request buffer full");
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Target {
    All,
    Single(u32),
    This,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum NetworkingEvent {
    RecvData { from: Target, data: Vec<u8> },
    Disconnected { target: Target },
    Connected { target: Target },
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Reliability {
    Reliable,
    Unreliable,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum NetworkingRequest {
    Exit,
    SendData {
        reliability: Reliability,
        target: Target,
        data: Vec<u8>,
    },
}

async fn handle_networking(
    mut tx_event: Sender<NetworkingEvent>,
    mut rx_request: Receiver<NetworkingRequest>,
) {
    loop {
        tokio::select! {
            request = rx_request.recv() => {
                let Some(request) = request else {
                    break;
                };

                match request {
                    NetworkingRequest::Exit => break,
                    NetworkingRequest::SendData { reliability, target, data } => {
                        // Handle sending data over the network
                    }
                }
            }
        }
    }
}

```

`networking/src/registry.rs`:

```rust
pub use typeid::ConstTypeId;

use anyhow::Result;

use crate::*;
use lazy_static::lazy_static;
use std::collections::HashMap;

pub struct NetRegistration {
    pub type_id: ConstTypeId,
    pub name: &'static str,
    pub from_bytes: fn(&[u8]) -> Result<Box<dyn Any>>,
}

pub fn from_bytes<T: NetSend>(bytes: &[u8]) -> Result<Box<T>> {
    let index = get_net_id::<T>();
    FROM_BYTES[index](bytes)?
        .downcast::<T>()
        .map_err(|_| anyhow::anyhow!("Failed to downcast Box<dyn Any> to Box<T>"))
}

pub fn get_net_id<T: NetSend>() -> usize {
    *NET_IDS.get(&ConstTypeId::of::<T>()).expect(
        "Type not registered as NetRecv. You must use the Derive macro to register the type.",
    )
}

inventory::collect!(NetRegistration);

type FromBytes = fn(&[u8]) -> Result<Box<dyn Any>>;

lazy_static! {
    pub static ref NET_IDS: HashMap<NetId, usize> = build_net_ids();
    pub static ref FROM_BYTES: Vec<FromBytes> = {
        let mut entries: Vec<_> = inventory::iter::<NetRegistration>.into_iter().collect();
        entries.sort_by_key(|e| NET_IDS[&e.type_id]);
        entries.into_iter().map(|r| r.from_bytes).collect()
    };
}

pub type NetId = ConstTypeId;

fn build_net_ids() -> HashMap<NetId, usize> {
    let mut entries: Vec<_> = inventory::iter::<NetRegistration>.into_iter().collect();
    entries.sort_by_key(|e| e.name); // ensures deterministic ordering
    entries
        .into_iter()
        .enumerate()
        .map(|(i, r)| (r.type_id, i))
        .collect()
}

```

`resources/materials/test_mat.json`:

```text
{
  "albedo": "rawr",
  "metallic": "#ff",
  "roughness": "#80",
  "ao": "#80"
}

```

`resources/shaders/fg_main.wgsl`:

```wgsl
struct FragmentInput {
    @location(0) worldPos : vec3<f32>,
    @location(1) normal   : vec3<f32>,
    @location(2) uv       : vec2<f32>,
};

struct Light {
    position : vec3<f32>,
    _pad1    : f32,
    color    : vec3<f32>,
    _pad2    : f32,
};

@group(0) @binding(1) var<storage, read> lights : array<Light>;
@group(0) @binding(2) var<uniform> cameraPos : vec3<f32>;

@group(0) @binding(3) var albedo_tex: texture_2d<f32>;
@group(0) @binding(4) var albedo_sampler: sampler;
@group(0) @binding(5) var metallic_tex: texture_2d<f32>;
@group(0) @binding(6) var metallic_sampler: sampler;
@group(0) @binding(7) var roughness_tex: texture_2d<f32>;
@group(0) @binding(8) var roughness_sampler: sampler;
@group(0) @binding(9) var ao_tex: texture_2d<f32>;
@group(0) @binding(10) var ao_sampler: sampler;

fn fresnelSchlick(cosTheta: f32, F0: vec3<f32>) -> vec3<f32> {
    return F0 + (1.0 - F0) * pow(1.0 - cosTheta, 5.0);
}

fn distributionGGX(N: vec3<f32>, H: vec3<f32>, roughness: f32) -> f32 {
    let a      = roughness * roughness;
    let a2     = a * a;
    let NdotH  = max(dot(N, H), 0.0);
    let NdotH2 = NdotH * NdotH;

    let denom = NdotH2 * (a2 - 1.0) + 1.0;
    return a2 / (3.14159265 * denom * denom);
}

fn geometrySchlickGGX(NdotV: f32, roughness: f32) -> f32 {
    let r = roughness + 1.0;
    let k = (r * r) / 8.0;
    return NdotV / (NdotV * (1.0 - k) + k);
}

fn geometrySmith(N: vec3<f32>, V: vec3<f32>, L: vec3<f32>, roughness: f32) -> f32 {
    let NdotV = max(dot(N, V), 0.0);
    let NdotL = max(dot(N, L), 0.0);
    let ggx1 = geometrySchlickGGX(NdotV, roughness);
    let ggx2 = geometrySchlickGGX(NdotL, roughness);
    return ggx1 * ggx2;
}

@fragment
fn main(input: FragmentInput) -> @location(0) vec4<f32> {
    let N = normalize(input.normal);
    let V = -normalize(cameraPos - input.worldPos);

    let albedo = textureSample(albedo_tex, albedo_sampler, input.uv).rgb;
    let metallic = textureSample(metallic_tex, metallic_sampler, input.uv).r;
    let roughness = textureSample(roughness_tex, roughness_sampler, input.uv).r;
    let ao = textureSample(ao_tex, ao_sampler, input.uv).r;

    let F0 = mix(vec3<f32>(0.04), albedo, metallic);

    var Lo = vec3<f32>(0.0);
    for (var i = 0u; i < arrayLength(&lights); i = i + 1u) {
        let light = lights[i];
        let L = normalize(light.position - input.worldPos);
        let H = normalize(V + L);

        let distance = length(light.position - input.worldPos);
        let attenuation = 1.0 / (distance * distance);
        let radiance = light.color * attenuation;

        let NDF = distributionGGX(N, H, roughness);
        let G   = geometrySmith(N, V, L, roughness);
        let F   = fresnelSchlick(max(dot(H, V), 0.0), F0);

        let numerator = NDF * G * F;
        let denominator = 4.0 * max(dot(N, V), 0.0) * max(dot(N, L), 0.0) + 0.001;
        let specular = numerator / denominator;

        let kS = F;
        var kD = vec3<f32>(1.0) - kS;
        kD *= 1.0 - metallic;

        let NdotL = max(dot(N, L), 0.0);
        Lo += (kD * albedo / 3.14159265 + specular) * radiance * NdotL;
    }

    let ambient = vec3<f32>(0.01) * albedo * ao;
    let color = ambient + Lo;

    let gamma = 2.2;
    let mapped = pow(color, vec3<f32>(1.0 / gamma));
    return vec4<f32>(mapped, 1.0);
}


```

`resources/shaders/quad_fs.wgsl`:

```wgsl
@group(0) @binding(0) var texture: texture_2d<f32>;
@group(0) @binding(1) var sampler_: sampler;

@fragment
fn main(@location(0) tex_coords: vec2<f32>) -> @location(0) vec4<f32> {
    return textureSample(texture, sampler_, tex_coords);
}

```

`resources/shaders/quad_vs.wgsl`:

```wgsl
@vertex
fn main(@location(0) position: vec3<f32>, @location(1) tex_coords: vec2<f32>) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(position, 1.0);
    output.tex_coords = tex_coords;
    return output;
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

```

`resources/shaders/vs_main.wgsl`:

```wgsl
struct VertexInput {
    @location(0) position : vec3<f32>,
    @location(1) normal   : vec3<f32>,
    @location(2) uv       : vec2<f32>,
};

struct Uniforms {
    model      : mat4x4<f32>,
    view       : mat4x4<f32>,
    projection : mat4x4<f32>,
};

struct VertexOutput {
    @builtin(position) position : vec4<f32>,
    @location(0) worldPos       : vec3<f32>,
    @location(1) normal         : vec3<f32>,
    @location(2) uv             : vec2<f32>,
};

@group(0) @binding(0) var<uniform> uniforms : Uniforms;

@vertex
fn main(input : VertexInput) -> VertexOutput {
    var output : VertexOutput;
    let worldPos = (uniforms.model * vec4<f32>(input.position, 1.0)).xyz;
    output.position = uniforms.projection * uniforms.view * vec4<f32>(worldPos, 1.0);
    output.worldPos = worldPos;
    output.normal = normalize((uniforms.model * vec4<f32>(input.normal, 0.0)).xyz);
    output.uv = input.uv;
    return output;
}
```

`resources/ui/menu.json`:

```text
{
    "type": "Container",
    "id": "menu",
    "toggle_id": "menu",
    "on_by_default": false,
    "children": [
        {
            "type": "Text",
            "rect": {
                "x": 100.0,
                "y": 300.0,
                "width": 2.0,
                "height": 2.0
            },
            "id": "title",
            "content": "SPIN-GameJam 2025",
            "size": 24.0,
            "font": "IndieFlower",
            "color": "#ff70ff",
            "align": "TopLeft"
        },
        {
            "type": "Image",
            "rect": {
                "x": 300.0,
                "y": 300.0,
                "width": 4.0,
                "height": 4.0
            },
            "id": "test image",
            "image": "rawr",
            "align": "Center"
        }
    ]
}
```

`resources/ui/root.json`:

```text
{
  "type": "Container",
  "id": "root",
  "children": [
    {
      "type": "SubFile",
      "file_path": "menu"
    }
  ]
}

```

`src/audio/mod.rs`:

```rust
use crate::*;

use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

use rodio::source::*;
use rodio::*;

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Audio::load());
    }
}

#[derive(Resource)]
pub struct Audio {
    stream_handle: OutputStream,
    sounds: HashMap<String, Buffered<Decoder<BufReader<File>>>>,
}

impl Audio {
    fn load() -> Self {
        let stream_handle = OutputStreamBuilder::open_default_stream().unwrap();
        let sounds = gather_dir("sounds", |path| {
            let file = File::open(path).ok()?;
            let buf_reader = BufReader::new(file);
            Some(Decoder::new(buf_reader).ok()?.buffered())
        })
        .unwrap();

        Self {
            stream_handle,
            sounds,
        }
    }

    pub fn play(&self, name: &str, volume: f32, looping: bool) {
        if let Some(sound) = self.sounds.get(name) {
            let sink = Sink::connect_new(self.stream_handle.mixer());
            let sound = (*sound).clone();
            //let sound = sound.amplify(volume);
            if looping {
                let sound = sound.repeat_infinite();
                sink.append(sound);
            } else {
                sink.append(sound);
            }

            sink.detach();
        } else {
            println!("Sound '{}' not found!", name);
        }
    }
}

```

`src/client.rs`:

```rust
use std::sync::Arc;

use glam::*;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

pub use ecs::*;
pub use networking::*;

pub mod audio;
pub mod physics;
pub mod render;
pub mod utils;

pub use audio::*;
pub use physics::*;
pub use render::model::ModelHandle;
use render::sprite::*;
pub use render::*;
use utils::input::Input;
pub use utils::time::*;
pub use utils::*;

#[derive(NetSend, Serialize, Deserialize)]
pub struct TestMessage {
    pub content: String,
}

#[tokio::main]
async fn main() {
    let mut app = App::new();

    struct WinitApp {
        app: App,
    }

    impl ApplicationHandler for WinitApp {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            let window_attributes = Window::default_attributes()
                .with_title("Game")
                .with_visible(true)
                .with_inner_size(winit::dpi::LogicalSize::new(800, 600))
                .with_position(winit::dpi::LogicalPosition::new(100, 100));
            let window = event_loop.create_window(window_attributes).unwrap();

            let gpu = pollster::block_on(Gpu::new(Arc::new(window)));
            self.app.insert_resource(gpu);

            let plugins = plugin_group!(
                physics::PhysicsPlugin,
                render::RenderPlugin,
                audio::AudioPlugin,
                render::ui::UiPlugin,
                utils::UtilPlugin::client(),
                networking::NetworkingPlugin::client(),
            );

            self.app.add_plugin(plugins);

            self.app.add_system(display_sprite, SystemStage::Update);
            self.app.add_system(control_player, SystemStage::Update);
            self.app.add_system(spin, SystemStage::Update);
            self.app.add_system(init_scene, SystemStage::Init);

            self.app.init();
            self.app.run();
        }

        fn window_event(
            &mut self,
            event_loop: &ActiveEventLoop,
            _id: WindowId,
            event: WindowEvent,
        ) {
            match event {
                WindowEvent::CloseRequested => {
                    event_loop.exit();
                    self.app.de_init();
                }
                WindowEvent::RedrawRequested => {
                    self.app.run();
                }
                _ => {
                    let window_events = self.app.get_resource_mut::<input::WindowEvents>();
                    if let Some(window_events) = window_events {
                        window_events.events.push(event.clone());
                    }
                }
            }
        }

        fn device_event(
            &mut self,
            _event_loop: &ActiveEventLoop,
            _device_id: winit::event::DeviceId,
            event: winit::event::DeviceEvent,
        ) {
            let device_events = self.app.get_resource_mut::<input::DeviceEvents>();
            if let Some(device_events) = device_events {
                device_events.events.push(event);
            }
        }

        fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
            self.app.run();
        }
    }

    app.insert_resource(input::WindowEvents { events: Vec::new() });
    app.insert_resource(input::DeviceEvents { events: Vec::new() });

    let mut app = WinitApp { app };

    let event_loop = EventLoop::builder()
        .build()
        .expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    event_loop
        .run_app(&mut app)
        .expect("Failed to run event loop");

    // Makes call to std::process::exit to avoid double drop of resources
    std::process::exit(0);
}

system! {
    fn init_scene(
        images: res &Images,
        gpu: res &Gpu,
        audio: res &Audio,
        commands: commands,
    ) {
        let (Some(gpu), Some(images), Some(audio)) = (gpu, images, audio) else {
            return;
        };

        audio.play("example", 0.2, true);

        let sprite = commands.spawn_entity();
        commands.add_component(sprite, SpriteBuilder::default().build(gpu, images));

        let entity = commands.spawn_entity();
        commands.add_component(entity, Transform::default());
        commands.add_component(entity, ModelHandle { path: "sphere".into() });
        commands.add_component(entity, MaterialHandle { name: "test_mat".into() });

        let cube = commands.spawn_entity();
        commands.add_component(cube, Transform {
            pos: Vec3::new(2.0, 0.0, 0.0),
            ..Default::default()
        });
        commands.add_component(cube, ModelHandle { path: "cube".into() });
        commands.add_component(cube, MaterialHandle { name: "test_mat".into() });

        use rand::prelude::*;
        let mut rng = rand::rng();

        for _ in 0..50 {
            let pos = Vec3::new(
                rng.random_range(-50.0..=50.0),
                0.0,
                rng.random_range(-50.0..=50.0),
            );

            let cube = commands.spawn_entity();
            commands.add_component(cube, Transform {
                pos,
                ..Default::default()
            });
            commands.add_component(cube, ModelHandle { path: "cube".into() });
            commands.add_component(cube, MaterialHandle { name: "test_mat".into() });
        }

        for _ in 0..50 {
            let pos = Vec3::new(
                rng.random_range(-50.0..=50.0),
                0.0,
                rng.random_range(-50.0..=50.0),
            );

            let cube = commands.spawn_entity();
            commands.add_component(cube, Transform {
                pos,
                ..Default::default()
            });
            commands.add_component(cube, ModelHandle { path: "sphere".into() });
            commands.add_component(cube, MaterialHandle { name: "test_mat".into() });
        }

        for i in 0..50 {
            let light = commands.spawn_entity();
            commands.add_component(light, Transform {
                pos: Vec3::new(rng.random_range(-50.0..=50.0), 5.0, rng.random_range(-50.0..=50.0)),
                ..Default::default()
            });

            let hue = rng.random_range((-std::f32::consts::PI)..=std::f32::consts::PI);
            let saturation = 1.0;
            let value = 1.0;

            fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Vec3 {
                let c = v * s;
                let x = c * (1.0 - ((h / (std::f32::consts::PI / 3.0)).rem_euclid(2.0) - 1.0).abs());
                let m = v - c;

                let (r1, g1, b1) = if h < std::f32::consts::PI / 3.0 {
                    (c, x, 0.0)
                } else if h < 2.0 * std::f32::consts::PI / 3.0 {
                    (x, c, 0.0)
                } else if h < std::f32::consts::PI {
                    (0.0, c, x)
                } else if h < 4.0 * std::f32::consts::PI / 3.0 {
                    (0.0, x, c)
                } else if h < 5.0 * std::f32::consts::PI / 3.0 {
                    (x, 0.0, c)
                } else {
                    (c, 0.0, x)
                };

                Vec3::new(r1 + m, g1 + m, b1 + m)
            }

            let color = hsv_to_rgb(hue, saturation, value) * 10.0;

            commands.add_component(light, Light {
                brightness: color,
            });
        }

        let camera_entity = commands.spawn_entity();
        commands.add_component(camera_entity, Transform {
            pos: Vec3::new(0.0, 0.0, -5.0),
            rot: Quat::look_to_rh(Vec3::Z, Vec3::Y),
            ..Default::default()
        });

        commands.add_component(camera_entity, Camera::new(
            45.0_f32.to_radians(),
            800.0 / 600.0,
            0.1,
            1000.0,
        ));
    }
}

system! {
    fn display_sprite(
        gpu: res &mut Gpu,
        sprites: query (&Sprite),
    ) {
        let Some(gpu) = gpu else {
            return;
        };

        for sprite in sprites {
            gpu.display(sprite, (200.0, 200.0), (4.0, 4.0), 0.0, Align::Center);
        }
    }
}

system! {
    fn spin(
        time: res &Time,
        objects: query (&mut Transform, &ModelHandle),
    ) {
        let Some(time) = time else {
            return;
        };

        let delta = time.delta_seconds;
        for (transform, _) in objects {
            transform.rot = (Quat::from_axis_angle(Vec3::Y, delta) * transform.rot).normalize();
        }
    }
}

system! {
    fn control_player(
        input: res &mut Input,
        time: res &Time,
        player: query (&mut Transform, &Camera),
    ) {
        let Some(input) = input else {
            return;
        };

        let Some(time) = time else {
            return;
        };

        let Some((player_transform, _camera)) = player.next() else {
            return;
        };

        if input.is_mouse_button_just_pressed(winit::event::MouseButton::Left) {
            input.cursor_grabbed = true;
        }

        if input.is_key_just_pressed(winit::keyboard::KeyCode::Escape) {
            input.cursor_grabbed = false;
        }


        let mut forward = player_transform.rot * -Vec3::Z;
        forward.y = 0.0;
        forward = forward.normalize();

        let mut right = player_transform.rot * Vec3::X;
        right.y = 0.0;
        right = right.normalize();

        let up = Vec3::Y;

        let mut movement = Vec3::ZERO;

        if input.is_key_pressed(winit::keyboard::KeyCode::KeyW) {
            movement += forward;
        }
        if input.is_key_pressed(winit::keyboard::KeyCode::KeyS) {
            movement -= forward;
        }

        if input.is_key_pressed(winit::keyboard::KeyCode::KeyA) {
            movement -= right;
        }
        if input.is_key_pressed(winit::keyboard::KeyCode::KeyD) {
            movement += right;
        }

        if input.is_key_pressed(winit::keyboard::KeyCode::KeyE) {
            movement += up;
        }
        if input.is_key_pressed(winit::keyboard::KeyCode::KeyQ) {
            movement -= up;
        }

        // uses `length_squared` to avoid a square root calculation
        if movement.length_squared() > 0.0 {
            movement = movement.normalize();
            movement = movement * 5.0 * time.delta_seconds;
            player_transform.pos += movement;
        }

        let (mouse_dx, mouse_dy) = input.get_mouse_delta();
        if input.cursor_grabbed && (mouse_dx != 0.0 || mouse_dy != 0.0) {
            let sensitivity = 0.0008;
            let yaw = -mouse_dx as f32 * sensitivity;
            let pitch = -mouse_dy as f32 * sensitivity;

            let cur_rot = player_transform.rot;
            let cur_euler = cur_rot.to_euler(EulerRot::YXZ);
            let new_pitch = (cur_euler.1 + pitch).clamp(-std::f32::consts::FRAC_PI_2 + 0.01, std::f32::consts::FRAC_PI_2 - 0.01);
            let pitch = new_pitch - cur_euler.1;

            let yaw_rot = Quat::from_axis_angle(Vec3::Y, yaw);
            let pitch_rot = Quat::from_axis_angle(right, pitch);
            player_transform.rot = (yaw_rot * pitch_rot * player_transform.rot).normalize();
        }
    }
}

```

`src/lib.rs`:

```rust
pub use ecs::*;
pub use networking::*;

pub mod audio;
pub mod physics;
pub mod render;
pub mod utils;

pub use audio::*;
pub use physics::*;
pub use render::model::*;
pub use render::*;
pub use utils::time::*;
pub use utils::*;

```

`src/physics/mod.rs`:

```rust
use crate::*;
use glam::{Mat4, Quat, Vec3};
use std::{cmp::Ordering, collections::HashMap};

pub mod test;

pub use test::{BodyHandle, BodyInit, BodyState, PhysicsTestWorld};

const DEFAULT_GRAVITY: Vec3 = Vec3::new(0.0, -9.81, 0.0);
const DEFAULT_FIXED_DT: f32 = 1.0 / 60.0;

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PhysicsWorld::default());
        app.insert_resource(PhysicsTime::default());
        app.insert_resource(PhysicsEvents::default());
        app.insert_resource(PhysicsDebugSettings::default());

        app.add_system(sync_ecs_to_physics, SystemStage::PreUpdate);
        app.add_system(run_physics_step, SystemStage::Update);
        app.add_system(sync_physics_to_ecs, SystemStage::PostUpdate);
        app.add_system(emit_physics_events, SystemStage::PostUpdate);
    }
}

#[derive(Component)]
pub struct Transform {
    pub pos: Vec3,
    pub scale: Vec3,
    pub rot: Quat,
}

#[derive(Component)]
pub struct Rotation2D(pub f32);

impl Default for Transform {
    fn default() -> Self {
        Self {
            pos: Vec3::ZERO,
            scale: Vec3 {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            },
            rot: Quat::look_to_rh(-Vec3::Z, Vec3::Y),
        }
    }
}

impl Transform {
    pub fn to_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rot, self.pos)
    }

    pub fn from_matrix(mat: Mat4) -> Self {
        let (scale, rot, pos) = mat.to_scale_rotation_translation();
        Self { pos, scale, rot }
    }

    pub fn to_view_matrix(&self) -> Mat4 {
        let translation = Mat4::from_translation(-self.pos);
        let rotation = Mat4::from_quat(self.rot.conjugate());
        rotation * translation
    }
}

#[derive(Component)]
pub struct Camera {
    pub fov_y: f32,
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
}

impl Camera {
    pub fn new(fov_y: f32, aspect: f32, near: f32, far: f32) -> Self {
        Self {
            fov_y,
            aspect,
            near,
            far,
        }
    }

    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov_y, self.aspect, self.near, self.far)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BodyType {
    Dynamic,
    Static,
}

#[derive(Component, Clone, Debug)]
pub struct RigidBody {
    pub body_type: BodyType,
    pub mass: f32,
}

impl RigidBody {
    pub fn dynamic(mass: f32) -> Self {
        let mass = mass.max(f32::EPSILON);
        Self {
            body_type: BodyType::Dynamic,
            mass,
        }
    }

    pub fn static_body() -> Self {
        Self {
            body_type: BodyType::Static,
            mass: f32::INFINITY,
        }
    }

    pub fn inverse_mass(&self) -> f32 {
        match self.body_type {
            BodyType::Dynamic => 1.0 / self.mass,
            BodyType::Static => 0.0,
        }
    }

    pub fn is_static(&self) -> bool {
        matches!(self.body_type, BodyType::Static)
    }
}

#[derive(Component, Clone, Debug)]
pub enum Collider {
    Sphere { radius: f32 },
    Box { half_extents: Vec3 },
    Capsule { half_height: f32, radius: f32 },
}

impl Collider {
    pub fn sphere(radius: f32) -> Self {
        Self::Sphere { radius }
    }

    pub fn cuboid(half_extents: Vec3) -> Self {
        Self::Box { half_extents }
    }

    pub fn capsule(half_height: f32, radius: f32) -> Self {
        Self::Capsule {
            half_height,
            radius,
        }
    }
}

#[derive(Component, Clone, Copy, Debug, Default)]
pub struct Velocity(pub Vec3);

#[derive(Component, Clone, Copy, Debug, Default)]
pub struct AngularVelocity(pub Vec3);

#[derive(Component, Clone, Copy, Debug, Default)]
pub struct ForceAccumulator(pub Vec3);

#[derive(Component, Clone, Debug, Default)]
pub struct PhysicsMaterial {
    pub restitution: f32,
    pub friction: f32,
}

#[derive(Component, Clone, Copy, Debug, Default)]
pub struct Sleeping(pub bool);

#[derive(Component, Clone, Copy, Debug, Default)]
pub struct PhysicsProxy;

#[derive(Clone, Debug)]
pub struct PhysicsBody {
    pub entity: u32,
    pub rigid_body: RigidBody,
    pub collider: Collider,
    pub velocity: Velocity,
    pub angular_velocity: AngularVelocity,
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
    pub accumulated_force: Vec3,
}

impl PhysicsBody {
    fn new(
        entity: u32,
        rigid_body: RigidBody,
        collider: Collider,
        transform: &Transform,
        velocity: Velocity,
        angular_velocity: AngularVelocity,
        accumulated_force: Vec3,
    ) -> Self {
        Self {
            entity,
            rigid_body,
            collider,
            velocity,
            angular_velocity,
            position: transform.pos,
            rotation: transform.rot,
            scale: transform.scale,
            accumulated_force,
        }
    }

    fn aabb(&self) -> (Vec3, Vec3) {
        match &self.collider {
            Collider::Sphere { radius } => {
                let r = radius.abs() * self.scale.max_element();
                let extents = Vec3::splat(r);
                (self.position - extents, self.position + extents)
            }
            Collider::Box { half_extents } => {
                let extents = Vec3::new(
                    half_extents.x * self.scale.x.abs(),
                    half_extents.y * self.scale.y.abs(),
                    half_extents.z * self.scale.z.abs(),
                );
                (self.position - extents, self.position + extents)
            }
            Collider::Capsule {
                half_height,
                radius,
            } => {
                let radial = radius.abs();
                let half_height = half_height.abs();
                let extents = Vec3::new(
                    radial * self.scale.x.abs(),
                    (half_height + radial) * self.scale.y.abs(),
                    radial * self.scale.z.abs(),
                );
                (self.position - extents, self.position + extents)
            }
        }
    }
}

#[inline]
fn aabb_overlap(min_a: Vec3, max_a: Vec3, min_b: Vec3, max_b: Vec3) -> bool {
    !(max_a.x < min_b.x
        || max_b.x < min_a.x
        || max_a.y < min_b.y
        || max_b.y < min_a.y
        || max_a.z < min_b.z
        || max_b.z < min_a.z)
}

#[derive(Resource, Debug)]
pub struct PhysicsWorld {
    gravity: Vec3,
    bodies: Vec<PhysicsBody>,
    entity_map: HashMap<u32, usize>,
    broad_phase_pairs: Vec<(u32, u32)>,
}

impl Default for PhysicsWorld {
    fn default() -> Self {
        Self {
            gravity: DEFAULT_GRAVITY,
            bodies: Vec::new(),
            entity_map: HashMap::new(),
            broad_phase_pairs: Vec::new(),
        }
    }
}

impl PhysicsWorld {
    pub fn new(gravity: Vec3) -> Self {
        Self {
            gravity,
            bodies: Vec::new(),
            entity_map: HashMap::new(),
            broad_phase_pairs: Vec::new(),
        }
    }

    pub fn gravity(&self) -> Vec3 {
        self.gravity
    }

    pub fn set_gravity(&mut self, gravity: Vec3) {
        self.gravity = gravity;
    }

    pub fn body_count(&self) -> usize {
        self.bodies.len()
    }

    pub fn get_body(&self, entity: u32) -> Option<&PhysicsBody> {
        self.entity_map
            .get(&entity)
            .and_then(|index| self.bodies.get(*index))
    }

    pub fn bodies(&self) -> &[PhysicsBody] {
        &self.bodies
    }

    pub fn broad_phase_pairs(&self) -> &[(u32, u32)] {
        &self.broad_phase_pairs
    }

    fn clear(&mut self) {
        self.bodies.clear();
        self.entity_map.clear();
        self.broad_phase_pairs.clear();
    }

    fn add_body(&mut self, body: PhysicsBody) {
        let index = self.bodies.len();
        self.entity_map.insert(body.entity, index);
        self.bodies.push(body);
    }

    fn rebuild_broad_phase(&mut self) {
        self.broad_phase_pairs.clear();

        let mut entries: Vec<_> = self
            .bodies
            .iter()
            .map(|body| {
                let (min, max) = body.aabb();
                (min, max, body.entity)
            })
            .collect();

        entries.sort_by(|a, b| {
            a.0.x
                .partial_cmp(&b.0.x)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.2.cmp(&b.2))
        });

        for i in 0..entries.len() {
            let (min_a, max_a, ent_a) = entries[i];
            for j in (i + 1)..entries.len() {
                let (min_b, max_b, ent_b) = entries[j];
                if min_b.x > max_a.x {
                    break;
                }

                if aabb_overlap(min_a, max_a, min_b, max_b) {
                    let pair = if ent_a < ent_b {
                        (ent_a, ent_b)
                    } else {
                        (ent_b, ent_a)
                    };
                    self.broad_phase_pairs.push(pair);
                }
            }
        }

        self.broad_phase_pairs.sort();
        self.broad_phase_pairs.dedup();
    }
}

#[derive(Resource, Debug)]
pub struct PhysicsTime {
    pub fixed_delta: f32,
    accumulator: f32,
}

impl Default for PhysicsTime {
    fn default() -> Self {
        Self {
            fixed_delta: DEFAULT_FIXED_DT,
            accumulator: 0.0,
        }
    }
}

impl PhysicsTime {
    pub fn accumulate(&mut self, dt: f32) {
        self.accumulator += dt;
    }

    pub fn consume_step(&mut self) -> bool {
        if self.accumulator >= self.fixed_delta {
            self.accumulator -= self.fixed_delta;
            true
        } else {
            false
        }
    }
}

#[derive(Default, Resource, Debug)]
pub struct PhysicsEvents {
    pub contacts: Vec<PhysicsContactEvent>,
    pub broad_phase_pairs: Vec<(u32, u32)>,
}

#[derive(Clone, Debug, Default)]
pub struct PhysicsContactEvent {
    pub entity_a: u32,
    pub entity_b: u32,
}

#[derive(Default, Resource, Debug)]
pub struct PhysicsDebugSettings {
    pub show_contacts: bool,
}

system!(
    fn sync_ecs_to_physics(
        physics_world: res &mut PhysicsWorld,
        bodies: query (
            &EntityId,
            &Transform,
            &RigidBody,
            &Collider,
            &Velocity,
            &AngularVelocity,
            &mut ForceAccumulator
        )
    ) {
        let Some(world) = physics_world else { return; };

        world.clear();

        for (entity_id, transform, rigid_body, collider, velocity, angular_velocity, force_accumulator) in bodies {
            let accumulated_force = force_accumulator.0;
            force_accumulator.0 = Vec3::ZERO;

            let body = PhysicsBody::new(
                entity_id.get(),
                rigid_body.clone(),
                collider.clone(),
                transform,
                *velocity,
                *angular_velocity,
                accumulated_force,
            );
            world.add_body(body);
        }

        world.rebuild_broad_phase();
    }
);

system!(
    fn run_physics_step(
        physics_time: res &mut PhysicsTime,
        physics_world: res &mut PhysicsWorld,
    ) {
        let (Some(time), Some(world)) = (physics_time, physics_world) else {
            return;
        };

        let gravity = world.gravity;
        let dt = time.fixed_delta;

        while time.consume_step() {
            for body in world.bodies.iter_mut() {
                if body.rigid_body.is_static() {
                    continue;
                }

                let inverse_mass = body.rigid_body.inverse_mass();
                let external_acceleration = body.accumulated_force * inverse_mass;
                let total_acceleration = gravity + external_acceleration;

                body.velocity.0 += total_acceleration * dt;
                body.position += body.velocity.0 * dt;

                let angular_speed = body.angular_velocity.0.length();
                if angular_speed > f32::EPSILON {
                    let axis = body.angular_velocity.0 / angular_speed;
                    let delta_angle = angular_speed * dt;
                    let delta_rot = Quat::from_axis_angle(axis, delta_angle);
                    body.rotation = (delta_rot * body.rotation).normalize();
                }

                body.accumulated_force = Vec3::ZERO;
            }
        }

        world.rebuild_broad_phase();
    }
);

system!(
    fn sync_physics_to_ecs(
        physics_world: res &PhysicsWorld,
        mut targets: query (
            &EntityId,
            &mut Velocity,
            &mut AngularVelocity,
            &mut Transform
        ),
    ) {
        let Some(world) = physics_world else { return; };

        for (entity_id, velocity, angular_velocity, transform) in targets {
            if let Some(body) = world.get_body(entity_id.get()) {
                velocity.0 = body.velocity.0;
                angular_velocity.0 = body.angular_velocity.0;
                transform.pos = body.position;
                transform.rot = body.rotation;
                transform.scale = body.scale;
            }
        }
    }
);

system!(
    fn emit_physics_events(
        physics_world: res &PhysicsWorld,
        physics_events: res &mut PhysicsEvents
    ) {
        let (Some(world), Some(events)) = (physics_world, physics_events) else {
            return;
        };

        events.contacts.clear();
        events.broad_phase_pairs.clear();
        events.broad_phase_pairs.extend(world.broad_phase_pairs().iter().copied());
    }
);

```

`src/physics/test.rs`:

```rust
use glam::Vec3;
use rand::{Rng, SeedableRng, rngs::StdRng};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BodyHandle(pub(crate) usize);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyState {
    pub position: Vec3,
    pub velocity: Vec3,
    pub mass: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyInit {
    pub position: Vec3,
    pub velocity: Vec3,
    pub mass: f32,
}

impl Default for BodyInit {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            velocity: Vec3::ZERO,
            mass: 1.0,
        }
    }
}

#[derive(Clone, Debug)]
struct TestBody {
    position: Vec3,
    velocity: Vec3,
    mass: f32,
}

impl TestBody {
    fn new(init: BodyInit) -> Self {
        Self {
            position: init.position,
            velocity: init.velocity,
            mass: init.mass.max(f32::EPSILON),
        }
    }

    fn state(&self) -> BodyState {
        BodyState {
            position: self.position,
            velocity: self.velocity,
            mass: self.mass,
        }
    }
}

pub struct PhysicsTestWorld {
    gravity: Vec3,
    dt: f32,
    seed: u64,
    rng: StdRng,
    bodies: Vec<TestBody>,
}

impl PhysicsTestWorld {
    pub fn new() -> Self {
        let seed = 0;
        Self {
            gravity: Vec3::new(0.0, -9.81, 0.0),
            dt: 1.0 / 60.0,
            seed,
            rng: StdRng::seed_from_u64(seed),
            bodies: Vec::new(),
        }
    }

    pub fn with_gravity(mut self, gravity: Vec3) -> Self {
        self.gravity = gravity;
        self
    }

    pub fn with_dt(mut self, dt: f32) -> Self {
        self.dt = dt;
        self
    }

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.reseed(seed);
        self
    }

    pub fn reseed(&mut self, seed: u64) {
        self.seed = seed;
        self.rng = StdRng::seed_from_u64(seed);
    }

    pub fn gravity(&self) -> Vec3 {
        self.gravity
    }

    pub fn dt(&self) -> f32 {
        self.dt
    }

    pub fn add_body(&mut self, init: BodyInit) -> BodyHandle {
        let handle = BodyHandle(self.bodies.len());
        self.bodies.push(TestBody::new(init));
        handle
    }

    pub fn spawn_random_body(&mut self) -> BodyHandle {
        let rng = &mut self.rng;
        let position = Vec3::new(
            rng.random_range(-2.0..=2.0),
            rng.random_range(0.5..=3.0),
            rng.random_range(-2.0..=2.0),
        );
        let velocity = Vec3::new(
            rng.random_range(-1.0..=1.0),
            rng.random_range(-1.0..=1.0),
            rng.random_range(-1.0..=1.0),
        );

        self.add_body(BodyInit {
            position,
            velocity,
            mass: 1.0,
        })
    }

    pub fn body_count(&self) -> usize {
        self.bodies.len()
    }

    pub fn body_state(&self, handle: BodyHandle) -> Option<BodyState> {
        self.bodies.get(handle.0).map(TestBody::state)
    }

    pub fn step(&mut self, steps: u32) {
        for _ in 0..steps {
            for body in &mut self.bodies {
                body.velocity += self.gravity * self.dt;
                body.position += body.velocity * self.dt;
            }
        }
    }

    pub fn total_kinetic_energy(&self) -> f32 {
        self.bodies
            .iter()
            .map(|body| 0.5 * body.mass * body.velocity.length_squared())
            .sum()
    }

    pub fn total_potential_energy(&self) -> f32 {
        let g = self.gravity;
        self.bodies
            .iter()
            .map(|body| -body.mass * g.dot(body.position))
            .sum()
    }

    pub fn total_energy(&self) -> f32 {
        self.total_kinetic_energy() + self.total_potential_energy()
    }

    pub fn clear_bodies(&mut self) {
        self.bodies.clear();
    }
}

```

`src/render/model.rs`:

```rust
use std::path::PathBuf;

use crate::*;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

pub struct Model {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub vertex_count: u32,
    pub index_count: u32,
}

#[derive(Component)]
pub struct ModelHandle {
    pub path: String,
}

impl Model {
    pub fn load_obj(path: &PathBuf, gpu: &Gpu) -> Option<Self> {
        let contents = std::fs::read_to_string(path).ok()?;

        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut uvs = Vec::new();
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for line in contents.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            match parts[0] {
                "v" => {
                    if parts.len() >= 4 {
                        positions.push([
                            parts[1].parse().ok()?,
                            parts[2].parse().ok()?,
                            parts[3].parse().ok()?,
                        ]);
                    }
                }
                "vn" => {
                    if parts.len() >= 4 {
                        normals.push([
                            parts[1].parse().ok()?,
                            parts[2].parse().ok()?,
                            parts[3].parse().ok()?,
                        ]);
                    }
                }
                "vt" => {
                    if parts.len() >= 3 {
                        uvs.push([parts[1].parse().ok()?, parts[2].parse().ok()?]);
                    }
                }
                "f" => {
                    if parts.len() >= 4 {
                        let mut face_indices = Vec::new();
                        for i in 1..parts.len() {
                            let indices_str: Vec<&str> = parts[i].split('/').collect();
                            if indices_str.len() >= 3 {
                                let pos_idx: usize = indices_str[0].parse().ok()?;
                                let uv_idx: usize = indices_str[1].parse().ok()?;
                                let normal_idx: usize = indices_str[2].parse().ok()?;

                                if pos_idx > 0
                                    && uv_idx > 0
                                    && normal_idx > 0
                                    && pos_idx <= positions.len()
                                    && uv_idx <= uvs.len()
                                    && normal_idx <= normals.len()
                                {
                                    face_indices.push((pos_idx - 1, uv_idx - 1, normal_idx - 1));
                                }
                            }
                        }

                        if face_indices.len() >= 3 {
                            for i in 1..face_indices.len() - 1 {
                                let v0 = face_indices[0];
                                let v1 = face_indices[i];
                                let v2 = face_indices[i + 1];

                                // For simplicity, just add new vertices for each triangle
                                // In a real implementation, you'd want to deduplicate vertices
                                for &(pos, uv, normal) in &[v0, v1, v2] {
                                    vertices.push(Vertex {
                                        position: positions[pos],
                                        normal: normals[normal],
                                        uv: uvs[uv],
                                    });
                                }

                                let base_index = vertices.len() as u16 - 3;
                                indices.extend_from_slice(&[
                                    base_index,
                                    base_index + 1,
                                    base_index + 2,
                                ]);
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        if vertices.is_empty() {
            println!("No vertices loaded from OBJ file");
            return None;
        }

        let vertex_buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("OBJ Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let index_buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("OBJ Index Buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });

        Some(Model {
            vertex_buffer,
            index_buffer,
            vertex_count: vertices.len() as u32,
            index_count: indices.len() as u32,
        })
    }

    pub fn get_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3, // position
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3, // normal
                },
                wgpu::VertexAttribute {
                    offset: (std::mem::size_of::<[f32; 3]>() + std::mem::size_of::<[f32; 3]>())
                        as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2, // uv
                },
            ],
        }
    }

    pub fn load(path: &PathBuf, gpu: &Gpu) -> Option<Self> {
        let file_extension = path.extension()?.to_str()?;
        match file_extension {
            "obj" => Self::load_obj(path, gpu),
            _ => {
                eprintln!("Unsupported model format: {}", file_extension);
                None
            }
        }
    }

    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..self.index_count, 0, 0..1);
    }
}

```

`src/render/mod.rs`:

```rust
pub mod model;
pub mod sprite;
pub mod ui;

use crate::*;

use model::{Model, ModelHandle};

use crate::physics::{Camera, Transform};

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use anyhow::Result;
use image::{ImageBuffer, Rgba};
use wgpu::util::DeviceExt;
use wgpu::{Extent3d, Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages};
use winit::window::Window;

#[derive(Resource)]
pub struct Images {
    pub images: HashMap<String, ImageBuffer<Rgba<u8>, Vec<u8>>>,
}

impl Images {
    pub fn load() -> Result<Self> {
        let images = gather_dir("textures", |path| {
            let img = image::open(path).ok()?.to_rgba8();
            Some(img)
        })?;
        Ok(Self { images })
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct MaterialData {
    albedo: [f32; 4],
    metallic: f32,
    roughness: f32,
    ao: f32,
    padding: f32,
}

#[derive(Component, Clone)]
pub struct MaterialHandle {
    pub name: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Material {
    pub albedo: String,
    pub metallic: String,
    pub roughness: String,
    pub ao: String,
}

#[derive(Resource)]
pub struct Materials {
    pub materials: HashMap<String, LoadedMaterial>,
}

pub struct LoadedMaterial {
    pub albedo: Texture,
    pub metallic: Texture,
    pub roughness: Texture,
    pub ao: Texture,
    pub albedo_sampler: wgpu::Sampler,
    pub metallic_sampler: wgpu::Sampler,
    pub roughness_sampler: wgpu::Sampler,
    pub ao_sampler: wgpu::Sampler,
}

pub fn image_to_texture(gpu: &crate::render::Gpu, img: &ImageBuffer<Rgba<u8>, Vec<u8>>) -> Texture {
    let size = Extent3d {
        width: img.width(),
        height: img.height(),
        depth_or_array_layers: 1,
    };
    let texture = gpu.device.create_texture(&TextureDescriptor {
        label: Some("Material Texture"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba8UnormSrgb,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        view_formats: &[],
    });
    gpu.queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &img.as_raw().as_slice(),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * (size.width as u32)),
            rows_per_image: Some(size.height as u32),
        },
        size,
    );
    texture
}

impl Materials {
    pub fn load(gpu: &Gpu, images: &Images) -> Result<Self> {
        let materials = crate::gather_dir("materials", |path| {
            let json = std::fs::read_to_string(path).ok()?;
            let mat: Material = serde_json::from_str(&json).ok()?;
            let get_tex = |name: &str| {
                if name.starts_with("#") {
                    let color_code = &name[1..];

                    if color_code.len() == 6 {
                        let r = u8::from_str_radix(&color_code[0..2], 16).ok()?;
                        let g = u8::from_str_radix(&color_code[2..4], 16).ok()?;
                        let b = u8::from_str_radix(&color_code[4..6], 16).ok()?;

                        return Some(image_to_texture(
                            gpu,
                            &ImageBuffer::from_fn(1, 1, |_, _| Rgba([r, g, b, 255])),
                        ));
                    } else if color_code.len() == 2 {
                        let r = u8::from_str_radix(&color_code[0..2], 16).ok()?;

                        return Some(image_to_texture(
                            gpu,
                            &ImageBuffer::from_fn(1, 1, |_, _| Rgba([r, 0, 0, 255])),
                        ));
                    }
                }

                let img = images.images.get(name)?;
                Some(image_to_texture(gpu, img))
            };
            let sampler = gpu
                .device
                .create_sampler(&wgpu::SamplerDescriptor::default());
            Some(LoadedMaterial {
                albedo: get_tex(&mat.albedo)?,
                metallic: get_tex(&mat.metallic)?,
                roughness: get_tex(&mat.roughness)?,
                ao: get_tex(&mat.ao)?,
                albedo_sampler: sampler.clone(),
                metallic_sampler: sampler.clone(),
                roughness_sampler: sampler.clone(),
                ao_sampler: sampler,
            })
        })?;
        Ok(Self { materials })
    }
}

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        let gpu = app.get_resource_mut::<Gpu>().unwrap();
        let images = Images::load().expect("Failed to load images");
        let shaders = Shaders::load(gpu);
        let models = Models::load(gpu);
        let materials = Materials::load(gpu, &images).expect("Failed to load materials");

        app.insert_resource(images);
        app.insert_resource(shaders);
        app.insert_resource(models);
        app.insert_resource(materials);

        app.add_system(render_system, SystemStage::Render);
        app.add_system(update_camera_aspect_ratio, SystemStage::PreUpdate);
    }
}

#[derive(Deserialize, Copy, Clone)]
pub enum Align {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

pub trait Displayable {
    fn get_texture_and_size(&self) -> (&wgpu::Texture, wgpu::Extent3d);
}

impl Displayable for Box<dyn Displayable> {
    fn get_texture_and_size(&self) -> (&wgpu::Texture, wgpu::Extent3d) {
        (**self).get_texture_and_size()
    }
}

pub struct Quad {
    pub texture: Rc<wgpu::Texture>,
    pub rect: (f32, f32, f32, f32), // x, y, width, height
    pub rot: f32,
    pub depth: f32,
}

#[derive(Resource)]
pub struct Gpu {
    pub window: Arc<Window>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub surface: wgpu::Surface<'static>,
    pub surface_format: wgpu::TextureFormat,

    pub depth_texture: Option<wgpu::Texture>,

    pub quads: Vec<Quad>,
}

#[derive(Resource)]
pub struct Shaders {
    pub shaders: HashMap<String, wgpu::ShaderModule>,
    pub model_pipeline: wgpu::RenderPipeline,
    pub model_bind_group_layout: wgpu::BindGroupLayout,
    pub quad_pipeline: wgpu::RenderPipeline,
    pub quad_bind_group_layout: wgpu::BindGroupLayout,
}

impl Shaders {
    pub fn load(gpu: &Gpu) -> Self {
        let shaders = crate::gather_dir("shaders", |path| {
            let file_extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

            let shader = match file_extension {
                //#[cfg(debug_assertions)]
                "wgsl" => gpu
                    .device
                    .create_shader_module(wgpu::ShaderModuleDescriptor {
                        label: path.to_str(),
                        source: wgpu::ShaderSource::Wgsl(
                            std::fs::read_to_string(&path)
                                .expect("Failed to read shader file")
                                .into(),
                        ),
                    }),
                /*#[cfg(not(debug_assertions))]
                "spv" => {
                    let shader_data: Vec<u8> =
                        std::fs::read(&path).expect("Failed to read shader file");
                    let source = wgpu::util::make_spirv(&shader_data);

                    gpu.device
                        .create_shader_module(wgpu::ShaderModuleDescriptor {
                            label: path.to_str(),
                            source,
                        })
                }*/
                _ => {
                    println!(
                        "Warning: Unsupported shader file extension: .{} at {:?}",
                        file_extension, path
                    );
                    return None;
                }
            };

            Some(shader)
        })
        .unwrap();

        let (model_pipeline, model_bind_group_layout) = Self::create_model_pipeline(gpu, &shaders);
        let (quad_pipeline, quad_bind_group_layout) = Self::create_quad_pipeline(gpu, &shaders);

        Self {
            shaders,
            model_pipeline,
            model_bind_group_layout,
            quad_pipeline,
            quad_bind_group_layout,
        }
    }

    fn create_model_pipeline(
        gpu: &Gpu,
        shaders: &HashMap<String, wgpu::ShaderModule>,
    ) -> (wgpu::RenderPipeline, wgpu::BindGroupLayout) {
        let vs_module = shaders.get("vs_main").expect("vs_main shader not found");
        let fs_module = shaders.get("fg_main").expect("fg_main shader not found");

        let bind_group_layout =
            gpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Model Bind Group Layout"),
                    entries: &[
                        // Uniforms
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Albedo
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 4,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                        // Metallic
                        wgpu::BindGroupLayoutEntry {
                            binding: 5,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 6,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                        // Roughness
                        wgpu::BindGroupLayoutEntry {
                            binding: 7,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 8,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                        // AO
                        wgpu::BindGroupLayoutEntry {
                            binding: 9,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 10,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                });

        let pipeline_layout = gpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Model Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        let pipeline = gpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Model Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: vs_module,
                    entry_point: Some("main"),
                    buffers: &[Model::get_vertex_layout()],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: fs_module,
                    entry_point: Some("main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: gpu.surface_format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth24Plus,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
                cache: None,
            });

        (pipeline, bind_group_layout)
    }

    fn create_quad_pipeline(
        gpu: &Gpu,
        shaders: &HashMap<String, wgpu::ShaderModule>,
    ) -> (wgpu::RenderPipeline, wgpu::BindGroupLayout) {
        let vs_module = shaders.get("quad_vs").expect("quad_vs shader not found");
        let fs_module = shaders.get("quad_fs").expect("quad_fs shader not found");

        let bind_group_layout =
            gpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Quad Bind Group Layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                });

        let pipeline_layout = gpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Quad Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        let pipeline = gpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Quad Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: vs_module,
                    entry_point: Some("main"),
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<[f32; 5]>() as wgpu::BufferAddress, // x, y, z, u, v
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x3,
                                offset: 0,
                                shader_location: 0,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x2,
                                offset: 12,
                                shader_location: 1,
                            },
                        ],
                    }],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: fs_module,
                    entry_point: Some("main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: gpu.surface_format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING), // Enable blending for transparency
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: None, // No depth for quads
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
                cache: None,
            });

        (pipeline, bind_group_layout)
    }
}

#[derive(Resource)]
pub struct Models {
    pub models: HashMap<String, Model>,
}

impl Models {
    pub fn load(gpu: &Gpu) -> Self {
        let models = crate::gather_dir("models", |path| Model::load(path, gpu)).unwrap();

        Self { models }
    }
}

impl Gpu {
    pub async fn new(window: Arc<Window>) -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .unwrap();

        let size = window.inner_size();

        let surface = instance.create_surface(window.clone()).unwrap();
        let cap = surface.get_capabilities(&adapter);
        let surface_format = cap.formats[0];

        let mut state = Self {
            window,
            device,
            queue,
            size,
            surface,
            surface_format,

            depth_texture: None,

            quads: Vec::new(),
        };

        state.configure_surface();

        state
    }

    pub fn get_window(&self) -> &Window {
        &self.window
    }

    pub fn configure_surface(&mut self) {
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.surface_format,
            view_formats: vec![],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: self.size.width,
            height: self.size.height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::AutoVsync,
        };
        self.surface.configure(&self.device, &surface_config);

        let depth_size = wgpu::Extent3d {
            width: self.size.width,
            height: self.size.height,
            depth_or_array_layers: 1,
        };

        let depth_desc = wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: depth_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };

        let depth_texture = self.device.create_texture(&depth_desc);
        self.depth_texture = Some(depth_texture);
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        self.configure_surface();
    }

    pub fn display(
        &mut self,
        item: &dyn Displayable,
        location: (f32, f32),
        scale: (f32, f32),
        rot: f32,
        depth: f32,
        align: Align,
    ) {
        let (texture, size) = item.get_texture_and_size();
        let size = (size.width as f32 * scale.0, size.height as f32 * scale.1);

        let (x, y) = match align {
            Align::TopLeft => (location.0, location.1),
            Align::TopCenter => (location.0 - size.0 / 2.0, location.1),
            Align::TopRight => (location.0 - size.0, location.1),
            Align::CenterLeft => (location.0, location.1 - size.1 / 2.0),
            Align::Center => (location.0 - size.0 / 2.0, location.1 - size.1 / 2.0),
            Align::CenterRight => (location.0 - size.0, location.1 - size.1 / 2.0),
            Align::BottomLeft => (location.0, location.1 - size.1),
            Align::BottomCenter => (location.0 - size.0 / 2.0, location.1 - size.1),
            Align::BottomRight => (location.0 - size.0, location.1 - size.1),
        };

        let rect = (x, y, size.0, size.1);

        let quad = Quad {
            texture: Rc::new(texture.clone()),
            rect,
            rot,
            depth,
        };
        self.insert_quad(quad);
    }

    fn insert_quad(&mut self, quad: Quad) {
        let pos = self
            .quads
            .binary_search_by(|q| q.depth.partial_cmp(&quad.depth).unwrap());
        let pos = match pos {
            Ok(pos) => pos,
            Err(pos) => pos,
        };
        self.quads.insert(pos, quad);
    }
}

system! {
    fn update_camera_aspect_ratio(
        gpu: res &Gpu,
        cameras: query (&mut Camera),
    ) {
        let Some(gpu) = gpu else {
            return;
        };

        for camera in cameras {
            camera.aspect = gpu.size.width as f32 / gpu.size.height as f32;
        }
    }
}

use glam::Vec3;

#[derive(Component)]
pub struct Light {
    pub brightness: Vec3,
}

impl Light {
    fn get_buffer(&self, transform: &Transform) -> [f32; 8] {
        [
            transform.pos.x,
            transform.pos.y,
            transform.pos.z,
            0.0,
            self.brightness.x,
            self.brightness.y,
            self.brightness.z,
            0.0,
        ]
    }
}

system!(
    fn render_system(
        gpu: res &mut Gpu,
        shaders: res &Shaders,
        models: res &Models,
        materials: res &Materials,

        to_display: query (&Transform, &ModelHandle, &MaterialHandle),
        lights: query (&Transform, &Light),
        camera: query (&Transform, &Camera),
    ) {
        let (Some(gpu), Some(shaders), Some(models), Some(materials)) = (gpu, shaders, models, materials) else {
            return;
        };

        let surface_texture = gpu
            .surface
            .get_current_texture()
            .expect("failed to acquire next swapchain texture");

        let texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                format: Some(gpu.surface_format),
                ..Default::default()
            });

        if let Some((transform, camera)) = camera.next() {
            let mut encoder = gpu.device.create_command_encoder(&Default::default());
            {
                let depth_view_option = gpu.depth_texture.as_ref().map(|tex| {
                    tex.create_view(&wgpu::TextureViewDescriptor::default())
                });

                let mut renderpass_desc = wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &texture_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.1,
                                g: 0.2,
                                b: 0.3,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                };

                if let Some(depth_view) = depth_view_option.as_ref() {
                    renderpass_desc.depth_stencil_attachment = Some(wgpu::RenderPassDepthStencilAttachment {
                        view: depth_view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    });
                }

                let projection_matrix = camera.projection_matrix();
                let projection_matrix = projection_matrix.to_cols_array_2d();

                let view_matrix = transform.to_view_matrix();
                let view_matrix = view_matrix.to_cols_array_2d();

                let mut light_buffer = Vec::new();
                for (transform, light) in lights {
                    let light = light.get_buffer(transform);
                    light_buffer.extend_from_slice(&light);
                }

                let mut renderpass = encoder.begin_render_pass(&renderpass_desc);

                for model in to_display {
                    let (transform, model_handle, material_handle) = model;

                    let Some(model) = models.models.get(&model_handle.path) else {
                        eprintln!("Model not found: {}", model_handle.path);
                        continue;
                    };

                    let Some(mat) = materials.materials.get(&material_handle.name) else {
                        eprintln!("Material not found: {}", material_handle.name);
                        continue;
                    };

                    let model_matrix = transform.to_matrix();
                    let model_matrix = model_matrix.to_cols_array_2d();

                    let uniforms_data = [
                        model_matrix,
                        view_matrix,
                        projection_matrix,
                    ];
                    let uniforms_buffer = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Uniforms Buffer"),
                        contents: bytemuck::cast_slice(&uniforms_data),
                        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    });

                    let light_buffer = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Light Buffer"),
                        contents: bytemuck::cast_slice(&light_buffer),
                        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                    });

                    let camera_data: [f32; 3] = transform.pos.to_array();
                    let camera_buffer = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Camera Buffer"),
                        contents: bytemuck::cast_slice(&camera_data),
                        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    });

                    let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("Model Bind Group"),
                        layout: &shaders.model_bind_group_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: uniforms_buffer.as_entire_binding(),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: light_buffer.as_entire_binding(),
                            },
                            wgpu::BindGroupEntry {
                                binding: 2,
                                resource: camera_buffer.as_entire_binding(),
                            },
                            wgpu::BindGroupEntry {
                                binding: 3,
                                resource: wgpu::BindingResource::TextureView(&mat.albedo.create_view(&Default::default())),
                            },
                            wgpu::BindGroupEntry {
                                binding: 4,
                                resource: wgpu::BindingResource::Sampler(&mat.albedo_sampler),
                            },
                            wgpu::BindGroupEntry {
                                binding: 5,
                                resource: wgpu::BindingResource::TextureView(&mat.metallic.create_view(&Default::default())),
                            },
                            wgpu::BindGroupEntry {
                                binding: 6,
                                resource: wgpu::BindingResource::Sampler(&mat.metallic_sampler),
                            },
                            wgpu::BindGroupEntry {
                                binding: 7,
                                resource: wgpu::BindingResource::TextureView(&mat.roughness.create_view(&Default::default())),
                            },
                            wgpu::BindGroupEntry {
                                binding: 8,
                                resource: wgpu::BindingResource::Sampler(&mat.roughness_sampler),
                            },
                            wgpu::BindGroupEntry {
                                binding: 9,
                                resource: wgpu::BindingResource::TextureView(&mat.ao.create_view(&Default::default())),
                            },
                            wgpu::BindGroupEntry {
                                binding: 10,
                                resource: wgpu::BindingResource::Sampler(&mat.ao_sampler),
                            },
                        ],
                    });

                    renderpass.set_pipeline(&shaders.model_pipeline);
                    renderpass.set_bind_group(0, &bind_group, &[]);
                    model.render(&mut renderpass);
                }
            }

            gpu.queue.submit([encoder.finish()]);
            let mut encoder = gpu.device.create_command_encoder(&Default::default());

            // Render quads in the same render pass
            {
                let renderpass_desc = wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &texture_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                };

                let mut renderpass = encoder.begin_render_pass(&renderpass_desc);
                let index_buffer = gpu
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Quad Index Buffer"),
                        contents: bytemuck::cast_slice(&[0u16, 1, 2, 2, 3, 0]),
                        usage: wgpu::BufferUsages::INDEX,
                    });

                gpu.quads.iter().for_each(|quad| {
                    let texture_view = quad.texture.create_view(&Default::default());
                    let sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
                        label: Some("Quad Sampler"),
                        address_mode_u: wgpu::AddressMode::ClampToEdge,
                        address_mode_v: wgpu::AddressMode::ClampToEdge,
                        address_mode_w: wgpu::AddressMode::ClampToEdge,
                        mag_filter: wgpu::FilterMode::Linear,
                        min_filter: wgpu::FilterMode::Linear,
                        mipmap_filter: wgpu::FilterMode::Linear,
                        ..Default::default()
                    });
                    let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("Quad Bind Group"),
                        layout: &shaders.quad_bind_group_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(&texture_view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::Sampler(&sampler),
                            },
                        ],
                    });

                    let w = gpu.size.width as f32;
                    let h = gpu.size.height as f32;

                    let hw = quad.rect.2 / 2.0;
                    let hh = quad.rect.3 / 2.0;
                    let cx = quad.rect.0 + hw;
                    let cy = quad.rect.1 + hh;

                    let theta = quad.rot;
                    let cos_t = theta.cos();
                    let sin_t = theta.sin();

                    // Rotate each corner offset
                    let tl_x = -hw * cos_t - (-hh) * sin_t;
                    let tl_y = -hw * sin_t + (-hh) * cos_t;
                    let tr_x = hw * cos_t - (-hh) * sin_t;
                    let tr_y = hw * sin_t + (-hh) * cos_t;
                    let br_x = hw * cos_t - hh * sin_t;
                    let br_y = hw * sin_t + hh * cos_t;
                    let bl_x = -hw * cos_t - hh * sin_t;
                    let bl_y = -hw * sin_t + hh * cos_t;

                    // Convert to NDC
                    let tlx_ndc = ((cx + tl_x) / w * 2.0) - 1.0;
                    let tly_ndc = 1.0 - (cy + tl_y) / h * 2.0;
                    let trx_ndc = ((cx + tr_x) / w * 2.0) - 1.0;
                    let try_ndc = 1.0 - (cy + tr_y) / h * 2.0;
                    let brx_ndc = ((cx + br_x) / w * 2.0) - 1.0;
                    let bry_ndc = 1.0 - (cy + br_y) / h * 2.0;
                    let blx_ndc = ((cx + bl_x) / w * 2.0) - 1.0;
                    let bly_ndc = 1.0 - (cy + bl_y) / h * 2.0;

                    let buffer = gpu
                        .device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Quad Vertex Buffer"),
                            contents: bytemuck::cast_slice(&[
                                tlx_ndc, tly_ndc, quad.depth, 0.0, 0.0,
                                trx_ndc, try_ndc, quad.depth, 1.0, 0.0,
                                brx_ndc, bry_ndc, quad.depth, 1.0, 1.0,
                                blx_ndc, bly_ndc, quad.depth, 0.0, 1.0,
                            ]),
                            usage: wgpu::BufferUsages::VERTEX,
                        });

                    renderpass.set_pipeline(&shaders.quad_pipeline);
                    renderpass.set_bind_group(0, &bind_group, &[]);
                    renderpass.set_vertex_buffer(0, buffer.slice(..));
                    renderpass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                    renderpass.draw_indexed(0..6, 0, 0..1);
                });
            }
            gpu.queue.submit([encoder.finish()]);
        }

        gpu.window.pre_present_notify();
        surface_texture.present();

        gpu.quads.clear();
        gpu.window.request_redraw();
    }
);

```

`src/render/sprite.rs`:

```rust
use image::{ImageBuffer, Rgba};
use wgpu::{Extent3d, TexelCopyBufferLayout, TexelCopyTextureInfo, Texture};

use super::{Displayable, Gpu, Images};

use crate::*;

#[derive(Clone)]
pub struct PalleteSwap {
    pub from: Vec<Rgba<u8>>,
    pub to: Vec<Rgba<u8>>,
}

impl PalleteSwap {
    pub fn new(from: Vec<Rgba<u8>>, to: Vec<Rgba<u8>>) -> Self {
        assert_eq!(from.len(), to.len());
        Self { from, to }
    }

    fn parse_color(s: &str) -> Option<Rgba<u8>> {
        let s = s.trim_start_matches('#');
        if s.len() != 6 && s.len() != 8 {
            return None;
        }
        let r = u8::from_str_radix(&s[0..2], 16).ok()?;
        let g = u8::from_str_radix(&s[2..4], 16).ok()?;
        let b = u8::from_str_radix(&s[4..6], 16).ok()?;
        let a = if s.len() == 8 {
            u8::from_str_radix(&s[6..8], 16).ok()?
        } else {
            255
        };
        Some(Rgba([r, g, b, a]))
    }

    pub fn load(contents: &str) -> Self {
        let mut from = Vec::new();
        let mut to = Vec::new();

        for line in contents.lines() {
            if line.trim().is_empty() || line.trim_start().starts_with("//") {
                continue;
            }

            let parts: Vec<&str> = line.split("->").map(|s| s.trim()).collect();
            if parts.len() != 2 {
                eprintln!("Invalid pallete swap line: {}", line);
                continue;
            }

            if let (Some(f), Some(t)) = (Self::parse_color(parts[0]), Self::parse_color(parts[1])) {
                from.push(f);
                to.push(t);
            } else {
                eprintln!("Invalid color in pallete swap: {}", line);
            }
        }

        Self { from, to }
    }

    pub fn apply(&self, image: &mut ImageBuffer<Rgba<u8>, Vec<u8>>) {
        for pixel in image.pixels_mut() {
            for (i, from_color) in self.from.iter().enumerate() {
                if pixel == from_color {
                    *pixel = self.to[i];
                    break;
                }
            }
        }
    }
}

#[derive(Component)]
pub struct Sprite {
    pub h: u32,
    pub w: u32,
    pub tex: Texture,
}

impl Displayable for Sprite {
    fn get_texture_and_size(&self) -> (&Texture, Extent3d) {
        (
            &self.tex,
            Extent3d {
                width: self.w,
                height: self.h,
                depth_or_array_layers: 1,
            },
        )
    }
}

pub struct SpriteBuilder {
    pub h: u32,
    pub w: u32,
    pub x: u32,
    pub y: u32,

    pub image_path: String,
    pub pallete_swap: Option<PalleteSwap>,
}

impl Default for SpriteBuilder {
    fn default() -> Self {
        Self {
            h: 0,
            w: 0,
            x: 0,
            y: 0,

            image_path: "rawr".to_string(),
            pallete_swap: None,
        }
    }
}

impl SpriteBuilder {
    pub fn build(&self, gpu: &Gpu, images: &Images) -> Sprite {
        let img = images
            .images
            .get(&self.image_path)
            .expect("Failed to load image");
        let mut img = img.clone();
        if self.w != 0 && self.h != 0 {
            img = image::imageops::crop_imm(&img, self.x, self.y, self.w, self.h).to_image();
        }
        if let Some(pallete_swap) = &self.pallete_swap {
            pallete_swap.apply(&mut img);
        }

        let w = img.width();
        let h = img.height();

        let size = wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        };

        let tex = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Sprite Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        gpu.queue.write_texture(
            TexelCopyTextureInfo {
                texture: &tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            img.into_raw().as_slice(),
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * w),
                rows_per_image: Some(h),
            },
            size,
        );

        Sprite { h, w, tex }
    }
}

#[derive(Component)]
pub struct Animation {
    pub frames: Vec<Sprite>,
    pub time_between_frames: f32,
    pub time_accumulator: f32,
    pub current_frame: usize,

    pub looping: bool,
    pub running: bool,
}

impl Displayable for Animation {
    fn get_texture_and_size(&self) -> (&Texture, Extent3d) {
        self.current_sprite().get_texture_and_size()
    }
}

impl Animation {
    pub fn from_frames(frames: Vec<Sprite>, speed: f32, looping: bool, running: bool) -> Self {
        Self {
            frames,
            time_between_frames: if speed == 0.0 { f32::MAX } else { 1.0 / speed },
            time_accumulator: 0.0,
            current_frame: 0,
            looping,
            running,
        }
    }

    pub fn from_spritesheet(
        path: String,
        gpu: &Gpu,
        images: &Images,
        pallete_swap: Option<PalleteSwap>,
        frame_w: u32,
        frame_h: u32,
        speed: f32,
        looping: bool,
        running: bool,
    ) -> Self {
        let img = images
            .images
            .get(&path)
            .expect("Failed to load spritesheet");

        let (sheet_w, sheet_h) = img.dimensions();
        let cols = sheet_w / frame_w;
        let rows = sheet_h / frame_h;

        let mut frames = Vec::new();
        for y in 0..rows {
            for x in 0..cols {
                let sprite = SpriteBuilder {
                    h: frame_h,
                    w: frame_w,
                    x: x * frame_w,
                    y: y * frame_h,
                    image_path: path.clone(),
                    pallete_swap: pallete_swap.clone(),
                }
                .build(gpu, images);

                frames.push(sprite);
            }
        }

        Self {
            frames,
            time_between_frames: if speed == 0.0 { f32::MAX } else { 1.0 / speed },
            time_accumulator: 0.0,
            current_frame: 0,
            looping,
            running,
        }
    }

    pub fn start(&mut self) {
        self.running = true;
    }

    pub fn stop(&mut self) {
        self.running = false;
    }

    pub fn update(&mut self, delta_time: f32) {
        if !self.running {
            return;
        }
        self.time_accumulator += delta_time;
        while self.time_accumulator >= self.time_between_frames {
            if !self.looping && self.current_frame == self.frames.len() - 1 {
                self.running = false;
                return;
            }
            self.current_frame = (self.current_frame + 1) % self.frames.len();
            self.time_accumulator -= self.time_between_frames;
        }
    }

    pub fn current_sprite(&self) -> &Sprite {
        &self.frames[self.current_frame]
    }

    pub fn reset(&mut self) {
        self.current_frame = 0;
        self.time_accumulator = 0.0;
    }

    pub fn advance(&mut self) {
        if !self.looping && self.current_frame == self.frames.len() - 1 {
            return;
        }
        self.current_frame = (self.current_frame + 1) % self.frames.len();
    }

    pub fn retreat(&mut self) {
        if self.current_frame == 0 {
            if !self.looping {
                return;
            }
            self.current_frame = self.frames.len() - 1;
        } else {
            self.current_frame -= 1;
        }
    }

    pub fn is_finished(&self) -> bool {
        !self.looping && self.current_frame == self.frames.len() - 1
    }
}

system!(
    fn update_animations(
        time: res &Time,
        anims: query (&mut Animation),
    ) {
        let Some(time) = time else {
            return;
        };

        for anim in anims {
            anim.update(time.delta_seconds);
        }
    }
);

```

`src/render/ui.rs`:

```rust
use crate::render::sprite::*;
use crate::utils::*;
use crate::*;

use fontdb::{self, ID};
use glyphon::{Cache, Color, FontSystem, Resolution, TextAtlas, TextBounds, TextRenderer};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        let gpu = app.get_resource::<Gpu>().unwrap();
        let images = app.get_resource::<Images>().unwrap();

        let (ui_state, ui_nodes) = UiState::load(gpu, images);
        app.insert_resource(ui_state);
        app.insert_resource(ui_nodes);
        app.add_system(display_ui, SystemStage::PostUpdate);
    }
}

system! {
    fn display_ui(
        ui: res &mut UiState,
        ui_nodes: res &mut UiNodes,
        gpu: res &mut Gpu,
    ) {
        let (Some(ui), Some(ui_nodes), Some(gpu)) = (ui, ui_nodes, gpu) else {
            return;
        };

        ui_nodes.root.display(ui, gpu);
    }
}

#[derive(Resource)]
pub struct UiState {
    toggles: HashSet<String>,
}

#[derive(Resource)]
pub struct UiNodes {
    root: UiNode,
}

impl UiState {
    pub fn show(&mut self, toggle_id: &str) {
        self.toggles.remove(toggle_id);
    }

    pub fn hide(&mut self, toggle_id: &str) {
        self.toggles.insert(toggle_id.to_string());
    }

    fn load(gpu: &Gpu, images: &Images) -> (Self, UiNodes) {
        let mut font_db = fontdb::Database::new();
        let font_map = gather_dir("fonts", |path| {
            if !path
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| matches!(s, "ttf" | "otf" | "woff" | "woff2"))
                .unwrap_or(false)
            {
                return None;
            }

            Some(font_db.load_font_source(fontdb::Source::File(path.to_path_buf()))[0])
        })
        .expect("could not load fonts");
        let mut fonts = FontSystem::new_with_locale_and_db("US".to_string(), font_db);

        let nodes = gather_dir("ui", |path| {
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                return None;
            }
            let file = std::fs::read_to_string(path).unwrap();
            Some(serde_json::from_str::<SerializedUiNode>(&file).unwrap())
        })
        .unwrap();
        let mut state = UiState {
            toggles: HashSet::new(),
        };

        let root = UiNode::from_serialized(
            nodes.get("root").unwrap(),
            &nodes,
            gpu,
            &mut fonts,
            images,
            &font_map,
            &mut state,
        );

        (state, UiNodes { root })
    }
}

#[derive(Deserialize, Copy, Clone)]
struct Rect {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum SerializedUiNode {
    Container {
        toggle_id: Option<String>,
        id: String,
        children: Vec<SerializedUiNode>,
        on_by_default: Option<bool>,
    },
    Text {
        rect: Rect,
        id: String,
        content: String,
        color: Option<String>,
        font: String,
        size: f32,
        align: Option<Align>,
    },
    Image {
        rect: Rect,
        id: String,
        image: String,
        align: Option<Align>,
    },
    SubFile {
        file_path: String,
    },
}

enum UiNode {
    Container {
        toggle_id: Option<String>,
        id: String,
        children: Vec<UiNode>,
    },
    Text {
        rect: Rect,
        text_displayable: TextDisplayable,
        id: String,
        align: Align,
    },
    Image {
        rect: Rect,
        id: String,
        image: Sprite,
        align: Align,
    },
}

impl UiNode {
    fn from_serialized(
        node: &SerializedUiNode,
        nodes: &HashMap<String, SerializedUiNode>,
        gpu: &Gpu,
        fonts: &mut FontSystem,
        images: &Images,
        font_map: &HashMap<String, ID>,
        state: &mut UiState,
    ) -> Self {
        match node {
            SerializedUiNode::Container {
                toggle_id,
                id,
                children,
                on_by_default,
            } => {
                if !on_by_default.unwrap_or(true) {
                    debug_assert!(toggle_id.is_some());
                    state.hide(toggle_id.as_ref().unwrap());
                }
                Self::Container {
                    toggle_id: toggle_id.clone(),
                    id: id.clone(),
                    children: children
                        .iter()
                        .map(|node| {
                            UiNode::from_serialized(
                                node, nodes, gpu, fonts, images, font_map, state,
                            )
                        })
                        .collect(),
                }
            }
            SerializedUiNode::Text {
                rect,
                id,
                content,
                font,
                color,
                size,
                align,
            } => {
                let mut text_displayable = TextDisplayable::new(
                    content.clone(),
                    *font_map.get(font).unwrap(),
                    *size,
                    color.clone().map(|c| {
                        debug_assert!(c.len() == 7 && c.starts_with('#'));
                        let color_code = &c[1..];

                        let r = u8::from_str_radix(&color_code[0..2], 16).unwrap();
                        let g = u8::from_str_radix(&color_code[2..4], 16).unwrap();
                        let b = u8::from_str_radix(&color_code[4..6], 16).unwrap();

                        [r, g, b]
                    }),
                );
                text_displayable
                    .prepare(gpu, fonts)
                    .expect(&format!("failed to prepare text {}", &content));
                Self::Text {
                    rect: *rect,
                    text_displayable,
                    id: id.clone(),
                    align: align.unwrap_or(Align::TopLeft),
                }
            }
            SerializedUiNode::Image {
                rect,
                id,
                image,
                align,
            } => Self::Image {
                rect: *rect,
                id: id.clone(),
                image: {
                    SpriteBuilder {
                        image_path: image.clone(),
                        ..Default::default()
                    }
                    .build(gpu, images)
                },
                align: align.unwrap_or(Align::TopLeft),
            },
            SerializedUiNode::SubFile { file_path } => UiNode::from_serialized(
                nodes.get(file_path).unwrap(),
                nodes,
                gpu,
                fonts,
                images,
                font_map,
                state,
            ),
        }
    }

    fn display(&self, ui: &mut UiState, gpu: &mut Gpu) {
        match self {
            UiNode::Container {
                toggle_id,
                id: _,
                children,
            } => {
                let should_display = if let Some(toggle_id) = toggle_id {
                    !ui.toggles.contains(toggle_id)
                } else {
                    true
                };
                if should_display {
                    for child in children {
                        child.display(ui, gpu);
                    }
                }
            }
            UiNode::Text {
                rect,
                id: _,
                text_displayable,
                align,
            } => {
                gpu.display(
                    text_displayable,
                    (rect.x, rect.y),
                    (rect.width, rect.height),
                    0.0,
                    0.0,
                    *align,
                );
            }
            UiNode::Image {
                rect,
                id,
                image,
                align,
            } => {
                gpu.display(
                    image,
                    (rect.x, rect.y),
                    (rect.width, rect.height),
                    0.0,
                    0.0,
                    *align,
                );
            }
        }
    }
}

pub struct TextDisplayable {
    content: String,
    font: ID,
    size: f32,
    color: [u8; 3],
    texture: Option<wgpu::Texture>,
    extent: Option<wgpu::Extent3d>,
}

impl TextDisplayable {
    pub fn new(content: String, font: ID, size: f32, color: Option<[u8; 3]>) -> Self {
        Self {
            content,
            font,
            size,
            texture: None,
            extent: None,
            color: color.unwrap_or([255, 255, 255]),
        }
    }

    pub fn prepare(&mut self, gpu: &Gpu, fonts: &mut FontSystem) -> anyhow::Result<()> {
        let cache = Cache::new(&gpu.device);
        let mut atlas = TextAtlas::new(
            &gpu.device,
            &gpu.queue,
            &cache,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        );
        let mut swash_cache = glyphon::SwashCache::new();

        let mut renderer = TextRenderer::new(
            &mut atlas,
            &gpu.device,
            wgpu::MultisampleState::default(),
            None,
        );

        let metrics = glyphon::Metrics::new(self.size, self.size * 1.2); // scale and line_height
        let mut buffer = glyphon::Buffer::new(fonts, metrics);
        let attrs = glyphon::Attrs::new();
        buffer.set_text(fonts, &self.content, &attrs, glyphon::Shaping::Advanced);
        buffer.shape_until_scroll(fonts, false);

        let width = buffer
            .layout_runs()
            .map(|run| run.line_w as u32)
            .max()
            .unwrap_or(1);
        let height = buffer.layout_runs().count() as u32 * (self.size as u32);

        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Text Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Text Render Encoder"),
            });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Text Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &texture_view, // Write to the texture
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            let mut viewport = glyphon::Viewport::new(&gpu.device, &cache);

            viewport.update(&gpu.queue, Resolution { width, height });

            let text_areas = vec![glyphon::TextArea {
                buffer: &buffer,
                left: 0.0,
                top: 0.0,
                scale: 1.0,
                bounds: TextBounds::default(),
                default_color: Color::rgb(self.color[0], self.color[1], self.color[2]),
                custom_glyphs: &[],
            }];

            renderer.prepare(
                &gpu.device,
                &gpu.queue,
                fonts,
                &mut atlas,
                &viewport,
                text_areas,
                &mut swash_cache,
            )?;
            renderer
                .render(&atlas, &viewport, &mut render_pass)
                .unwrap();

            atlas.trim();
        }
        gpu.queue.submit(Some(encoder.finish()));

        self.texture = Some(texture);
        self.extent = Some(wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        });

        Ok(())
    }
}

impl Displayable for TextDisplayable {
    fn get_texture_and_size(&self) -> (&wgpu::Texture, wgpu::Extent3d) {
        (
            self.texture.as_ref().expect("Texture not prepared"),
            self.extent.expect("Extent not prepared"),
        )
    }
}

```

`src/server.rs`:

```rust
use glam::*;

pub use ecs::*;
pub use networking::*;

pub mod audio;
pub mod physics;
pub mod render;
pub mod utils;

pub use audio::*;
pub use physics::*;
pub use render::*;
pub use utils::time::*;
pub use utils::*;

#[derive(NetSend, Serialize, Deserialize)]
pub struct TestMessage {
    pub content: String,
}

#[tokio::main]
async fn main() {
    let mut app = App::new();

    let plugins = plugin_group!(
        physics::PhysicsPlugin,
        utils::UtilPlugin::server(),
        networking::NetworkingPlugin::server(),
    );

    app.add_plugin(plugins);

    app.init();

    loop {
        app.run();
        if app.should_exit() {
            break;
        }
    }

    app.de_init();

    std::process::exit(0);
}

```

`src/spin/mod.rs`:

```rust
use crate::*;
use glam::Vec3;

#[derive(Resource)]
pub struct PlayerPosition(pub Vec3);

pub struct Wall {
    pub p1: Vec3,
    pub p2: Vec3,
}
#[derive(Component)]
pub struct Walls(pub Vec<Wall>);

pub enum AIState {
    Idle,
    Sus(f32),
    Noticed(f32),
    Chase(bool),
    Search(f32),
}
#[derive(Component)]
pub struct Ai {
    pub last_position: Vec3,
    pub state: AIState,
}

```

`src/topdown.rs`:

```rust
use std::sync::Arc;

use glam::*;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

pub use ecs::*;
pub use networking::*;

pub mod physics;
pub mod render;
pub mod spin;
pub mod utils;

pub use physics::*;
pub use render::model::ModelHandle;
use render::sprite::*;
pub use render::*;
pub use spin::*;
use utils::input::Input;
pub use utils::time::*;
pub use utils::*;

static UNIT_SIZE: f32 = 32.0;
static SPRITE_SCALE: f32 = 2.0;
static PLAYER_SPEED: f32 = 16.0;
static SCREEN_W: u32 = 1280;
static SCREEN_H: u32 = 720;

static ENEMY_SPEED: f32 = 12.0;
static ENEMY_PERSONAL_SPACE: f32 = 2.0;
static ENEMY_VISION_DIST: f32 = 10.0;
static ENEMY_SUS_TIMER: f32 = 2.0;
static ENEMY_VISION_RADIANS: f32 = 1.0;
static ENEMY_SURPRISE_TIMER: f32 = 0.5;

fn ray_intersects_segment(ray_origin: Vec3, ray_dir: Vec3, ray_len: f32, wall: &Wall) -> bool {
    let wall_dir = wall.p2 - wall.p1;
    let denom = ray_dir.x * wall_dir.y - ray_dir.y * wall_dir.x;
    if denom.abs() < f32::EPSILON {
        return false;
    }

    let diff = wall.p1 - ray_origin;
    let t = (diff.x * wall_dir.y - diff.y * wall_dir.x) / denom;
    let s = (diff.x * ray_dir.y - diff.y * ray_dir.x) / denom;

    t >= 0.0 && t <= ray_len && s >= 0.0 && s <= 1.0
}

#[tokio::main]
async fn main() {
    let mut app = App::new();

    struct WinitApp {
        app: App,
    }

    impl ApplicationHandler for WinitApp {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            let window_attributes = Window::default_attributes()
                .with_title("Game")
                .with_visible(true)
                .with_inner_size(winit::dpi::LogicalSize::new(SCREEN_W, SCREEN_H))
                .with_position(winit::dpi::LogicalPosition::new(100, 100));
            let window = event_loop.create_window(window_attributes).unwrap();

            let gpu = pollster::block_on(Gpu::new(Arc::new(window)));
            self.app.insert_resource(gpu);

            let plugins = plugin_group!(
                // physics::PhysicsPlugin,
                render::RenderPlugin,
                utils::UtilPlugin::client(),
                // networking::NetworkingPlugin::client(),
            );
            self.app.add_plugin(plugins);

            self.app.add_system(update_animations, SystemStage::Update);
            self.app.add_system(draw_sprites, SystemStage::Update);
            self.app.add_system(draw_walls, SystemStage::Update);
            self.app.add_system(control_player, SystemStage::Update);
            self.app.add_system(process_ai, SystemStage::Update);
            self.app.add_system(init_scene, SystemStage::Init);

            self.app.init();
            self.app.run();
        }

        fn window_event(
            &mut self,
            event_loop: &ActiveEventLoop,
            _id: WindowId,
            event: WindowEvent,
        ) {
            match event {
                WindowEvent::CloseRequested => {
                    event_loop.exit();
                    self.app.de_init();
                }
                WindowEvent::RedrawRequested => {
                    self.app.run();
                }
                _ => {
                    let window_events = self.app.get_resource_mut::<input::WindowEvents>();
                    if let Some(window_events) = window_events {
                        window_events.events.push(event.clone());
                    }
                }
            }
        }

        fn device_event(
            &mut self,
            _event_loop: &ActiveEventLoop,
            _device_id: winit::event::DeviceId,
            event: winit::event::DeviceEvent,
        ) {
            let device_events = self.app.get_resource_mut::<input::DeviceEvents>();
            if let Some(device_events) = device_events {
                device_events.events.push(event);
            }
        }

        fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
            self.app.run();
        }
    }

    app.insert_resource(input::WindowEvents { events: Vec::new() });
    app.insert_resource(input::DeviceEvents { events: Vec::new() });

    let mut app = WinitApp { app };

    let event_loop = EventLoop::builder()
        .build()
        .expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    event_loop
        .run_app(&mut app)
        .expect("Failed to run event loop");

    // Makes call to std::process::exit to avoid double drop of resources
    std::process::exit(0);
}

system! {
    fn init_scene(
        images: res &Images,
        gpu: res &Gpu,
        commands: commands,
    ) {
        let (Some(gpu), Some(images)) = (gpu, images) else {
            return;
        };

        // I hath decided: 1 unit is 32 px
        let background = commands.spawn_entity();
        let bg_sprite_size = 2048.0;
        let bg_tile_size = 22.0;
        let bg_scale = UNIT_SIZE * bg_tile_size / bg_sprite_size * SPRITE_SCALE;
        commands.add_component(background, SpriteBuilder {
            image_path: "map_placeholder".to_string(),
            w: bg_sprite_size as u32,
            h: bg_sprite_size as u32,
            ..Default::default()
        }.build(gpu, images));
        commands.add_component(background, Transform {
            pos: Vec3::new(0.0, 0.0, 0.0),
            rot: Quat::look_to_rh(Vec3::Z, Vec3::Y),
            scale: Vec3::new(bg_scale, bg_scale, 0.0),
            ..Default::default()
        });
        commands.add_component(background, Rotation2D(0.0));

        // player
        let player = commands.spawn_entity();
        let player_sprite_size = 256.0;
        let player_scale = UNIT_SIZE / player_sprite_size * SPRITE_SCALE;
        commands.add_component(player, SpriteBuilder {
            image_path: "player_placeholder".to_string(),
            w: player_sprite_size as u32,
            h: player_sprite_size as u32,
            ..Default::default()
        }.build(gpu, images));
        commands.add_component(player, Transform {
            pos: Vec3::new(0.0, 0.0, 0.1),
            rot: Quat::look_to_rh(Vec3::Z, Vec3::Y),
            scale: Vec3::new(player_scale, player_scale, 0.0),
            ..Default::default()
        });
        commands.add_component(player, Camera::new(
            45.0_f32.to_radians(),
            800.0 / 600.0,
            0.1,
            100.0,
        ));
        commands.add_component(player, Rotation2D(3.14 / 4.0));
        commands.insert_resource(PlayerPosition(Vec3::ZERO));

        let enemy = commands.spawn_entity();
        let enemy_sprite_size = 256.0;
        let enemy_scale = UNIT_SIZE / enemy_sprite_size * SPRITE_SCALE;
        commands.add_component(enemy, SpriteBuilder {
            image_path: "enemy_placeholder".to_string(),
            w: enemy_sprite_size as u32,
            h: enemy_sprite_size as u32,
            ..Default::default()
        }.build(gpu, images));
        commands.add_component(enemy, Transform {
            pos: Vec3::new(4.0, 4.0, 0.2),
            rot: Quat::look_to_rh(Vec3::Z, Vec3::Y),
            scale: Vec3::new(enemy_scale, enemy_scale, 0.0),
            ..Default::default()
        });
        commands.add_component(enemy, Rotation2D(0.0));
        commands.add_component(enemy, Ai {
            last_position: Vec3::ZERO,
            state: AIState::Idle,
        });

        // walls container
        let walls = commands.spawn_entity();
        let mut walls_comp = Walls(Vec::new());
        walls_comp.0.push(Wall {
            p1: Vec3::new(-9.0, -10.0, 0.0) * SPRITE_SCALE,
            p2: Vec3::new(-9.0, 2.0, 0.0) * SPRITE_SCALE
        });
        walls_comp.0.push(Wall {
            p1: Vec3::new(-9.0, -10.0, 0.0) * SPRITE_SCALE,
            p2: Vec3::new(9.0, -10.0, 0.0) * SPRITE_SCALE
        });
        walls_comp.0.push(Wall {
            p1: Vec3::new(9.0, -10.0, 0.0) * SPRITE_SCALE,
            p2: Vec3::new(9.0, 7.0, 0.0) * SPRITE_SCALE
        });
        walls_comp.0.push(Wall {
            p1: Vec3::new(9.0, 7.0, 0.0) * SPRITE_SCALE,
            p2: Vec3::new(2.0, 7.0, 0.0) * SPRITE_SCALE
        });
        walls_comp.0.push(Wall {
            p1: Vec3::new(2.0, 7.0, 0.0) * SPRITE_SCALE,
            p2: Vec3::new(2.0, 9.0, 0.0) * SPRITE_SCALE
        });
        walls_comp.0.push(Wall {
            p1: Vec3::new(-2.0, 9.0, 0.0) * SPRITE_SCALE,
            p2: Vec3::new(-2.0, 7.0, 0.0) * SPRITE_SCALE
        });
        walls_comp.0.push(Wall {
            p1: Vec3::new(-2.0, 7.0, 0.0) * SPRITE_SCALE,
            p2: Vec3::new(-9.0, 7.0, 0.0) * SPRITE_SCALE
        });
        walls_comp.0.push(Wall {
            p1: Vec3::new(-9.0, 7.0, 0.0) * SPRITE_SCALE,
            p2: Vec3::new(-9.0, 4.0, 0.0) * SPRITE_SCALE
        });
        walls_comp.0.push(Wall {
            p1: Vec3::new(-9.0, -1.0, 0.0) * SPRITE_SCALE,
            p2: Vec3::new(-7.0, -1.0, 0.0) * SPRITE_SCALE
        });
        walls_comp.0.push(Wall {
            p1: Vec3::new(-5.0, -1.0, 0.0) * SPRITE_SCALE,
            p2: Vec3::new(-1.0, -1.0, 0.0) * SPRITE_SCALE
        });
        walls_comp.0.push(Wall {
            p1: Vec3::new(-4.0, -1.0, 0.0) * SPRITE_SCALE,
            p2: Vec3::new(-4.0, 7.0, 0.0) * SPRITE_SCALE
        });
        walls_comp.0.push(Wall {
            p1: Vec3::new(-2.0, -1.0, 0.0) * SPRITE_SCALE,
            p2: Vec3::new(-2.0, -3.0, 0.0) * SPRITE_SCALE
        });
        walls_comp.0.push(Wall {
            p1: Vec3::new(-2.0, -5.0, 0.0) * SPRITE_SCALE,
            p2: Vec3::new(-2.0, -8.0, 0.0) * SPRITE_SCALE
        });
        walls_comp.0.push(Wall {
            p1: Vec3::new(-2.0, -9.0, 0.0) * SPRITE_SCALE,
            p2: Vec3::new(-2.0, -10.0, 0.0) * SPRITE_SCALE
        });
        walls_comp.0.push(Wall {
            p1: Vec3::new(2.0, -1.0, 0.0) * SPRITE_SCALE,
            p2: Vec3::new(2.0, -3.0, 0.0) * SPRITE_SCALE
        });
        walls_comp.0.push(Wall {
            p1: Vec3::new(2.0, -5.0, 0.0) * SPRITE_SCALE,
            p2: Vec3::new(2.0, -8.0, 0.0) * SPRITE_SCALE
        });
        walls_comp.0.push(Wall {
            p1: Vec3::new(2.0, -9.0, 0.0) * SPRITE_SCALE,
            p2: Vec3::new(2.0, -10.0, 0.0) * SPRITE_SCALE
        });
        walls_comp.0.push(Wall {
            p1: Vec3::new(-2.0, -7.0, 0.0) * SPRITE_SCALE,
            p2: Vec3::new(2.0, -7.0, 0.0) * SPRITE_SCALE
        });
        walls_comp.0.push(Wall {
            p1: Vec3::new(1.0, -1.0, 0.0) * SPRITE_SCALE,
            p2: Vec3::new(4.0, -1.0, 0.0) * SPRITE_SCALE
        });
        // TODO: why does this wall break everything?
        // walls_comp.0.push(Wall {
        //     p1: Vec3::new(4.0, -1.0, 0.0) * SPRITE_SCALE,
        //     p2: Vec3::new(4.0, 7.0, 0.0) * SPRITE_SCALE
        // });
        commands.add_component(walls, walls_comp);
        commands.add_component(walls, SpriteBuilder {
            image_path: "rawr".to_string(),
            w: UNIT_SIZE as u32,
            h: UNIT_SIZE as u32,
            ..Default::default()
        }.build(gpu, images));
    }
}

system! {
    fn draw_sprites(
        gpu: res &mut Gpu,
        sprites: query (&Sprite, &Transform, &Rotation2D),
        animations: query (&Animation, &Transform, &Rotation2D),
        player: query (&Transform, &Camera)
    ) {
        let Some(gpu) = gpu else {return;};
        let Some((player_transform, _camera)) = player.next() else {return;};

        for (sprite, transform, rotation) in sprites {
            let relative_x = transform.pos.x - player_transform.pos.x;
            let relative_y = transform.pos.y - player_transform.pos.y;
            let z_index = transform.pos.z;
            let x_px = relative_x * UNIT_SIZE + SCREEN_W as f32 / 2.0;
            let y_px = relative_y * UNIT_SIZE + SCREEN_H as f32 / 2.0;
            gpu.display(sprite,
                (x_px, y_px),
                (transform.scale.x, transform.scale.y),
                rotation.0,
                z_index,
                Align::Center
            );
        }

        for (animation, transform, rotation) in animations {
            let relative_x = transform.pos.x - player_transform.pos.x;
            let relative_y = transform.pos.y - player_transform.pos.y;
            let z_index = transform.pos.z;
            let x_px = relative_x * UNIT_SIZE + SCREEN_W as f32 / 2.0;
            let y_px = relative_y * UNIT_SIZE + SCREEN_H as f32 / 2.0;
            gpu.display(animation, (x_px, y_px), (transform.scale.x, transform.scale.y), rotation.0, z_index, Align::Center);
        }
    }
}

system! {
    fn process_ai(
        time: res &Time,
        player_pos: res &PlayerPosition,
        enemies: query (&mut Transform, &mut Rotation2D, &mut Ai),
        walls_comp: query (&Walls)
    ) {
        let Some(time) = time else {return;};
        let Some(player_pos) = player_pos else {return;};
        let Some(walls) = walls_comp.next() else {return;};

        for (enemy_transform, enemy_rotation, ai) in enemies {
            let displacement = player_pos.0 - enemy_transform.pos;
            let dist = displacement.length();
            let player_dir = displacement.normalize_or_zero();
            let facing_dir = Vec3::new(enemy_rotation.0.cos(), enemy_rotation.0.sin(), 0.0);

            // Helper: Check if player is visible (in range, FOV, clear LOS)
            let mut is_visible = |current_facing_dir: Vec3| -> bool {
                if dist > ENEMY_VISION_DIST || dist < f32::EPSILON { return false; }
                let dot = current_facing_dir.dot(player_dir);
                if dot.acos() > ENEMY_VISION_RADIANS { return false; }  // Out of FOV half-angle
                // Check LOS: ray to player
                for wall in walls.0.iter() {
                    if ray_intersects_segment(enemy_transform.pos, player_dir, dist, wall) {
                        return false;  // Hits wall before player
                    }
                }
                true
            };

            match ai.state {
                AIState::Idle => {
                    if dist < ENEMY_PERSONAL_SPACE {
                        println!("Too close; sus");
                        ai.state = AIState::Sus(ENEMY_SUS_TIMER);
                        continue;
                    }
                    if !is_visible(facing_dir) { continue; }  // Not visible: stay idle

                    println!("In vision cone and visible; sus");
                    ai.state = AIState::Sus(ENEMY_SUS_TIMER);
                    ai.last_position = player_pos.0;  // Record on detection
                }
                AIState::Sus(mut countdown) => {
                    if !is_visible(facing_dir) {  // Check current facing (pre-turn)
                        println!("Lost sight during sus");
                        ai.state = AIState::Idle;
                        ai.last_position = Vec3::ZERO;
                        continue;
                    }

                    // Still visible: turn to player
                    enemy_rotation.0 = displacement.y.atan2(displacement.x);
                    let facing_dir_after_turn = Vec3::new(enemy_rotation.0.cos(), enemy_rotation.0.sin(), 0.0);
                    if !is_visible(facing_dir_after_turn) {  // Double-check after turn (edge case)
                        ai.state = AIState::Idle;
                        continue;
                    }

                    ai.last_position = player_pos.0;
                    let mut dt = time.delta_seconds;
                    if dist < ENEMY_PERSONAL_SPACE { dt *= 2.0; }
                    countdown -= dt;
                    if countdown <= 0.0 {
                        println!("Sus timer out; noticed");
                        ai.state = AIState::Noticed(ENEMY_SURPRISE_TIMER);
                    } else {
                        ai.state = AIState::Sus(countdown);
                    }
                }
                AIState::Noticed(mut countdown) => {
                    let was_visible = is_visible(facing_dir);
                    if !was_visible {
                        println!("Lost sight during noticed");
                        ai.state = AIState::Search(ENEMY_SUS_TIMER);  // Or Idle; search last pos
                        continue;
                    }

                    // Turn to player (update facing for next frame's visible check)
                    enemy_rotation.0 = displacement.y.atan2(displacement.x);
                    ai.last_position = player_pos.0;

                    countdown -= time.delta_seconds;
                    if countdown <= 0.0 {
                        println!("Noticed timer out; chase");
                        ai.state = AIState::Chase(was_visible);  // Always true here, but for consistency
                    } else {
                        ai.state = AIState::Noticed(countdown);
                    }
                }
                AIState::Chase(initially_visible) => {
                    let target = if initially_visible && is_visible(facing_dir) {
                        player_pos.0  // Still see: chase current
                    } else {
                        ai.last_position  // Lost sight: chase last known
                    };
                    let chase_disp = target - enemy_transform.pos;
                    let chase_dist = chase_disp.length();
                    if chase_dist > f32::EPSILON {
                        let movement = chase_disp.normalize() * ENEMY_SPEED * time.delta_seconds;
                        enemy_rotation.0 = chase_disp.y.atan2(chase_disp.x);
                        enemy_transform.pos += movement;

                        // Optional: If reach last pos and not visible, go to Search or Idle
                        if !initially_visible && chase_dist < 0.5 {  // Threshold
                            ai.state = AIState::Search(3.0);  // Search for 3s
                        }
                    }
                }
                AIState::Search(mut countdown) => {
                    // Simple: face/linger at last pos, check if regain sight
                    if ai.last_position.length_squared() > 0.0 {
                        let search_disp = ai.last_position - enemy_transform.pos;
                        enemy_rotation.0 = search_disp.y.atan2(search_disp.x);
                    }
                    countdown -= time.delta_seconds;
                    if is_visible(facing_dir) {
                        ai.state = AIState::Sus(ENEMY_SUS_TIMER);  // Regain sight
                    } else if countdown <= 0.0 {
                        ai.state = AIState::Idle;
                        ai.last_position = Vec3::ZERO;
                    } else {
                        ai.state = AIState::Search(countdown);
                    }
                }
            }
        }
    }
}

system! {
    fn draw_walls(
        gpu: res &mut Gpu,
        player_pos: res &PlayerPosition,
        walls: query (&Walls, &Sprite),
    ) {
        let Some(gpu) = gpu else {return;};
        let Some(player_pos) = player_pos else {return;};
        let Some((walls_comp, walls_sprite)) = walls.next() else {return;};

        for wall in walls_comp.0.iter() {
            let wall_dir = wall.p2 - wall.p1;
            let wall_ctr = wall.p1 + wall_dir / 2.0;
            let ctr_rx = wall_ctr.x - player_pos.0.x;
            let ctr_ry = wall_ctr.y - player_pos.0.y;
            let x_px = ctr_rx * UNIT_SIZE + SCREEN_W as f32 / 2.0;
            let y_px = ctr_ry * UNIT_SIZE + SCREEN_H as f32 / 2.0;

            let mut scale_x = 0.1;
            let mut scale_y = 0.1;
            if wall_dir.x.abs() > wall_dir.y.abs() {
                scale_x = wall_dir.x.abs();
            } else {
                scale_y = wall_dir.y.abs();
            }
            gpu.display(walls_sprite,
                (x_px, y_px),
                (scale_x, scale_y),
                0.0,
                1.0,
                Align::Center
            )
        }
    }
}

system! {
    fn control_player(
        input: res &mut Input,
        time: res &Time,
        mut player_pos: res &mut PlayerPosition,
        player: query (&mut Transform, &Camera, &mut Rotation2D),
        walls: query (&Walls),
    ) {
        let Some (input) = input else {return;};
        let Some (time) = time else {return;};
        let Some(player_pos) = player_pos else {return;};
        let Some((player_transform, _camera, rotation)) = player.next() else {return;};
        let Some (walls_comp) = walls.next() else {return;};

        // WASD
        let mut movement = Vec3::ZERO;
        if input.is_key_pressed(winit::keyboard::KeyCode::KeyW) {movement -= Vec3::Y;}
        if input.is_key_pressed(winit::keyboard::KeyCode::KeyS) {movement += Vec3::Y;}
        if input.is_key_pressed(winit::keyboard::KeyCode::KeyA) {movement -= Vec3::X;}
        if input.is_key_pressed(winit::keyboard::KeyCode::KeyD) {movement += Vec3::X;}
        movement = movement.normalize();

        // ray intersection
        for wall in walls_comp.0.iter() {
            if ray_intersects_segment(
                player_transform.pos,
                movement,
                PLAYER_SPEED * time.delta_seconds * 8.0, // magic number 👻
                wall
            ) {
                movement = Vec3::ZERO;
            }
        }

        // uses `length_squared` to avoid a square root calculation
        if movement.length_squared() > 0.0 {
            movement = movement * PLAYER_SPEED * time.delta_seconds;
            player_transform.pos += movement;
        }

        let (mousex, mousey) = input.get_mouse_position();
        let to_mousex = mousex - SCREEN_W as f64 / 2.0;
        let to_mousey = mousey - SCREEN_H as f64 / 2.0;
        rotation.0 = to_mousey.atan2(to_mousex) as f32;
        player_pos.0 = player_transform.pos;
    }
}

```

`src/utils/input.rs`:

```rust
use std::collections::HashMap;
use winit::{
    event::WindowEvent,
    keyboard::{KeyCode, PhysicalKey},
};

use crate::*;

#[derive(Resource)]
pub struct WindowEvents {
    pub events: Vec<WindowEvent>,
}

impl WindowEvents {
    pub fn new(events: Vec<WindowEvent>) -> Self {
        Self { events }
    }
}

#[derive(Resource)]
pub struct DeviceEvents {
    pub events: Vec<winit::event::DeviceEvent>,
}

impl DeviceEvents {
    pub fn new(events: Vec<winit::event::DeviceEvent>) -> Self {
        Self { events }
    }
}

system!(
    fn input_system(
        input: res &mut Input,
        gpu: res &mut Gpu,
        events: res &mut WindowEvents,
        device_events: res &mut DeviceEvents,
    ) {
        let Some(events) = events else {
            return;
        };
        let Some(input) = input else {
            return;
        };
        let Some(gpu) = gpu else {
            return;
        };
        let Some(device_events) = device_events else {
            return;
        };

        input.update(gpu, events, device_events);
    }
);

#[derive(Resource)]
pub struct Input {
    keys: HashMap<KeyCode, bool>,
    key_just_pressed: HashMap<KeyCode, bool>,
    mouse_buttons: HashMap<winit::event::MouseButton, bool>,
    mouse_buttons_just_pressed: HashMap<winit::event::MouseButton, bool>,
    prev_mouse_pos: (f64, f64),
    mouse_delta: (f64, f64),
    cursor_in_window: bool,
    pub cursor_grabbed: bool,
}

impl Input {
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
            key_just_pressed: HashMap::new(),
            mouse_buttons: HashMap::new(),
            mouse_buttons_just_pressed: HashMap::new(),
            prev_mouse_pos: (0.0, 0.0),
            mouse_delta: (0.0, 0.0),
            cursor_in_window: false,
            cursor_grabbed: false,
        }
    }

    pub fn update(
        &mut self,
        gpu: &mut Gpu,
        events: &mut WindowEvents,
        device_events: &mut DeviceEvents,
    ) {
        self.key_just_pressed.clear();
        self.mouse_buttons_just_pressed.clear();
        let mut mouse_delta = (0.0, 0.0);

        for event in events.events.drain(..) {
            match event {
                WindowEvent::KeyboardInput { event, .. } => match event.physical_key {
                    PhysicalKey::Code(keycode) => {
                        let pressed = event.state == winit::event::ElementState::Pressed;
                        _ = self.keys.insert(keycode, pressed);
                        _ = self.key_just_pressed.insert(keycode, pressed);
                    }
                    _ => {}
                },
                WindowEvent::MouseInput { state, button, .. } => {
                    let pressed = state == winit::event::ElementState::Pressed;
                    _ = self.mouse_buttons.insert(button, pressed);
                    _ = self.mouse_buttons_just_pressed.insert(button, pressed);
                }
                WindowEvent::Resized(physical_size) => {
                    gpu.resize(physical_size);
                }
                WindowEvent::CursorMoved { position, .. } => {
                    // Only use this for delta if NOT grabbed (raw motion takes priority when grabbed)
                    if !self.cursor_grabbed {
                        if !self.cursor_in_window {
                            self.prev_mouse_pos = (position.x, position.y);
                            self.cursor_in_window = true;
                        }

                        mouse_delta.0 = position.x - self.prev_mouse_pos.0;
                        mouse_delta.1 = position.y - self.prev_mouse_pos.1;
                        self.prev_mouse_pos = (position.x, position.y);
                    }
                }
                WindowEvent::CursorEntered { .. } => {
                    self.cursor_in_window = false;
                }
                _ => {}
            }
        }

        // Process device events for raw mouse motion (when grabbed)
        for event in device_events.events.drain(..) {
            match event {
                winit::event::DeviceEvent::MouseMotion { delta } => {
                    // Use raw delta when cursor is grabbed
                    if self.cursor_grabbed {
                        mouse_delta.0 += delta.0;
                        mouse_delta.1 += delta.1;
                    }
                }
                _ => {}
            }
        }

        if self.cursor_grabbed {
            let _ = gpu
                .window
                .set_cursor_grab(winit::window::CursorGrabMode::Locked);
            gpu.window.set_cursor_visible(false);
        } else {
            let _ = gpu
                .window
                .set_cursor_grab(winit::window::CursorGrabMode::None);
            gpu.window.set_cursor_visible(true);
        }

        self.mouse_delta = mouse_delta;
    }

    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        *self.keys.get(&key).unwrap_or(&false)
    }

    pub fn is_key_just_pressed(&self, key: KeyCode) -> bool {
        *self.key_just_pressed.get(&key).unwrap_or(&false)
    }

    pub fn is_mouse_button_pressed(&self, button: winit::event::MouseButton) -> bool {
        *self.mouse_buttons.get(&button).unwrap_or(&false)
    }

    pub fn is_mouse_button_just_pressed(&self, button: winit::event::MouseButton) -> bool {
        *self
            .mouse_buttons_just_pressed
            .get(&button)
            .unwrap_or(&false)
    }

    pub fn get_mouse_delta(&self) -> (f64, f64) {
        self.mouse_delta
    }

    pub fn get_mouse_position(&self) -> (f64, f64) {
        self.prev_mouse_pos
    }
}

```

`src/utils/mod.rs`:

```rust
use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::utils::input::Input;
use crate::*;

pub mod input;
pub mod time;

pub struct UtilPlugin {
    is_server: bool,
}

impl UtilPlugin {
    pub fn client() -> Self {
        Self { is_server: false }
    }

    pub fn server() -> Self {
        Self { is_server: true }
    }
}
impl Plugin for UtilPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Input::new());

        if !self.is_server {
            app.add_system(input::input_system, SystemStage::PreUpdate);
        }
        app.add_system(time::update_time, SystemStage::PreUpdate);
        app.add_system(time::init_time, SystemStage::Init);
    }
}

pub fn get_resource_path(relative_path: &str) -> PathBuf {
    let path = std::env::current_exe().expect("Can't find path to executable");
    let path = format!(
        "{}/resources/{}",
        path.parent().unwrap().display(),
        relative_path
    );

    PathBuf::from(path)
}

pub fn load_resource_string(relative_path: &str) -> Result<String> {
    let path = get_resource_path(relative_path);
    let content = std::fs::read_to_string(path)?;
    Ok(content)
}

pub fn load_resource_bytes(relative_path: &str) -> Result<Vec<u8>> {
    let path = get_resource_path(relative_path);
    let bytes = std::fs::read(path)?;
    Ok(bytes)
}

pub fn load_resource_json<T: serde::de::DeserializeOwned>(relative_path: &str) -> Result<T> {
    let json = load_resource_string(relative_path)?;
    let data = serde_json::from_str(&json)?;
    Ok(data)
}

pub fn save_resource_string(relative_path: &str, data: &str) -> Result<()> {
    let path = get_resource_path(relative_path);
    std::fs::write(path, data)?;
    Ok(())
}

pub fn save_resource_bytes(relative_path: &str, data: &[u8]) -> Result<()> {
    let path = get_resource_path(relative_path);
    std::fs::write(path, data)?;
    Ok(())
}

pub fn save_resource_json<T: serde::ser::Serialize>(relative_path: &str, data: &T) -> Result<()> {
    let json = serde_json::to_string_pretty(data)?;
    save_resource_string(relative_path, &json)
}

pub fn gather_all_files(root: &PathBuf) -> Result<Vec<PathBuf>> {
    let read_dir = std::fs::read_dir(root)?;
    let mut files = Vec::new();

    for entry in read_dir {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            files.extend(gather_all_files(&path)?);
        } else {
            files.push(path);
        }
    }

    Ok(files)
}

pub fn gather_dir<T>(
    dir: &str,
    mut filter_map: impl FnMut(&PathBuf) -> Option<T>,
) -> Result<HashMap<String, T>> {
    let mut results = HashMap::new();
    let path = get_resource_path(dir);
    for file in gather_all_files(&path)? {
        if let Some(result) = filter_map(&file) {
            let file_extension = file.extension().and_then(|s| s.to_str()).unwrap_or("");

            let relative_dir = file
                .strip_prefix(&path)
                .unwrap()
                .to_str()
                .unwrap()
                .strip_suffix(&format!(".{}", file_extension))
                .unwrap()
                .to_string();

            #[cfg(target_os = "windows")]
            let relative_dir = relative_dir.replace("\\", "/");

            results.insert(relative_dir, result);
        }
    }
    Ok(results)
}

```

`src/utils/time.rs`:

```rust
use crate::*;
use std::time::Instant;

#[derive(Resource)]
pub struct Time {
    pub delta_seconds: f32,
    last_call: Instant,
}

system!(
    fn init_time(commands: commands) {
        commands.insert_resource(Time {
            delta_seconds: 0.0,
            last_call: Instant::now(),
        });
    }
);

system!(
    fn update_time(
        time: res &mut Time,
    ) {
        let Some(time) = time else {
            return;
        };
        let now = Instant::now();
        time.delta_seconds = (now - time.last_call).as_secs_f32();
        time.last_call = now;
    }
);

```

`tests/app.rs`:

```rust
use rust_game_engine::*;

#[derive(Resource, Default)]
struct StageLog(Vec<&'static str>);

system! {
    fn log_pre(log: res &mut StageLog) {
        if let Some(log) = log {
            log.0.push("pre");
        }
    }
}

system! {
    fn log_update(log: res &mut StageLog) {
        if let Some(log) = log {
            log.0.push("update");
        }
    }
}

system! {
    fn log_post(log: res &mut StageLog) {
        if let Some(log) = log {
            log.0.push("post");
        }
    }
}

system! {
    fn log_render(log: res &mut StageLog) {
        if let Some(log) = log {
            log.0.push("render");
        }
    }
}

#[test]
fn systems_execute_in_defined_stage_order() {
    let mut app = App::new();

    app.insert_resource(StageLog::default());

    app.add_system(log_pre, SystemStage::PreUpdate);
    app.add_system(log_update, SystemStage::Update);
    app.add_system(log_post, SystemStage::PostUpdate);
    app.add_system(log_render, SystemStage::Render);

    app.run();

    let commands: &Commands = &app;
    let world_ptr = commands.world;
    let log =
        unsafe { World::get_resource::<StageLog>(world_ptr).expect("StageLog resource not found") };
    assert_eq!(log.0, vec!["pre", "update", "post", "render"]);
}

```

`tests/physics.rs`:

```rust
use rust_game_engine::physics::{
    AngularVelocity, BodyInit, Camera, Collider, ForceAccumulator, PhysicsDebugSettings,
    PhysicsEvents, PhysicsPlugin, PhysicsTestWorld, PhysicsTime, PhysicsWorld, RigidBody,
    Transform, Velocity,
};
use rust_game_engine::{App, Commands, World};

use glam::{Mat4, Quat, Vec3};

fn assert_mat4_close(a: Mat4, b: Mat4, epsilon: f32) {
    let a = a.to_cols_array();
    let b = b.to_cols_array();
    for (ai, bi) in a.iter().zip(b.iter()) {
        assert!(
            (ai - bi).abs() <= epsilon,
            "matrices differ: {} vs {}",
            ai,
            bi
        );
    }

    #[test]
    fn physics_world_broad_phase_pairs_are_deterministic() {
        let mut app = App::new();
        app.add_plugin(PhysicsPlugin);

        let e1 = app.spawn_entity();
        app.add_component(e1, Transform::default()).unwrap();
        app.add_component(e1, RigidBody::dynamic(1.0)).unwrap();
        app.add_component(e1, Collider::sphere(0.5)).unwrap();
        app.add_component(e1, Velocity(Vec3::ZERO)).unwrap();
        app.add_component(e1, AngularVelocity(Vec3::ZERO)).unwrap();
        app.add_component(e1, ForceAccumulator(Vec3::ZERO)).unwrap();

        let mut t2 = Transform::default();
        t2.pos = Vec3::new(0.25, 0.0, 0.0);
        let e2 = app.spawn_entity();
        app.add_component(e2, t2).unwrap();
        app.add_component(e2, RigidBody::dynamic(1.0)).unwrap();
        app.add_component(e2, Collider::sphere(0.5)).unwrap();
        app.add_component(e2, Velocity(Vec3::ZERO)).unwrap();
        app.add_component(e2, AngularVelocity(Vec3::ZERO)).unwrap();
        app.add_component(e2, ForceAccumulator(Vec3::ZERO)).unwrap();

        let mut t3 = Transform::default();
        t3.pos = Vec3::new(5.0, 0.0, 0.0);
        let e3 = app.spawn_entity();
        app.add_component(e3, t3).unwrap();
        app.add_component(e3, RigidBody::dynamic(1.0)).unwrap();
        app.add_component(e3, Collider::sphere(0.5)).unwrap();
        app.add_component(e3, Velocity(Vec3::ZERO)).unwrap();
        app.add_component(e3, AngularVelocity(Vec3::ZERO)).unwrap();
        app.add_component(e3, ForceAccumulator(Vec3::ZERO)).unwrap();

        app.run();

        let commands: &Commands = &app;
        let world_ptr = commands.world;
        let physics_world = unsafe { World::get_resource::<PhysicsWorld>(world_ptr).unwrap() };

        let pairs = physics_world.broad_phase_pairs();
        assert_eq!(pairs, &[(e1, e2)]);

        // Running again without changes should yield the same ordering.
        app.run();
        let physics_world = unsafe { World::get_resource::<PhysicsWorld>(world_ptr).unwrap() };
        assert_eq!(physics_world.broad_phase_pairs(), &[(e1, e2)]);
    }

    #[test]
    fn physics_world_broad_phase_pairs_respect_axis_ordering() {
        let mut app = App::new();
        app.add_plugin(PhysicsPlugin);

        let mut transforms = [
            Vec3::new(-1.5, 0.0, 0.0),
            Vec3::new(-0.5, 0.0, 0.0),
            Vec3::new(0.5, 0.0, 0.0),
            Vec3::new(1.5, 0.0, 0.0),
        ];

        let mut entities = Vec::new();
        for pos in transforms.iter_mut() {
            let mut t = Transform::default();
            t.pos = *pos;
            let entity = app.spawn_entity();
            app.add_component(entity, t).unwrap();
            app.add_component(entity, RigidBody::dynamic(1.0)).unwrap();
            app.add_component(entity, Collider::sphere(0.75)).unwrap();
            app.add_component(entity, Velocity(Vec3::ZERO)).unwrap();
            app.add_component(entity, AngularVelocity(Vec3::ZERO))
                .unwrap();
            app.add_component(entity, ForceAccumulator(Vec3::ZERO))
                .unwrap();
            entities.push(entity);
        }

        app.run();

        let commands: &Commands = &app;
        let world_ptr = commands.world;
        let physics_world = unsafe { World::get_resource::<PhysicsWorld>(world_ptr).unwrap() };

        let expected_pairs = vec![
            (entities[0].min(entities[1]), entities[0].max(entities[1])),
            (entities[1].min(entities[2]), entities[1].max(entities[2])),
            (entities[2].min(entities[3]), entities[2].max(entities[3])),
        ];

        assert_eq!(physics_world.broad_phase_pairs(), expected_pairs);
    }

    #[test]
    fn physics_events_emit_broad_phase_pairs() {
        let mut app = App::new();
        app.add_plugin(PhysicsPlugin);

        let mut t1 = Transform::default();
        t1.pos = Vec3::new(-0.25, 0.0, 0.0);
        let e1 = app.spawn_entity();
        app.add_component(e1, t1).unwrap();
        app.add_component(e1, RigidBody::dynamic(1.0)).unwrap();
        app.add_component(e1, Collider::sphere(0.6)).unwrap();
        app.add_component(e1, Velocity(Vec3::ZERO)).unwrap();
        app.add_component(e1, AngularVelocity(Vec3::ZERO)).unwrap();
        app.add_component(e1, ForceAccumulator(Vec3::ZERO)).unwrap();

        let mut t2 = Transform::default();
        t2.pos = Vec3::new(0.25, 0.0, 0.0);
        let e2 = app.spawn_entity();
        app.add_component(e2, t2).unwrap();
        app.add_component(e2, RigidBody::dynamic(1.0)).unwrap();
        app.add_component(e2, Collider::sphere(0.6)).unwrap();
        app.add_component(e2, Velocity(Vec3::ZERO)).unwrap();
        app.add_component(e2, AngularVelocity(Vec3::ZERO)).unwrap();
        app.add_component(e2, ForceAccumulator(Vec3::ZERO)).unwrap();

        app.run();

        let commands: &Commands = &app;
        let world_ptr = commands.world;
        let events = unsafe { World::get_resource::<PhysicsEvents>(world_ptr).unwrap() };
        assert_eq!(events.broad_phase_pairs, vec![(e1.min(e2), e1.max(e2))]);
    }
}

#[test]
fn transform_matrix_roundtrip() {
    let transform = Transform {
        pos: Vec3::new(1.0, 2.0, 3.0),
        scale: Vec3::new(2.0, 3.0, 4.0),
        rot: Quat::from_euler(glam::EulerRot::XYZ, 0.3, -1.2, 0.7),
    };

    let matrix = transform.to_matrix();
    let expected =
        Mat4::from_scale_rotation_translation(transform.scale, transform.rot, transform.pos);
    assert_mat4_close(matrix, expected, 1e-6);

    let reconstructed = Transform::from_matrix(matrix);
    assert!(transform.pos.abs_diff_eq(reconstructed.pos, 1e-5));
    assert!(transform.scale.abs_diff_eq(reconstructed.scale, 1e-5));
    assert!(transform.rot.abs_diff_eq(reconstructed.rot, 1e-5));
}

#[test]
fn transform_view_matrix_is_inverse_of_model_matrix() {
    let transform = Transform {
        pos: Vec3::new(-5.0, 0.5, 12.0),
        scale: Vec3::ONE,
        rot: Quat::from_rotation_y(0.75),
    };

    let model = transform.to_matrix();
    let view = transform.to_view_matrix();
    let expected_view = model.inverse();

    assert_mat4_close(view, expected_view, 1e-5);
}

#[test]
fn camera_projection_matches_glam_helpers() {
    let camera = Camera::new(55.0_f32.to_radians(), 1920.0 / 1080.0, 0.01, 250.0);
    let projection = camera.projection_matrix();
    let expected = Mat4::perspective_rh(camera.fov_y, camera.aspect, camera.near, camera.far);
    assert_mat4_close(projection, expected, 1e-6);
}

#[test]
fn physics_test_world_initializes_with_defaults() {
    let world = PhysicsTestWorld::new();

    assert_eq!(world.gravity(), Vec3::new(0.0, -9.81, 0.0));
    assert!((world.dt() - (1.0 / 60.0)).abs() < f32::EPSILON);
    assert_eq!(world.body_count(), 0);
}

#[test]
fn physics_test_world_adds_bodies_and_steps() {
    let mut world = PhysicsTestWorld::new();

    let handle = world.add_body(BodyInit {
        position: Vec3::new(0.0, 1.0, 0.0),
        velocity: Vec3::ZERO,
        mass: 2.0,
    });

    assert_eq!(world.body_count(), 1);

    world.step(10);

    let state = world.body_state(handle).expect("body should exist");

    assert!(
        state.velocity.y < 0.0,
        "gravity should accelerate body downward"
    );
    assert!(state.position.y < 1.0, "body should have moved downward");
}

#[test]
fn physics_test_world_energy_helpers_track_system_energy() {
    let mut world = PhysicsTestWorld::new().with_gravity(Vec3::ZERO);

    world.add_body(BodyInit {
        position: Vec3::new(0.0, 0.0, 0.0),
        velocity: Vec3::new(1.0, 0.0, 0.0),
        mass: 3.0,
    });

    let kinetic = world.total_kinetic_energy();
    let potential = world.total_potential_energy();
    let total = world.total_energy();

    assert!(kinetic > 0.0);
    assert_eq!(potential, 0.0);
    assert!((total - kinetic).abs() < 1e-6);

    world.clear_bodies();
    assert_eq!(world.body_count(), 0);
    assert_eq!(world.total_energy(), 0.0);
}

#[test]
fn physics_test_world_seed_controls_randomized_bodies() {
    let mut world_a = PhysicsTestWorld::new().with_seed(42);
    let mut world_b = PhysicsTestWorld::new().with_seed(42);
    let mut world_c = PhysicsTestWorld::new().with_seed(1337);

    let handle_a1 = world_a.spawn_random_body();
    let handle_b1 = world_b.spawn_random_body();
    let handle_c1 = world_c.spawn_random_body();

    let state_a1 = world_a.body_state(handle_a1).unwrap();
    let state_b1 = world_b.body_state(handle_b1).unwrap();
    let state_c1 = world_c.body_state(handle_c1).unwrap();

    assert_eq!(state_a1.position, state_b1.position);
    assert_eq!(state_a1.velocity, state_b1.velocity);
    assert_eq!(state_a1.mass, state_b1.mass);

    assert_ne!(state_a1.position, state_c1.position);
    assert_ne!(state_a1.velocity, state_c1.velocity);

    world_b.reseed(9001);
    let handle_b2 = world_b.spawn_random_body();
    let state_b2 = world_b.body_state(handle_b2).unwrap();

    assert_ne!(state_b1.position, state_b2.position);
}

#[test]
fn physics_plugin_inserts_resources() {
    let mut app = App::new();
    app.add_plugin(PhysicsPlugin);

    let commands: &Commands = &app;
    let world_ptr = commands.world;

    let physics_world =
        unsafe { World::get_resource::<PhysicsWorld>(world_ptr).expect("PhysicsWorld missing") };
    assert_eq!(physics_world.gravity(), Vec3::new(0.0, -9.81, 0.0));
    assert_eq!(physics_world.body_count(), 0);

    unsafe {
        let _time = World::get_resource::<PhysicsTime>(world_ptr).expect("PhysicsTime missing");
        let _events =
            World::get_resource::<PhysicsEvents>(world_ptr).expect("PhysicsEvents missing");
        let _debug = World::get_resource::<PhysicsDebugSettings>(world_ptr)
            .expect("PhysicsDebugSettings missing");
    }
}

#[test]
fn physics_plugin_collects_bodies_from_ecs() {
    let mut app = App::new();
    app.add_plugin(PhysicsPlugin);

    let dynamic_entity = app.spawn_entity();
    app.add_component(dynamic_entity, RigidBody::dynamic(2.0))
        .unwrap();
    app.add_component(dynamic_entity, Collider::sphere(0.5))
        .unwrap();
    app.add_component(dynamic_entity, Transform::default())
        .unwrap();
    app.add_component(dynamic_entity, Velocity(Vec3::new(0.0, 1.0, 0.0)))
        .unwrap();
    app.add_component(dynamic_entity, AngularVelocity(Vec3::ZERO))
        .unwrap();
    app.add_component(dynamic_entity, ForceAccumulator(Vec3::ZERO))
        .unwrap();

    let static_entity = app.spawn_entity();
    app.add_component(static_entity, RigidBody::static_body())
        .unwrap();
    app.add_component(static_entity, Collider::cuboid(Vec3::splat(1.0)))
        .unwrap();
    app.add_component(static_entity, Transform::default())
        .unwrap();
    app.add_component(static_entity, Velocity(Vec3::ZERO))
        .unwrap();
    app.add_component(static_entity, AngularVelocity(Vec3::ZERO))
        .unwrap();
    app.add_component(static_entity, ForceAccumulator(Vec3::ZERO))
        .unwrap();

    {
        let commands: &Commands = &app;
        let world_ptr = commands.world;
        let time = unsafe {
            World::get_resource_mut::<PhysicsTime>(world_ptr).expect("PhysicsTime missing")
        };
        let dt = time.fixed_delta;
        time.accumulate(dt);
    }

    app.run();

    let commands: &Commands = &app;
    let world_ptr = commands.world;

    let physics_world =
        unsafe { World::get_resource::<PhysicsWorld>(world_ptr).expect("PhysicsWorld missing") };
    assert_eq!(physics_world.body_count(), 2);

    let dynamic_body = physics_world
        .get_body(dynamic_entity)
        .expect("dynamic body missing");
    assert!(!dynamic_body.rigid_body.is_static());
    assert_eq!(dynamic_body.accumulated_force, Vec3::ZERO);

    let static_body = physics_world
        .get_body(static_entity)
        .expect("static body missing");
    assert!(static_body.rigid_body.is_static());

    assert!(
        physics_world
            .bodies()
            .iter()
            .any(|body| matches!(body.collider, Collider::Sphere { .. }))
    );
    assert!(
        physics_world
            .bodies()
            .iter()
            .any(|body| matches!(body.collider, Collider::Box { .. }))
    );
}

#[test]
fn physics_plugin_applies_gravity_and_forces() {
    let mut app = App::new();
    app.add_plugin(PhysicsPlugin);

    let entity = app.spawn_entity();
    app.add_component(entity, RigidBody::dynamic(2.0)).unwrap();
    app.add_component(entity, Collider::sphere(0.25)).unwrap();
    app.add_component(entity, Transform::default()).unwrap();
    app.add_component(entity, Velocity(Vec3::ZERO)).unwrap();
    app.add_component(entity, AngularVelocity(Vec3::new(0.0, 1.0, 0.0)))
        .unwrap();
    app.add_component(entity, ForceAccumulator(Vec3::new(4.0, 0.0, 0.0)))
        .unwrap();

    unsafe {
        let commands: &Commands = &app;
        let world_ptr = commands.world;
        let time = World::get_resource_mut::<PhysicsTime>(world_ptr).expect("PhysicsTime missing");
        time.accumulate(time.fixed_delta);
    }

    app.run();

    let commands: &Commands = &app;
    let world_ptr = commands.world;

    unsafe {
        let dt = World::get_resource::<PhysicsTime>(world_ptr)
            .expect("PhysicsTime missing")
            .fixed_delta;

        let velocity = World::get_components::<Velocity>(world_ptr)
            .into_iter()
            .find(|(id, _)| *id == entity)
            .map(|(_, vel)| vel.0)
            .expect("Velocity component missing");

        let transform = World::get_components::<Transform>(world_ptr)
            .into_iter()
            .find(|(id, _)| *id == entity)
            .map(|(_, t)| t)
            .expect("Transform missing");

        let force_after_step = World::get_components::<ForceAccumulator>(world_ptr)
            .into_iter()
            .find(|(id, _)| *id == entity)
            .map(|(_, force)| force.0)
            .expect("ForceAccumulator missing");

        assert!((velocity.y + 9.81 * dt).abs() < 1e-5);
        assert!((velocity.x - 2.0 * dt).abs() < 1e-5);
        assert!(force_after_step.length() < 1e-5);
        assert!(transform.pos.y < 0.0);
    }
}

```

`tests/time.rs`:

```rust
use std::thread;
use std::time::Duration;

use rust_game_engine::utils::time::{self, Time};
use rust_game_engine::{App, Commands, SystemStage, World};

#[test]
fn time_systems_initialize_and_update_delta() {
    let mut app = App::new();

    app.add_system(time::init_time, SystemStage::Init);
    app.add_system(time::update_time, SystemStage::PreUpdate);

    app.init();

    let commands: &Commands = &app;
    let world_ptr = commands.world;

    let time =
        unsafe { World::get_resource::<Time>(world_ptr).expect("Time resource not initialized") };
    assert_eq!(time.delta_seconds, 0.0);

    thread::sleep(Duration::from_millis(5));

    app.run();

    let time =
        unsafe { World::get_resource::<Time>(world_ptr).expect("Time resource not present") };
    assert!(time.delta_seconds > 0.0);
}

```

