pub use async_trait::async_trait as __async_trait_reexport;
pub use executor::Executor;
pub use policy::RetryPolicy;
pub use retry_result::RetryResult;

#[cfg(feature = "macros")]
extern crate retry_rs_macros;

#[cfg(feature = "macros")]
pub use retry_rs_macros::*;

pub mod retry_result;
pub mod policy;
pub mod executor;
pub mod retryer;


pub mod prelude {
    pub use crate::retry_result::{RetryResult,RetryResult::Retry,RetryResult::Abort,RetryResult::Success};
    pub use crate::{success, abort, retry};
    pub use crate::policy::{RetryPolicy, RetryLimit, RetryPolicyBuilder, RetryPolicyBuilderError};
    pub use crate::executor::{Executor,AsyncFunction,self};
    pub use crate::retryer::BoxRetryer;
    pub use crate::backoff;

    #[cfg(feature = "macros")]
    pub use retry_rs_macros::*;

    #[cfg(feature = "macros")]
    pub use crate::__async_trait_reexport;
}


pub(crate) mod util {
    use crate::policy::RetryPolicy;

    pub(crate) enum OwnedOrRef<'a, T> {
    Owned(T),
    Ref(&'a T),
}

    impl OwnedOrRef<'_, RetryPolicy> {
        pub fn as_ref(&self) -> &RetryPolicy {
            match self {
                OwnedOrRef::Owned(p) => p,
                OwnedOrRef::Ref(p) => p,
            }
        }
    }

}


pub type BackoffPolicy = fn(&RetryPolicy, usize) -> u64;

pub mod backoff {
    use crate::policy::RetryPolicy;
    pub fn exponential_backoff(policy: &RetryPolicy, attempt: usize) -> u64 {
        let multiplier = 2u64.pow(attempt as u32 - 1);
        policy.base_delay * multiplier
    }

    pub fn linear_backoff(policy: &RetryPolicy, attempt: usize) -> u64 {
        policy.base_delay * attempt as u64
    }

    pub fn constant_backoff(policy: &RetryPolicy, _attempt: usize) -> u64 {
        policy.base_delay
    }

}

#[inline(always)]
pub fn success<T, E>(v: T) -> RetryResult<T, E> {
    RetryResult::Success(v)
}
#[inline(always)]
pub fn retry<T, E>(e: E) -> RetryResult<T, E> {
    RetryResult::Retry(e)
}
#[inline(always)]
pub fn abort<T, E>(e: E) -> RetryResult<T, E> {
    RetryResult::Abort(e)
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use super::*;
    use rand::Rng;
    use executor::Executor;
    use crate::policy::default_next_delay;

    fn generate_random_number() -> u8 {
        let mut rng = rand::rng();
        rng.random_range(1..=100)
    }

    #[tokio::test]
    async fn api_testing() {
        struct TestExecutor;

        #[async_trait]
        impl Executor<usize, String> for TestExecutor {
            async fn execute(&self) -> RetryResult<usize, String> {
                success(1)
            }
        }

        let mut ex = TestExecutor.default_retry_policy();

        let p = RetryPolicy {
            limit: Default::default(),
            base_delay: 500,
            delay_time: default_next_delay,
        };

        ex.set_policy(p);

        let r = ex.run().await;

        let _ = dbg!(r);
    }


    #[tokio::test]
    async fn rng_testing() {
        struct PreparedExampleFunction(u32, String);

        #[async_trait]
        impl Executor<String, String> for PreparedExampleFunction {
            async fn execute(&self) -> RetryResult<String, String> {
                let (arg0, arg1) = (self.0, self.1.clone());
                example_function(arg0, arg1).await
            }
        }

        async fn example_function(v: u32, s: String) -> RetryResult<String, String> {
            let rng = generate_random_number();
            if rng == 100 {
                let data_1 = v;
                let data_2 = s;
                let s = format!("{data_1}_{data_2}");
                success(s)
            } else if rng < 5 {
                abort("simulated error".to_string())
            } else {
                retry("intermittent error".to_string())
            }
        }

        let func = PreparedExampleFunction(1u32, "something".to_string());
        let mut ex = func.default_retry_policy();

        let p = RetryPolicy {
            limit: Default::default(),
            base_delay: 500,
            delay_time: default_next_delay,
        };

        ex.set_policy(p);

        let r = ex.run().await;

        match r {
            Ok(v) => {
                println!("Success: {}", v);
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
}
