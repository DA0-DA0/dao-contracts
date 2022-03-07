use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, AttributeArgs, DataEnum, DeriveInput, Variant};

/// Adds the nesecary fields to an such that the enum implements the
/// interface needed to be a cw-governance voting module.
///
/// For example:
///
/// ```
/// use cw_governance_macros::cw_governance_voting_query;
///
/// #[cw_governance_voting_query]
/// enum QueryMsg {}
/// ```
///
/// Will transform the enum to:
///
/// ```
/// enum QueryMsg {
///     VotingPowerAtHeight {
/// 	  address: String,
/// 	  height: Option<u64>
///     },
///     TotalPowerAtHeight {
///       height: Option<u64>
///     }
/// }
/// ```
///
/// Note that other derive macro invocations must occur after this
/// procedural macro as they may depend on the new fields. For
/// example, the following will fail becase the `Clone` derivation
/// occurs before the addition of the field.
///
/// ```compile_fail
/// use cw_governance_macros::cw_governance_voting_query;
///
/// #[derive(Clone)]
/// #[cw_governance_voting_query]
/// #[allow(dead_code)]
/// enum Test {
///     Foo,
///     Bar(u64),
///     Baz { foo: u64 },
/// }
/// ```
#[proc_macro_attribute]
pub fn cw_governance_voting_query(metadata: TokenStream, input: TokenStream) -> TokenStream {
    // Make sure that no arguments were passed in.
    let args = parse_macro_input!(metadata as AttributeArgs);
    if !args.is_empty() {
        return syn::Error::new_spanned(
            args.into_iter().nth(0),
            "voting query macro takes no arguments",
        )
        .to_compile_error()
        .into();
    }

    let mut ast: DeriveInput = parse_macro_input!(input);
    match &mut ast.data {
        syn::Data::Enum(DataEnum { variants, .. }) => {
            let voting_power: Variant = syn::parse2(quote! { VotingPowerAtHeight {
                address: ::std::string::String,
                height: ::std::option::Option<::std::primitive::u64>
            } })
            .unwrap();

            let total_power: Variant = syn::parse2(quote! { TotalPowerAtHeight {
                height: ::std::option::Option<::std::primitive::u64>
            } })
            .unwrap();

            variants.push(voting_power);
            variants.push(total_power);
        }
        _ => {
            return syn::Error::new(
                ast.ident.span(),
                "voting query types can not be only be derived for enums",
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
