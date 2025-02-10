use proc_macro::TokenStream;
use quote::{format_ident, quote, ToTokens};
use std::fs;
use std::ops::Deref;
use syn::{
    parse_macro_input, AngleBracketedGenericArguments, FnArg, ItemFn, PatType, PathArguments,
    ReturnType, Type,
};
#[proc_macro_attribute]
pub fn retryable(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);

    let fn_name = &input_fn.sig.ident;
    let struct_name = fn_name; // Struct name matches function name
    let inner_fn_name = format_ident!("{}_inner", fn_name);
    let inputs = &input_fn.sig.inputs;
    let mut ret_type_t: Option<TokenStream> = None;
    let mut ret_type_e: Option<TokenStream> = None;
    let output = match &input_fn.sig.output {
        ReturnType::Type(_, ty) => {
            match ty.deref() {
                Type::Path(p) => {
                    for seg in &p.path.segments {
                        let p2 = &seg.arguments;
                        match p2 {
                            PathArguments::AngleBracketed(ab) => {
                                for x in &ab.args {
                                    if ret_type_t.is_none() {
                                        ret_type_t = Some(x.to_token_stream().into());
                                    } else if ret_type_e.is_none() {
                                        ret_type_e = Some(x.to_token_stream().into());
                                    }
                                }
                            }
                            _ => {}
                        };
                    }
                }
                _ => {}
            };

            quote! { #ty }
        }
        _ => {
            quote! { () }
        }
    };

    if ret_type_t.is_none() || ret_type_e.is_none() {
        panic!("Could not determine Signature for Executor - expected return type to be of the form Retryable<T,E>. Type Aliases that change this form may affect the macros");
    }
    let ret_type_t: proc_macro2::TokenStream = ret_type_t.unwrap().into();
    let ret_type_e: proc_macro2::TokenStream = ret_type_e.unwrap().into();

    let body = &input_fn.block;

    // Extract parameter types for tuple struct
    let struct_fields = inputs.iter().filter_map(|arg| {
        if let FnArg::Typed(PatType { ty, .. }) = arg {
            Some(quote! { #ty })
        } else {
            None
        }
    });

    // Extract parameters for function call
    let param_names = (0..inputs.len()).map(|i| {
        let index = syn::Index::from(i);
        quote! { self.#index.clone() }
    });

    let expanded = quote! {

        #[allow(non_camel_case_types)]
        struct #struct_name(#(#struct_fields,)*);

        #[async_trait]
        impl Executor<#ret_type_t, #ret_type_e> for #struct_name {
            async fn execute(&self) -> #output {
                __RETRIERS__INTERNAL::#inner_fn_name(#(#param_names),*).await
            }
        }


        #[doc(hidden)]
        mod __RETRIERS__INTERNAL{
            use  retriers_lib::*;
            use super::*;
            #[doc(hidden)]
            pub async fn #inner_fn_name(#inputs) -> #output #body
        }
    };

    TokenStream::from(expanded).into()
}

//
// use proc_macro::TokenStream;
// use quote::{quote, format_ident};
// use syn::{parse_macro_input, ItemFn};
//
// #[proc_macro_attribute]
// pub fn retryable(_attr: TokenStream, item: TokenStream) -> TokenStream {
//     let input = parse_macro_input!(item as ItemFn);
//     let fn_name = &input.sig.ident;
//     let struct_name = fn_name; // Struct has the same name as the function
//     let inner_fn_name = format_ident!("{}_inner", fn_name);
//     let args = &input.sig.inputs;
//     let output = &input.sig.output;
//
//     // Extract argument names
//     let arg_names = input.sig.inputs.iter().enumerate().map(|(i, arg)| {
//         match arg {
//             syn::FnArg::Typed(pat) => &pat.pat,
//             _ => panic!("Unsupported argument type"),
//         }
//     });
//
//     let expanded = quote! {
//         struct #struct_name (#args);
//
//         #[async_trait]
//         impl Executor<#output> for #struct_name {
//             async fn execute(&self) -> #output {
//                 let (#(#arg_names),*) = (self.0.clone(), self.1.clone());
//                 #inner_fn_name(#(#arg_names),*).await
//             }
//         }
//
//         async fn #inner_fn_name #args #output #input.block
//     };
//
//     TokenStream::from(expanded)
// }
