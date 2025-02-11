use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote, quote_spanned, ToTokens};
use std::fs;
use std::ops::Deref;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Comma;
use syn::{parse_macro_input, AngleBracketedGenericArguments, FnArg, ItemFn, Pat, PatType, PathArguments, ReturnType, Type};

#[proc_macro_attribute]
pub fn retry_prepare(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let original_tokens: proc_macro2::TokenStream = item.clone().into();
    let input_fn = parse_macro_input!(item as ItemFn);

    let retryable_data = RetryableParseData::from_function(input_fn, original_tokens);

    let expanded = retryable_data.expand_prepared();

    let z = TokenStream::from(expanded);
    let s = z.to_string();
    fs::write("src/generated.txt", s).expect("Unable to write file");
    z
}

#[proc_macro_attribute]
pub fn retry(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let original_tokens: proc_macro2::TokenStream = item.clone().into();
    let input_fn = parse_macro_input!(item as ItemFn);

    let retryable_data = RetryableParseData::from_function(input_fn, original_tokens);

    let expanded = retryable_data.expand_retry();

    let z = TokenStream::from(expanded);
    let s = z.to_string();
    fs::write("src/generated.txt", s).expect("Unable to write file");
    z
}

struct RetryableParseData {
    struct_name: proc_macro2::Ident,
    inputs: Punctuated<FnArg, Comma>,
    ret_type_t: proc_macro2::TokenStream,
    ret_type_e: proc_macro2::TokenStream,

    output: proc_macro2::TokenStream,
    original_body: syn::Block,

    original_tokens: proc_macro2::TokenStream,
    ctime_error: proc_macro2::TokenStream,
}

impl RetryableParseData {
    pub(crate) fn from_function(input_fn: ItemFn, original_tokens: proc_macro2::TokenStream) -> Self {
        let fn_name = &input_fn.sig.ident;
        let struct_name = fn_name; // Struct name matches function name
        let inputs = &input_fn.sig.inputs;
        let mut ret_type_t: Option<TokenStream> = None;
        let mut ret_type_e: Option<TokenStream> = None;
        let mut ctime_type_loc: Option<Span> = None;
        let output = match &input_fn.sig.output {
            ReturnType::Type(_, ty) => {
                match ty.deref() {
                    Type::Path(p) => {
                        ctime_type_loc = Some(p.span().clone());

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

        let mut _ctime_err = quote! {};
        let mut use_ctime_error = false;
        if ret_type_t.is_none() || ret_type_e.is_none() {
            let span = ctime_type_loc.unwrap();
            use_ctime_error = true;
            _ctime_err = quote_spanned! {span=>
            compile_error!("Return type must be of the form RetryResult<T, E>. The retryable proc macro is unable to determine the underlying value and error types behind a type alias.");
        };
        }
        let ret_type_t: proc_macro2::TokenStream = ret_type_t.unwrap_or(quote! {()}.into()).into();
        let ret_type_e: proc_macro2::TokenStream = ret_type_e.unwrap_or(quote! {()}.into()).into();

        let body = &input_fn.block;

        RetryableParseData {
            struct_name: struct_name.clone(),
            inputs: inputs.clone(),
            ret_type_t: ret_type_t.clone(),
            ret_type_e: ret_type_e.clone(),
            output: output.clone(),
            original_body: *body.clone(),
            original_tokens: original_tokens.clone().into(),
            ctime_error: _ctime_err.clone(),
        }
    }

    pub(crate) fn expand_prepared(&self) -> proc_macro2::TokenStream {
        let inputs = &self.inputs;

        let struct_name = &self.struct_name;

        let ret_type_t = &self.ret_type_t;
        let ret_type_e = &self.ret_type_e;
        let output = &self.output;
        let body = &self.original_body;
        let original_tokens = &self.original_tokens;
        let _ctime_err = &self.ctime_error;
        let inner_fn_name = format_ident!("{}_inner", struct_name);

        let use_ctime_error = !_ctime_err.is_empty();

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
                use super::*;
                #[doc(hidden)]
                pub async fn #inner_fn_name(#inputs) -> #output #body
            }
        };

        if use_ctime_error {
            /*throw a compilation error but retain the original tokens so that it doesn't totally break lsp/intellisense etc*/
            quote! {
                #original_tokens
                #_ctime_err

            }
        } else {
            expanded
        }
    }

    pub(crate) fn expand_retry(&self) -> proc_macro2::TokenStream {
        let struct_name = &self.struct_name;
        let inputs = &self.inputs;
        let ret_type_t = &self.ret_type_t;
        let ret_type_e = &self.ret_type_e;
        let output = &self.output;
        let body = &self.original_body;
        let inner_fn_name = format_ident!("__inner__");

        let struct_fields = inputs.iter().filter_map(|arg| {
            if let FnArg::Typed(PatType { ty, .. }) = arg {
                Some(quote! { #ty })
            } else {
                None
            }
        });

        let param_names = (0..inputs.len()).map(|i| {
            let index = syn::Index::from(i);
            quote! { self.#index.clone() }
        });


        let arg_names = inputs.iter().filter_map(|arg| {
            if let FnArg::Typed(PatType { pat, .. }) = arg {
                if let Pat::Ident(ident) = &**pat {
                    Some(quote! { #ident })
                } else {
                    None
                }
            } else {
                None
            }
        });


        quote! {
            async fn #struct_name(#inputs) -> Result<#ret_type_t, #ret_type_e> {
                #[allow(non_camel_case_types)]
                struct __inner__struct(#(#struct_fields,)*);

                async fn #inner_fn_name(#inputs) -> RetryResult<#ret_type_t, #ret_type_e> #body

                #[async_trait]
                impl Executor<#ret_type_t, #ret_type_e> for __inner__struct {
                    async fn execute(&self) -> RetryResult<#ret_type_t, #ret_type_e> {
                        #inner_fn_name(#(#param_names),*).await
                    }
                }

                let ex = __inner__struct(#(#arg_names),*);
                ex.retry_with_default_policy().await
            }
        }
    }
}