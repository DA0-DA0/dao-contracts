use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};
use proc_macro::TokenStream;
use quote::quote;
use serde::de::DeserializeOwned;
use std::error::Error;

#[proc_macro_attribute]
pub fn cw_verifier_execute(metadata: TokenStream, input: TokenStream) -> TokenStream {
    merge_variants(
        metadata,
        input,
        quote! {
            enum Right {
                VerifyAndExecuteSignedMessage {
                    msg: ::cw_verifier_middleware::WrappedMessage
                },
            }
        }
        .into(),
    )
}
