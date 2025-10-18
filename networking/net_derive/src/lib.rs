use proc_macro::TokenStream;
use proc_macro2::{Delimiter, Ident, TokenStream as TokenStream2, TokenTree};
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
                get_component_id::<Self>()
            }

            fn into_bytes(&self) -> Vec<u8> {
                bincode::serialize(self).unwrap()
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
                get_component_id::<Self>()
            }

            fn from_bytes(bytes: &[u8]) -> Self {
                bincode::deserialize(bytes).unwrap()
            }
        }

        submit! {
            NetRecvRegistration {
                type_id: ConstTypeId::of::<#name>(),
                name: #type_name,
            }
        }
    }
    .into()
}
