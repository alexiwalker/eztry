use function_info::FunctionInfo;
use proc_macro::TokenStream;
use proc_macro2::Ident;
use syn::{parse_macro_input, ItemFn};

mod function_info;
mod parser;

/// converts a function to a struct containing the original function, and associated retry policies
/// useful if you want to pass the arguments to a function and define the policy procedurally then execute it elsewhere
///
/// The return type of the original function must be of the form ```RetryResult<T, E>```,
/// and the return type of the new function will be ```Result<T, E>``` where T and E are the types of the original function.
///
/// the Retryable and Abort results are of type E, and the Success result is of type T
///
/// if the function returns Retryable results but fails >= the number of retries on the policy, the function will return the last E as a Result<_, E>
///
/// if the function returns any Abort results, the function will return the first Abort result as a Result<_, E>
///
/// The function will return the first Success result as a Result<T, _>
///
/// Example:
/// ```ignore
///
/// #[retry_prepare]
/// async fn prepared_executor(agent: DemoStructWithAsync) -> RetryResult<u32, u32> {
///     let res = agent.execute_async().await;
///     match res {
///         Ok(val) => Success(val.get().unwrap() as u32),
///         Err(val) => {
///             let v = val.get().unwrap() as u32;
///             if v == 0 {
///                 Abort(v)
///             } else {
///                 Retry(v)
///             }
///         },
///     }
/// }
///
/// async fn prepared_function() {
///     let agent = get_async_demo_agent();
///     let res = prepared_executor(agent).retry_with_default_policy().await;
///     assert!(res.is_ok())
/// }
///     
///```
///
#[proc_macro_attribute]
pub fn retry_prepare(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let original_tokens: proc_macro2::TokenStream = item.clone().into();
    let input_fn = parse_macro_input!(item as ItemFn);
    let retryable_data = FunctionInfo::from_function(input_fn, original_tokens);
    let expanded = retryable_data.expand_prepared();
    TokenStream::from(expanded)
}

/// Builds a retryable function with the given policy (or the default policy if none is provided).
/// The function build will use the same name and arguments as the original function, and can be ```.await```ed directly
///
/// The return type of the original function must be of the form ```RetryResult<T, E>```,
/// and the return type of the new function will be ```Result<T, E>``` where T and E are the types of the original function.
///
///
///
///
/// the Retryable and Abort results are of type E, and the Success result is of type T
///
/// if the function returns Retryable results but fails >= the number of retries on the policy, the function will return the last E as a Result<_, E>
///
/// if the function returns any Abort results, the function will return the first Abort result as a Result<_, E>
///
/// The function will return the first Success result as a Result<T, _>
///
/// Example:
/// ```ignore
///
/// #[retry]
/// async fn retryable_function(agent: DemoStructWithAsync) -> RetryResult<u32, u32> {
///     let res = agent.execute_async().await;
///     match res {
///         Ok(val) => Success(val.get().unwrap() as u32),
///         Err(val) => {
///             let v = val.get().unwrap() as u32;
///             if v == 0 {
///                 Abort(v)
///             } else {
///                 Retry(v)
///             }
///         },
///     }
/// }
///
/// async fn retry_function() {
///     let agent = get_async_demo_agent();
///     let res = retryable_function(agent).await;
///     assert!(res.is_ok())
/// }
///     
#[proc_macro_attribute]
pub fn retry(attr: TokenStream, item: TokenStream) -> TokenStream {
    let policy_fn = if attr.is_empty() {
        None
    } else {
        Some(parse_macro_input!(attr as Ident))
    };

    let original_tokens: proc_macro2::TokenStream = item.clone().into();
    let input_fn = parse_macro_input!(item as ItemFn);

    let retryable_data = FunctionInfo::from_function(input_fn, original_tokens);
    let expanded = retryable_data.expand_retry(policy_fn);

    TokenStream::from(expanded)
}
