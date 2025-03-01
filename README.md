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
async fn retryable_function(demo: DemoStructWithAsync) -> RetryResult<u32, u32> {
	let res = demo.execute_async().await;
	match res {
		Ok(val) => Success(val.get().unwrap() as u32),
		Err(val) => {
			let v = val.get().unwrap() as u32;
			if v == 0 {
				Abort(v)
			} else {
				Retry(v)
			}
		}
	}
}

async fn retry_function() {
	let demo = get_demo_struct();
	let res = retryable_function(demo).await;
	println!("success: {}", res.is_ok())
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
async fn agent_executor(demo: DemoStructWithAsync) -> RetryResult<u32, u32> {
	let res = demo.execute_async().await;
	match res {
		Ok(val) => Success(val.get().unwrap() as u32),
		Err(val) => Retry(val.get().unwrap() as u32),
	}
}

async fn retry_function() {
	let demo = get_demo_struct();
	let res = agent_executor(demo).await;
	println!("success: {}", res.is_ok())
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
async fn prepared_executor(demo: DemoStructWithAsync) -> RetryResult<u32, u32> {
	let res = demo.execute_async().await;
	match res {
		Ok(val) => Success(val.get().unwrap() as u32),
		Err(val) => {
			let v = val.get().unwrap() as u32;
			if v == 0 {
				Abort(v)
			} else {
				Retry(v)
			}
		}
	}
}

#[tokio::test]
async fn prepared_function() {
	let demo = get_demo_struct();
	let res = prepared_executor(demo).retry_with_default_policy().await;
	println!("success: ", res.is_ok())
}


```

---

#### Retrying an async closure

A trait exported by the prelude allows async closures to be retried directly without wrapping them with any other helpers.

Versions exist to retry with the default policy, or with a specified policy.



Default policy:


```rust


#[tokio::test]
async fn retry_directly_on_closure() {
    const DEFAULT_RETRIES: u64 = 50;

    /* SETUP DEFAULTS */
    let policy = RetryPolicy::builder()
        .limit(RetryLimit::Limited(DEFAULT_RETRIES))
        .backoff_policy(constant_backoff)
        .base_delay(1) //1 ms delay so it runs quickly
        .build_with_defaults();

    global::set_default_policy(policy);

    /* TEST WITH NEW DEFAULT */

    let agent = get_delayed_success_agent(DEFAULT_RETRIES);

    let res = (|| async {
        match agent.execute().await {
            Ok(_v) => Success(()),
            Err(_e) => Retry(()),
        }
    }).retry_with_default_policy().await;

    let count = agent.count().await;
    assert!(&res.is_ok());
    assert_eq!(count, DEFAULT_RETRIES);

}
    

```


Custom policy:

```rust

#[tokio::test]
async fn retry_directly_on_closure_with_policy() {
    const DEFAULT_RETRIES: u64 = 4;

    /* SETUP DEFAULTS */
    let policy = RetryPolicy::builder()
        .limit(RetryLimit::Limited(DEFAULT_RETRIES))
        .backoff_policy(constant_backoff)
        .base_delay(1) //1 ms delay so it runs quickly
        .build_with_defaults();

    let agent = get_delayed_success_agent(DEFAULT_RETRIES);

    let res = (|| async {
        match agent.execute().await {
            Ok(_v) => Success(()),
            Err(_e) => Retry(()),
        }
    }).retry(&policy).await;

    let count = agent.count().await;
    assert!(&res.is_ok());
    assert_eq!(count, DEFAULT_RETRIES);

}
    
```

---



#### Changing the global default policy

The global default policy defaults to the following values:

```rust

const GLOBAL_DEFAULT_POLICY: RetryPolicy = RetryPolicy {
    limit: RetryLimit::Unlimited,
    base_delay: 1000,
    delay_time: constant_backoff,
};

```

This is the policy that will be used if no other policy is specified. 


The following methods exist in the exported global module:


```rust

pub mod global {

	pub fn set_default_policy(policy: policy::RetryPolicy) { ... }

	pub fn get_default_policy() -> &'static RetryPolicy { ... }

	pub fn reset_default_policy() { ... }    
    
}

```

set_default_policy takes in a retry policy and sets it as the global default policy. WARNING: this leaks the policy into 'static, so ensure it is called only once.

get_default_policy returns a reference to the global default policy - either the GLOBAL_DEFAULT_POLICY or the one set by set_default_policy

reset_default_policy resets the global default policy to the default values specified above.


Any methods that do not take a policy reference to retry, or are named with '..._with_default_policy' will use the global default policy. All other methods will require a policy to be provided