use crate::parser;
use proc_macro2::Ident;
use quote::{format_ident, quote, quote_spanned};
use std::collections::HashSet;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Comma;
use syn::{
    parse_quote, FnArg, ItemFn, Pat, PatType, Type,
    TypeReference,
};

pub struct FunctionInfo {
    pub(crate) struct_name: Ident,
    pub(crate) inputs: Punctuated<FnArg, Comma>,
    pub(crate) ret_type_t: proc_macro2::TokenStream,
    pub(crate) ret_type_e: proc_macro2::TokenStream,
    pub(crate) output: proc_macro2::TokenStream,
    pub(crate) original_body: syn::Block,
    pub(crate) original_tokens: proc_macro2::TokenStream,
    pub(crate) ctime_error: proc_macro2::TokenStream,
}

impl FunctionInfo {
    pub(crate) fn from_function(
        input_fn: ItemFn,
        original_tokens: proc_macro2::TokenStream,
    ) -> Self {
        parser::parse_function(&input_fn, original_tokens)
    }

    fn extract_lifetimes_with_defaults(
        inputs: &Punctuated<FnArg, Comma>,
    ) -> (proc_macro2::TokenStream, Option<proc_macro2::TokenStream>) {
        let mut lifetimes = HashSet::new();
        let mut needs_default_lifetime = false;
        let mut updated_types: Vec<Box<Type>> = Vec::new();

        for arg in inputs {
            if let FnArg::Typed(PatType { ty, .. }) = arg {
                match ty.as_ref() {
                    Type::Reference(TypeReference {
                        lifetime,
                        elem,
                        mutability,
                        ..
                    }) => {
                        if let Some(lifetime) = lifetime {
                            lifetimes.insert(lifetime.clone());
                            updated_types.push(ty.clone()); // Keep the existing lifetime
                        } else {
                            needs_default_lifetime = true;
                            let new_ty = Type::Reference(TypeReference {
                                lifetime: Some(parse_quote! { 'args }),
                                elem: elem.clone(),
                                mutability: *mutability,
                                and_token: Default::default(),
                            });
                            updated_types.push(Box::new(new_ty));
                        }
                    }
                    _ => updated_types.push(ty.clone()),
                }
            }
        }

        let lifetime_tokens = if !lifetimes.is_empty() {
            let lifetimes_vec: Vec<_> = lifetimes.into_iter().collect();
            quote! { <#(#lifetimes_vec),*> }
        } else if needs_default_lifetime {
            quote! { <'args> }
        } else {
            quote! {}
        };

        let updated_types_tokens = if needs_default_lifetime {
            Some(quote! { #(#updated_types),* })
        } else {
            None
        };

        (lifetime_tokens, updated_types_tokens)
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

        let mut struct_fields = Self::get_arg_types(inputs);
        let param_names = Self::get_struct_field_names(inputs);

        let has_self = Self::is_self(inputs);

        if has_self {
            let err_no_prepare_for_self = quote_spanned! {inputs.span()=>
                compile_error!("Cannot use #[retry_prepare] on a function that takes self as an argument. Use #[retry] instead.");
            };

            return quote! {
                #err_no_prepare_for_self
                #original_tokens
            };
        }

        let (lifetimes, revised_fields) = Self::extract_lifetimes_with_defaults(inputs);

        if revised_fields.is_some() {
            struct_fields = revised_fields.unwrap()
        }

        let anon_lifetime = if !lifetimes.is_empty() {
            quote! { <'_> }
        } else {
            quote! {}
        };

        let inner_mod_name = format_ident!("__{struct_name}_MOD_INTERNAL");

        let expanded = quote! {
            #[allow(non_camel_case_types)]
            struct #struct_name #lifetimes (#struct_fields);
            impl retry_rs::prelude::Executor<#ret_type_t, #ret_type_e> for #struct_name #anon_lifetime {
                #[allow(
                    elided_named_lifetimes,
                    clippy::async_yields_async,
                    clippy::diverging_sub_expression,
                    clippy::let_unit_value,
                    clippy::needless_arbitrary_self_type,
                    clippy::no_effect_underscore_binding,
                    clippy::shadow_same,
                    clippy::type_complexity,
                    clippy::type_repetition_in_bounds,
                    clippy::used_underscore_binding
                )]
                fn execute<'life0, 'async_trait>(
                    &'life0 self,
                ) -> ::core::pin::Pin<Box<dyn ::core::future::Future<Output = #output,> + ::core::marker::Send + 'async_trait,>,>
                where
                    'life0: 'async_trait,
                    Self: 'async_trait,
                {
                    Box::pin(async move {
                        if let ::core::option::Option::Some(__ret) = ::core::option::Option::None::<
                            #output,
                        > {
                            #[allow(unreachable_code)] return __ret;
                        }
                        let __self = self;
                        let __ret: #output = {
                            #inner_mod_name::#inner_fn_name(#param_names)
                                .await
                        };
                        #[allow(unreachable_code)] __ret
                    })
                }
            }

            #[doc(hidden)]
            mod #inner_mod_name {
                use super::*;
                #[doc(hidden)]
                #[inline(always)]
                pub async fn #inner_fn_name #lifetimes (#inputs) -> #output #body
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

    pub(crate) fn expand_retry(&self, policy_fn: Option<Ident>) -> proc_macro2::TokenStream {
        let fn_name = &self.struct_name;
        let inputs = &self.inputs;
        let ret_type_t = &self.ret_type_t;
        let ret_type_e = &self.ret_type_e;
        let output = quote! { Result<#ret_type_t, #ret_type_e> };
        let body = &self.original_body;

        let struct_fields = Self::get_arg_types(inputs);
        let param_names = Self::get_struct_field_names(inputs);
        let arg_names = Self::get_arg_names(inputs);
        let is_self = Self::is_self(inputs);
        let without_receiver = Self::args_without_receiver(inputs);
        let policy_call = Self::get_policy_call(&policy_fn);

        /*

        Please, do not try to modify the macro below directly
        most of it is generated by using cargo expand on the macro for async_trait in a demo crate
        to get the correct lifetime desugaring so my macro works without the user needing to
        add async_trait to their dependencies

        */
        let _ctime_err = &self.ctime_error;
        let original_tokens = &self.original_tokens;
        if !self.ctime_error.is_empty() {
            quote! {
                #original_tokens
                #_ctime_err
            }
        } else if is_self {
            let policy = if policy_fn.is_none() {
                quote! { RetryPolicy::default() }
            } else {
                quote! { #policy_fn() }
            };

            let formatted_inner_fn_name = format_ident!("{fn_name}__inner__");

            quote! {
                    async fn #formatted_inner_fn_name(#inputs) -> RetryResult<#ret_type_t, #ret_type_e>
                       #body

                   async fn #fn_name(#inputs) -> Result<#ret_type_t, #ret_type_e> {
                       let policy = #policy; /*default if not supplied in macro, otherwise use f()*/
                       let mut i = 0;
                       loop {
                           i+=1;
                           let r = self.#formatted_inner_fn_name(#without_receiver).await;

                           match r {
                               retry_rs::RetryResult::Success(s) => {
                                   return Ok(s);
                               }
                               retry_rs::RetryResult::Retry(e) => {
                                   if !policy.can_retry(i) {
                                       return Err(e)
                                   } else {
                                       policy.wait(i).await
                                   }
                               }
                               retry_rs::RetryResult::Abort(e) => {
                                   return Err(e)
                               }
                           }
                       }
                   }
            }
        } else {
            quote! {
               async fn #fn_name(#inputs) -> #output {
                    #[allow(non_camel_case_types)]
                    struct __inner__struct(#struct_fields);
                    async fn  __inner__(#inputs) -> RetryResult<#ret_type_t, #ret_type_e> #body
                    impl retry_rs::prelude::Executor<#ret_type_t, #ret_type_e> for __inner__struct {
                        #[allow(
                            elided_named_lifetimes,
                            clippy::async_yields_async,
                            clippy::diverging_sub_expression,
                            clippy::let_unit_value,
                            clippy::needless_arbitrary_self_type,
                            clippy::no_effect_underscore_binding,
                            clippy::shadow_same,
                            clippy::type_complexity,
                            clippy::type_repetition_in_bounds,
                            clippy::used_underscore_binding
                        )]
                        fn execute<'life0, 'async_trait>(
                            &'life0 self,
                        ) -> ::core::pin::Pin<
                            Box<
                                dyn ::core::future::Future<
                                    Output = RetryResult<#ret_type_t, #ret_type_e>,
                                > + ::core::marker::Send + 'async_trait,
                            >,
                        >
                        where
                            'life0: 'async_trait,
                            Self: 'async_trait,
                        {
                            Box::pin(async move {
                                if let ::core::option::Option::Some(__ret) = ::core::option::Option::None::<
                                    RetryResult<#ret_type_t, #ret_type_e>,
                                > {
                                    #[allow(unreachable_code)] return __ret;
                                }
                                let __self = self;
                                let __ret: RetryResult<#ret_type_t, #ret_type_e> = {
                                    __inner__(
                                            #param_names
                                        )
                                        .await
                                };
                                #[allow(unreachable_code)] __ret
                            })
                        }
                    }
                    let ex = __inner__struct(#arg_names);
                    #policy_call
                }
            }
        }
    }

    fn get_policy_call(policy_fn: &Option<Ident>) -> proc_macro2::TokenStream {
        if let Some(policy_fn) = policy_fn {
            quote! { ex.retry_with_policy(#policy_fn()).await }
        } else {
            quote! { ex.retry_with_default_policy().await }
        }
    }

    fn get_arg_names(inputs: &Punctuated<FnArg, Comma>) -> proc_macro2::TokenStream {
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

        quote! { #(#arg_names),* }
    }

    fn is_self(inputs: &Punctuated<FnArg, Comma>) -> bool {
        let first_input = inputs.first();
        match first_input {
            None => false,
            Some(input) => matches!(input, FnArg::Receiver(_)),
        }
    }

    fn get_struct_field_names(inputs: &Punctuated<FnArg, Comma>) -> proc_macro2::TokenStream {
        let first_input = inputs.first();
        let mut skip_first = false;
        match first_input {
            None => {}
            Some(input) => match input {
                FnArg::Receiver(_) => {
                    skip_first = true;
                }
                FnArg::Typed(PatType { .. }) => {}
            },
        }

        let param_names = (0..inputs.len()).filter_map(|i| {
            if skip_first && i == 0 {
                return None;
            }

            let index = syn::Index::from(i);
            Some(quote! { self.#index.clone() })
        });

        quote! {#(#param_names),*}
    }

    fn get_arg_types(inputs: &Punctuated<FnArg, Comma>) -> proc_macro2::TokenStream {
        let types = inputs.iter().filter_map(|arg| {
            if let FnArg::Typed(PatType { ty, .. }) = arg {
                Some(quote! { #ty })
            } else {
                None
            }
        });

        quote! { #(#types,)* }
    }

    fn args_without_receiver(
        inputs: &Punctuated<FnArg, Comma>,
    ) -> Option<proc_macro2::TokenStream> {
        let first_input = inputs.first();
        let is_self = matches!(first_input, Some(FnArg::Receiver(_)));

        if !is_self {
            return None;
        }

        let args = inputs.iter().skip(1).map(|arg| {
            if let FnArg::Typed(PatType { pat, ty, .. }) = arg {
                match **ty {
                    Type::Reference(_) => quote! { &#pat },
                    _ => quote! { #pat.clone() },
                }
            } else {
                quote! {}
            }
        });

        Some(quote! { #(#args),* })
    }
}
