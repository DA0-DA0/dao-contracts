use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, AttributeArgs, DataEnum, DeriveInput, Variant};

/// Adds the nesecary fields to an such that the enum implements the
/// interface needed to be support the vote-hooks msgs.
///
/// For example:
///
/// ```
/// use cwd_vote_hooks_macros::vote_hooks;
/// use cwd_vote_hooks::VoteHookMsg;
///
/// #[vote_hooks]
/// enum ExecuteMsg {}
/// ```
///
/// Will transform the enum to:
///
/// ```
/// use cwd_vote_hooks::VoteHookMsg;
///
/// enum QueryMsg {
///     VoteHook(VoteHookMsg)
/// }
/// ```
///
/// Note that other derive macro invocations must occur after this
/// procedural macro as they may depend on the new fields. For
/// example, the following will fail becase the `Clone` derivation
/// occurs before the addition of the field.
///
/// ```compile_fail
/// use cwd_macros::voting_query;
///
/// #[derive(Clone)]
/// #[vote_hooks]
/// #[allow(dead_code)]
/// enum Test {
///     Foo,
///     Bar(u64),
///     Baz { foo: u64 },
/// }
/// ```
#[proc_macro_attribute]
pub fn vote_hooks(metadata: TokenStream, input: TokenStream) -> TokenStream {
    // Make sure that no arguments were passed in.
    let args = parse_macro_input!(metadata as AttributeArgs);
    if let Some(first_arg) = args.first() {
        return syn::Error::new_spanned(first_arg, "vote hooks macro takes no arguments")
            .to_compile_error()
            .into();
    }

    let mut ast: DeriveInput = parse_macro_input!(input);
    match &mut ast.data {
        syn::Data::Enum(DataEnum { variants, .. }) => {
            let proposal_hook_wrapper: Variant =
                syn::parse2(quote! { VoteHook(VoteHookMsg) }).unwrap();
            variants.push(proposal_hook_wrapper);
        }
        _ => {
            return syn::Error::new(
                ast.ident.span(),
                "vote hooks types can not be only be derived for enums",
            )
            .to_compile_error()
            .into()
        }
    };

    quote! {
    #ast
    }
    .into()
}
