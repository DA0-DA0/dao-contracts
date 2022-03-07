use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, AttributeArgs, DataEnum, DeriveInput, Variant};

/// Adds the nesecary fields to an such that the enum implements the
/// interface needed to be a cw-governance voting module.
///
/// For example:
///
/// ```
/// use cw_governance_macros::voting_query;
///
/// #[voting_query]
/// enum QueryMsg {}
/// ```
///
/// Will transform the enum to:
///
/// ```
/// enum QueryMsg {
///     VotingPowerAtHeight {
///       address: String,
///       height: Option<u64>
///     },
///     TotalPowerAtHeight {
///       height: Option<u64>
///     },
///     Info {},
/// }
/// ```
///
/// Note that other derive macro invocations must occur after this
/// procedural macro as they may depend on the new fields. For
/// example, the following will fail becase the `Clone` derivation
/// occurs before the addition of the field.
///
/// ```compile_fail
/// use cw_governance_macros::voting_query;
///
/// #[derive(Clone)]
/// #[voting_query]
/// #[allow(dead_code)]
/// enum Test {
///     Foo,
///     Bar(u64),
///     Baz { foo: u64 },
/// }
/// ```
#[proc_macro_attribute]
pub fn voting_query(metadata: TokenStream, input: TokenStream) -> TokenStream {
    // Make sure that no arguments were passed in.
    let args = parse_macro_input!(metadata as AttributeArgs);
    if let Some(first_arg) = args.first() {
        return syn::Error::new_spanned(first_arg, "voting query macro takes no arguments")
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

            let info: Variant = syn::parse2(quote! { Info {} }).unwrap();

            variants.push(voting_power);
            variants.push(total_power);
            variants.push(info);
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

/// Adds the nesecary fields to an enum such that it implements the
/// interface needed to be a cw-governance governance module.
///
/// For example:
///
/// ```
/// use cw_governance_macros::govmod_query;
///
/// #[govmod_query]
/// enum QueryMsg {}
/// ```
///
/// Will transform the enum to:
///
/// ```
/// enum QueryMsg {
///     Info {},
/// }
/// ```
///
/// Note that other derive macro invocations must occur after this
/// procedural macro as they may depend on the new fields. For
/// example, the following will fail becase the `Clone` derivation
/// occurs before the addition of the field.
///
/// ```compile_fail
/// use cw_governance_macros::govmod_query;
///
/// #[derive(Clone)]
/// #[govmod_query]
/// #[allow(dead_code)]
/// enum Test {
///     Foo,
///     Bar(u64),
///     Baz { foo: u64 },
/// }
/// ```
#[proc_macro_attribute]
pub fn govmod_query(metadata: TokenStream, input: TokenStream) -> TokenStream {
    // Make sure that no arguments were passed in.
    let args = parse_macro_input!(metadata as AttributeArgs);
    if let Some(first_arg) = args.first() {
        return syn::Error::new_spanned(first_arg, "govmod query macro takes no arguments")
            .to_compile_error()
            .into();
    }

    let mut ast: DeriveInput = parse_macro_input!(input);
    match &mut ast.data {
        syn::Data::Enum(DataEnum { variants, .. }) => {
            let info: Variant = syn::parse2(quote! { Info {} }).unwrap();

            variants.push(info);
        }
        _ => {
            return syn::Error::new(
                ast.ident.span(),
                "govmod query types can not be only be derived for enums",
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
