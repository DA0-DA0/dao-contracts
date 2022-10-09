use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, AttributeArgs, DataEnum, DeriveInput, Variant};

/// Adds the necessary fields to an such that the enum implements the
/// interface needed to be a voting module.
///
/// For example:
///
/// ```
/// use cwd_macros::voting_query;
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

/// Adds the necessary fields to an enum such that it implements the
/// interface needed to be a voting module with a token.
///
/// For example:
///
/// ```
/// use cwd_macros::token_query;
///
/// #[token_query]
/// enum QueryMsg {}
/// ```
///
/// Will transform the enum to:
///
/// ```
/// enum QueryMsg {
///     TokenContract {},
/// }
/// ```
///
/// Note that other derive macro invocations must occur after this
/// procedural macro as they may depend on the new fields. For
/// example, the following will fail becase the `Clone` derivation
/// occurs before the addition of the field.
///
/// ```compile_fail
/// use cwd_macros::token_query;
///
/// #[derive(Clone)]
/// #[token_query]
/// #[allow(dead_code)]
/// enum Test {
///     Foo,
///     Bar(u64),
///     Baz { foo: u64 },
/// }
/// ```
#[proc_macro_attribute]
pub fn token_query(metadata: TokenStream, input: TokenStream) -> TokenStream {
    // Make sure that no arguments were passed in.
    let args = parse_macro_input!(metadata as AttributeArgs);
    if let Some(first_arg) = args.first() {
        return syn::Error::new_spanned(first_arg, "token query macro takes no arguments")
            .to_compile_error()
            .into();
    }

    let mut ast: DeriveInput = parse_macro_input!(input);
    match &mut ast.data {
        syn::Data::Enum(DataEnum { variants, .. }) => {
            let info: Variant = syn::parse2(quote! { TokenContract {} }).unwrap();

            variants.push(info);
        }
        _ => {
            return syn::Error::new(
                ast.ident.span(),
                "token query types can not be only be derived for enums",
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

/// Adds the necessary fields to an enum such that it implements the
/// interface needed to be a voting module that has an
/// active check threshold.
///
/// For example:
///
/// ```
/// use cwd_macros::active_query;
///
/// #[active_query]
/// enum QueryMsg {}
/// ```
///
/// Will transform the enum to:
///
/// ```
/// enum QueryMsg {
///     IsActive {},
/// }
/// ```
///
/// Note that other derive macro invocations must occur after this
/// procedural macro as they may depend on the new fields. For
/// example, the following will fail becase the `Clone` derivation
/// occurs before the addition of the field.
///
/// ```compile_fail
/// use cwd_macros::active_query;
///
/// #[derive(Clone)]
/// #[active_query]
/// #[allow(dead_code)]
/// enum Test {
///     Foo,
///     Bar(u64),
///     Baz { foo: u64 },
/// }
/// ```
#[proc_macro_attribute]
pub fn active_query(metadata: TokenStream, input: TokenStream) -> TokenStream {
    // Make sure that no arguments were passed in.
    let args = parse_macro_input!(metadata as AttributeArgs);
    if let Some(first_arg) = args.first() {
        return syn::Error::new_spanned(first_arg, "token query macro takes no arguments")
            .to_compile_error()
            .into();
    }

    let mut ast: DeriveInput = parse_macro_input!(input);
    match &mut ast.data {
        syn::Data::Enum(DataEnum { variants, .. }) => {
            let info: Variant = syn::parse2(quote! { IsActive {} }).unwrap();

            variants.push(info);
        }
        _ => {
            return syn::Error::new(
                ast.ident.span(),
                "token query types can not be only be derived for enums",
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

/// Adds the necessary fields to an enum such that it implements the
/// interface needed to be a module that has an
/// info query.
///
/// For example:
///
/// ```
/// use cwd_macros::info_query;
///
/// #[info_query]
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
/// use cwd_macros::info_query;
///
/// #[derive(Clone)]
/// #[info_query]
/// #[allow(dead_code)]
/// enum Test {
///     Foo,
///     Bar(u64),
///     Baz { foo: u64 },
/// }
/// ```
#[proc_macro_attribute]
pub fn info_query(metadata: TokenStream, input: TokenStream) -> TokenStream {
    // Make sure that no arguments were passed in.
    let args = parse_macro_input!(metadata as AttributeArgs);
    if let Some(first_arg) = args.first() {
        return syn::Error::new_spanned(first_arg, "info query macro takes no arguments")
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
                "info query types can not be only be derived for enums",
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

/// Adds the necessary fields to an enum such that it implements the
/// interface needed to be a proposal module.
///
/// For example:
///
/// ```
/// use cwd_macros::proposal_module_query;
///
/// #[proposal_module_query]
/// enum QueryMsg {}
/// ```
///
/// Will transform the enum to:
///
/// ```
/// enum QueryMsg {
///     Dao {},
/// }
/// ```
///
/// Note that other derive macro invocations must occur after this
/// procedural macro as they may depend on the new fields. For
/// example, the following will fail becase the `Clone` derivation
/// occurs before the addition of the field.
///
/// ```compile_fail
/// use cwd_macros::proposal_module_query;
///
/// #[derive(Clone)]
/// #[proposal_module_query]
/// #[allow(dead_code)]
/// enum Test {
///     Foo,
///     Bar(u64),
///     Baz { foo: u64 },
/// }
/// ```
#[proc_macro_attribute]
pub fn proposal_module_query(metadata: TokenStream, input: TokenStream) -> TokenStream {
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
            let dao: Variant = syn::parse2(quote! { Dao {} }).unwrap();

            variants.push(dao);
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

/// Limits the number of variants allowed on an enum at compile
/// time. For example, the following will not compile:
///
/// ```compile_fail
/// use cwd_macros::limit_variant_count;
///
/// #[limit_variant_count(1)]
/// enum Two {
///     One {},
///     Two {},
/// }
/// ```
#[proc_macro_attribute]
pub fn limit_variant_count(metadata: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(metadata as AttributeArgs);
    if args.len() != 1 {
        panic!("macro takes one argument. ex: `#[limit_variant_count(4)]`")
    }

    let limit: usize = if let syn::NestedMeta::Lit(syn::Lit::Int(unparsed)) = args.first().unwrap()
    {
        match unparsed.base10_parse() {
            Ok(limit) => limit,
            Err(e) => return e.to_compile_error().into(),
        }
    } else {
        return syn::Error::new_spanned(args[0].clone(), "argument should be an integer literal")
            .to_compile_error()
            .into();
    };

    let ast: DeriveInput = parse_macro_input!(input);
    match ast.data {
        syn::Data::Enum(DataEnum { ref variants, .. }) => {
            if variants.len() > limit {
                return syn::Error::new_spanned(
                    variants[limit].clone(),
                    format!("this enum's variant count is limited to {}", limit),
                )
                .to_compile_error()
                .into();
            }
        }
        _ => {
            return syn::Error::new(
                ast.ident.span(),
                "limit_variant_count may only be derived for enums",
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
