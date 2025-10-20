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
                get_send_id::<Self>()
            }

            fn get_bytes(&self) -> Vec<u8> {
                bincode::serde::encode_to_vec::<&#name, bincode::config::Configuration>(self, bincode::config::Configuration::default()).unwrap()
            }
        }

        submit! {
            NetSendRegistration {
                type_id: ConstTypeId::of::<#name>(),
                name: #type_name,
            }
        }
    }
    .into()
}

#[proc_macro_derive(NetRecv)]
pub fn derive_net_recv(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let type_name = name.to_string();

    quote! {
        impl NetRecv for #name {
            fn get_type_id(&self) -> usize {
                get_recv_id::<Self>()
            }

            fn from_bytes(bytes: &[u8]) -> Self {
                bincode::serde::decode_from_slice::<#name, bincode::config::Configuration>(bytes, bincode::config::Configuration::default()).unwrap().0
            }
        }

        submit! {
            NetRecvRegistration {
                type_id: ConstTypeId::of::<#name>(),
                name: #type_name,
                from_bytes: |bytes: &[u8]| -> Box<dyn std::any::Any> { Box::new(#name::from_bytes(bytes)) },
            }
        }
    }
    .into()
}
