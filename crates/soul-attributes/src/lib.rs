mod error;

use crate::error::{SoulError, SoulResult};

use proc_macro::TokenStream;
use quote::quote;
use std::collections::BTreeMap;
use syn::{
    Error, Expr, ExprLit, Item, Lit, MetaNameValue, Token, parse_macro_input,
    punctuated::Punctuated,
};

fn compile_error(spanned: impl quote::ToTokens, error: SoulError) -> TokenStream {
    Error::new_spanned(spanned, error.to_string())
        .to_compile_error()
        .into()
}

fn parse_fields(
    args: &Punctuated<MetaNameValue, Token![,]>,
) -> SoulResult<BTreeMap<String, String>> {
    let mut fields = BTreeMap::new();

    for arg in args {
        let Some(ident) = arg.path.get_ident() else {
            return Err(SoulError::non_identifier_key());
        };

        let key = ident.to_string();

        let value = match &arg.value {
            Expr::Lit(ExprLit {
                lit: Lit::Str(value),
                ..
            }) => value.value(),
            _ => return Err(SoulError::non_string_value()),
        };

        if key == "id" && value.trim().is_empty() {
            return Err(SoulError::empty_value(&key));
        }

        if fields.insert(key.clone(), value).is_some() {
            return Err(SoulError::duplicate_field(&key));
        }
    }

    if !fields.contains_key("id") {
        return Err(SoulError::missing_id());
    }

    Ok(fields)
}

#[proc_macro_attribute]
pub fn soul(args: TokenStream, item: TokenStream) -> TokenStream {
    let parser = Punctuated::<MetaNameValue, Token![,]>::parse_terminated;
    let args = parse_macro_input!(args with parser);
    let parsed_item = parse_macro_input!(item as Item);

    if let Err(error) = parse_fields(&args) {
        return compile_error(&args, error);
    }

    quote!(#parsed_item).into()
}
