use std::cmp;
use std::cmp::Ordering;
use crate::RetryLimit::{Limited, Unlimited};
use async_trait::async_trait;

const DEFAULT_POLICY: RetryPolicy = RetryPolicy {
    limit: RetryLimit::Unlimited,
    base_delay: 1000,
    delay_time: default_next_delay,
};
#[async_trait]
pub trait Executor<T, E>: Send + Sync {
    async fn execute(&self) -> RetryResult<T, E>;

    fn default_retry_policy(self) -> Retryer<T, E>
    where
        Self: Sized + 'static,
    {
        Retryer {
            policy: DEFAULT_POLICY,
            count: 0,
            function: Box::new(self),
        }
    }

    async fn retry_with_policy(self, policy: RetryPolicy) -> Result<T, E>
    where
        Self: Sized + 'static,
        T: Send + Sync,
        E: Send + Sync,
    {
        Retryer {
            policy,
            count: 0,
            function: Box::new(self),
        }.run().await
    }

    async fn retry_with_default_policy(self) -> Result<T, E>
    where
        Self: Sized + 'static,
        T: Send + Sync,
        E: Send + Sync,
    {
        Retryer {
            policy:DEFAULT_POLICY,
            count: 0,
            function: Box::new(self),
        }.run().await
    }

    async fn call(self) -> RetryResult<T, E>
    where
        Self: Sized + 'static,
    {
        self.execute().await
    }

    fn use_policy(self, policy: RetryPolicy) -> Retryer<T, E>
    where
        Self: Sized + 'static,
    {
        Retryer {
            policy,
            count: 0,
            function: Box::new(self),
        }
    }
}

pub type AsyncFunction<T, E> = Box<dyn Executor<T, E>>;

#[derive(Debug)]
pub enum RetryResult<T, E> {
    Success(T),
    Retryable(E), /* Propagated only if all retries exhausted*/
    Abort(E),
}

impl<T, E> Into<Result<T, E>> for RetryResult<T, E> {
    fn into(self) -> Result<T, E> {
        match self {
            RetryResult::Success(t) => Ok(t),
            RetryResult::Abort(e) | RetryResult::Retryable(e) => Err(e),
        }
    }
}

#[derive(Default, Debug)]
pub enum RetryLimit {
    #[default]
    Unlimited,
    Limited(usize),
}

impl cmp::PartialEq for RetryLimit {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Unlimited, Unlimited) => true,
            (Limited(a), Limited(b)) => a == b,
            _ => false,
        }
    }
}

impl PartialEq<usize> for RetryLimit {
    fn eq(&self, other: &usize) -> bool {
        match self {
            Unlimited => false,
            Limited(a) => a == other,
        }
    }
}

//allow somecount > retrylimit comparison without having to match on the enum
impl PartialOrd<usize> for RetryLimit {
    fn partial_cmp(&self, count: &usize) -> Option<Ordering> {
        match self {
            Unlimited => Some(Ordering::Less),
            Limited(lim) => {
                /* more explicit than using the cmp traits directly because its representing the logic of limits better*/
                if count < lim {
                    Some(Ordering::Less)
                } else if count == lim {
                    Some(Ordering::Equal)
                } else {
                    Some(Ordering::Greater)
                }
            }
        }
    }
}
impl PartialEq<RetryLimit> for usize {
    fn eq(&self, other: &RetryLimit) -> bool {
        other.eq(self)
    }
}

impl PartialOrd<RetryLimit> for usize {
    fn partial_cmp(&self, other: &RetryLimit) -> Option<Ordering> {
        other.partial_cmp(self)
    }
}


pub struct RetryPolicy {
    pub limit: RetryLimit,
    pub base_delay: u64,
    pub delay_time: fn(&RetryPolicy, usize) -> u64,
}

impl RetryPolicy {
    pub async fn wait(&self, count: usize) -> () {
        let t = (self.delay_time)(self, count);
        let t = std::time::Duration::from_millis(t);
        tokio::time::sleep(t).await;
    }
}

fn default_next_delay(policy: &RetryPolicy, _count: usize) -> u64 {
    policy.base_delay
}

pub struct Retryer<T, E> {
    pub policy: RetryPolicy,
    count: usize, /* not pub, meant to be internal only */
    pub function: AsyncFunction<T, E>,
}



impl<T, E> Retryer<T, E> {
    pub async fn run(mut self) -> Result<T, E> {
        let f = &self.function;
        let policy = &self.policy;
        self.count = 0;
        loop {
            self.count += 1;
            let r = f.execute().await;
            match r {
                RetryResult::Success(v) => return Ok(v),
                RetryResult::Abort(v) => return Err(v),
                RetryResult::Retryable(e) => {
                    if self.count > policy.limit {
                        println!("Retry limit reached, {} {:?}", self.count, policy.limit);
                        return Err(e);
                    }
                    policy.wait(self.count).await
                }
            }
        }
    }

    pub fn set_policy(&mut self, policy: RetryPolicy) {
        self.policy = policy;
    }
    pub fn count(&self) -> usize {
        self.count
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        RetryPolicy {
            limit: Default::default(),
            base_delay: 1000,
            delay_time: default_next_delay,
        }
    }
}

pub type BackoffPolicy = fn(&RetryPolicy, usize) -> u64;

#[derive(Default, Debug)]
pub struct RetryPolicyBuilder {
    limit: Option<RetryLimit>,
    base_delay: Option<u64>,
    backoff_policy: Option<BackoffPolicy>,
}

pub mod backoff_policy {
    pub fn exponential_backoff(policy: &crate::RetryPolicy, attempt: usize) -> u64 {
        let multiplier = 2u64.pow(attempt as u32 - 1);
        let delay = policy.base_delay * multiplier;
        delay
    }

    pub fn linear_backoff(policy: &crate::RetryPolicy, attempt: usize) -> u64 {
        let delay = policy.base_delay * attempt as u64;
        delay
    }

    pub fn constant_backoff(policy: &crate::RetryPolicy, _attempt: usize) -> u64 {
        policy.base_delay
    }
}

impl RetryPolicyBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn new_with_defaults() -> Self {
        Self {
            limit: Some(RetryLimit::Unlimited),
            base_delay: Some(1000),
            backoff_policy: Some(default_next_delay),
        }
    }

    pub fn limit(mut self, limit: RetryLimit) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn base_delay(mut self, base_delay: u64) -> Self {
        self.base_delay = Some(base_delay);
        self
    }

    pub fn backoff_policy(mut self, backoff_policy: fn(&RetryPolicy, usize) -> u64) -> Self {
        self.backoff_policy = Some(backoff_policy);
        self
    }

    pub fn build(self) -> RetryPolicy {
        RetryPolicy {
            limit: self.limit.unwrap(),
            base_delay: self.base_delay.unwrap(),
            delay_time: self.backoff_policy.unwrap(),
        }
    }
    pub fn build_with_defaults(self) -> RetryPolicy {
        RetryPolicy {
            limit: self.limit.unwrap_or(Unlimited),
            base_delay: self.base_delay.unwrap_or(1000),
            delay_time: self.backoff_policy.unwrap_or(default_next_delay),
        }
    }
    pub fn try_build(self) -> Result<RetryPolicy, RetryPolicyBuilderError> {
        let mut error = RetryPolicyBuilderError {
            missing_base_delay: false,
            missing_backoff_policy: false,
            missing_limit: false,
        };

        let mut missing_any = false;
        if self.limit.is_none() {
            error.missing_limit = true;
            missing_any = true;
        }
        if self.base_delay.is_none() {
            error.missing_base_delay = true;
            missing_any = true;
        }
        if self.backoff_policy.is_none() {
            error.missing_backoff_policy = true;
            missing_any = true;
        }

        if missing_any {
            return Err(error);
        }

        Ok(RetryPolicy {
            limit: self.limit.unwrap(),
            base_delay: self.base_delay.unwrap(),
            delay_time: self.backoff_policy.unwrap(),
        })
    }
}

pub struct RetryPolicyBuilderError {
    missing_limit: bool,
    missing_base_delay: bool,
    missing_backoff_policy: bool,
}

pub fn success<T, E>(v: T) -> RetryResult<T, E> {
    RetryResult::Success(v)
}
pub fn retry<T, E>(e: E) -> RetryResult<T, E> {
    RetryResult::Retryable(e)
}
pub fn abort<T, E>(e: E) -> RetryResult<T, E> {
    RetryResult::Abort(e)
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::RetryResult::{Abort, Retryable, Success};
//     use rand::Rng;
//
//     fn generate_random_number() -> u8 {
//         let mut rng = rand::rng();
//         rng.random_range(1..=100)
//     }
//
//     #[tokio::test]
//     async fn api_testing() {
//         struct TestExecutor;
//
//         #[async_trait]
//         impl Executor<usize, String> for TestExecutor {
//             async fn execute(&self) -> RetryResult<usize, String> {
//                 success(1)
//             }
//         }
//
//         let mut ex = TestExecutor.default_retry_policy();
//
//         let p = RetryPolicy {
//             limit: Default::default(),
//             base_delay: 500,
//             delay_time: default_next_delay,
//         };
//
//         ex.set_policy(p);
//
//         let r = ex.run().await;
//
//         dbg!(r);
//     }
//
//     #[tokio::test]
//     async fn rng_testing() {
//         struct PreparedExampleFunction(u32, String);
//
//         #[async_trait]
//         impl Executor<String, String> for PreparedExampleFunction {
//             async fn execute(&self) -> RetryResult<String, String> {
//                 let (_0, _1) = (self.0.clone(), self.1.clone());
//                 example_function(_0, _1).await
//             }
//         }
//
//         async fn example_function(v: u32, s: String) -> RetryResult<String, String> {
//             let mut rng = generate_random_number();
//             println!("RNG: {}", rng);
//             if rng == 100 {
//                 let data_1 = v;
//                 let data_2 = s;
//                 let s = format!("{data_1}_{data_2}");
//                 let _ = tokio::fs::write("./tmp_file.txt", &s).await;
//                 success(s)
//             } else if rng < 5 {
//                 abort("simulated error".to_string())
//             } else {
//                 retry("intermittent error".to_string())
//             }
//         }
//
//         let mut ex = PreparedExampleFunction(1u32, "something".to_string()).default_retry_policy();
//
//         let p = RetryPolicy {
//             limit: Default::default(),
//             base_delay: 500,
//             delay_time: default_next_delay,
//         };
//
//         ex.set_policy(p);
//
//         let r = ex.run().await;
//
//         dbg!(r);
//     }
// }
