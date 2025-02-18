use syn::{ItemFn, PathArguments, ReturnType, Type};
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, quote_spanned, ToTokens};
use std::ops::Deref;
use syn::spanned::Spanned;
use crate::function_info::FunctionInfo;

pub fn parse_function(input_fn: &ItemFn, original_tokens: proc_macro2::TokenStream) -> FunctionInfo {
    let fn_name = &input_fn.sig.ident;
    let struct_name = fn_name; // Struct name matches function name
    let inputs = &input_fn.sig.inputs;
    let mut ret_type_t: Option<TokenStream> = None;
    let mut ret_type_e: Option<TokenStream> = None;
    let mut ctime_type_loc: Option<Span> = None;
    let output = match &input_fn.sig.output {
        ReturnType::Type(_, ty) => {
            if let Type::Path(p) = ty.deref() {
                ctime_type_loc = Some(p.span());

                for seg in &p.path.segments {
                    let p2 = &seg.arguments;
                    if let PathArguments::AngleBracketed(ab) = p2 {
                        for x in &ab.args {
                            if ret_type_t.is_none() {
                                ret_type_t = Some(x.to_token_stream().into());
                            } else if ret_type_e.is_none() {
                                ret_type_e = Some(x.to_token_stream().into());
                            }
                        }
                    };
                }
            };
            quote! { #ty }
        }
        _ => {
            quote! { () }
        }
    };

    let mut _ctime_err = quote! {};
    if ret_type_t.is_none() || ret_type_e.is_none() {
        let span = ctime_type_loc.unwrap();
        _ctime_err = quote_spanned! {span=>
                compile_error!("Return type must be of the form RetryResult<T, E>. The retryable proc macro is unable to determine the underlying value and error types behind a type alias.");
            };
    }
    let ret_type_t: proc_macro2::TokenStream = ret_type_t.unwrap_or(quote! {()}.into()).into();
    let ret_type_e: proc_macro2::TokenStream = ret_type_e.unwrap_or(quote! {()}.into()).into();

    let body = &input_fn.block;

    FunctionInfo {
        struct_name: struct_name.clone(),
        inputs: inputs.clone(),
        ret_type_t: ret_type_t.clone(),
        ret_type_e: ret_type_e.clone(),
        output: output.clone(),
        original_body: *body.clone(),
        original_tokens: original_tokens.clone(),
        ctime_error: _ctime_err.clone(),
    }
}