# retry-rs


retry-rs is a utility to make retrying async functions easier

Providing access to the underlying retry executors and policies as well as simple-to-use proc macros
that allow the caller to specify the logic of a function without having to consider retry logic for each usage

Simply apply the #[retry] and #[retry-prepare] macros to any async function that returns Retryable<T,E>


### Examples

#### the default #[retry] macro 

This uses the default retry policy, which has an unlimited number of attempts
and a constant 1-second delay between attempts

```rust

 #[retry]
 async fn retryable_function(agent: DemoStructWithAsync) -> RetryResult<u32, u32> {
     let res = agent.execute_async().await;
     match res {
         Ok(val) => Success(val.get().unwrap() as u32),
         Err(val) => {
             let v = val.get().unwrap() as u32;
             if v == 0 {
                 Fatal(v)
             } else {
                 Retry(v)
             }
         },
     }
 }
 
 async fn retry_function() {
     let agent = get_async_demo_agent();
     let res = retryable_function(agent).await;
     println!("success: {}",res.is_ok())
 }

```
----



#### Using a custom retry policy

```rust

pub fn linear_backoff(policy: &RetryPolicy, attempt: usize) -> u64 {
    policy.base_delay * attempt as u64
}

fn retry_5_times() -> RetryPolicy {
    RetryPolicyBuilder::new_with_defaults()
        .limit(RetryLimit::Limited(5))
        .backoff_policy(linear_backoff)
        .base_delay(100)
        .build_with_defaults()
}

#[retry(retry_5_times)]
async fn agent_executor(agent: DemoStructWithAsync) -> RetryResult<u32, u32> {
    let mut guard = agent.lock().await;
    let res = guard.execute_async().await;
    match res {
        Ok(val) => Success(val.get().unwrap() as u32),
        Err(val) => Retry(val.get().unwrap() as u32),
    }
}

async fn retry_function() {
    let agent = get_async_demo_agent();
    let res = agent_executor(agent).await;
    println!("success: {}",res.is_ok())
}

```

----


#### Using #[retry-prepare] to prepare the function without executing it

This converts the original functions into a struct holding all the necessary information to execute the function

including its arguments and the retry policy to use.

Once the struct is created, specify the policy with the retry_with_policy and retry_with_default_policy methods and then
.await the result

```rust


#[retry_prepare]
async fn prepared_executor(agent: DemoStructWithAsync) -> RetryResult<u32, u32> {
    let mut guard = agent.lock().await;
    let res = guard.execute_async().await;
    match res {
        Ok(val) => Success(val.get().unwrap() as u32),
        Err(val) => {
            let v = val.get().unwrap() as u32;
            if v ==0 {
                Abort(v)
            } else {
                Retry(v)
            }
            
        },
    }
}

#[tokio::test]
async fn prepared_function() {
    let agent = get_async_demo_agent();
    let res = prepared_executor(agent).retry_with_default_policy().await;
    println!("success: ",res.is_ok())
}


```