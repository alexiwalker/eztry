pub use async_trait::async_trait;
#[async_trait]
pub trait Executor<T, E> : Send + Sync {
    async fn execute(&self) -> Retryable<T, E>;

    fn default_retry_policy(self) -> Retryer<T, E> where Self: Sized + 'static
    {
        Retryer {
            policy: Default::default(),
            count: 0,
            function: Box::new(self)
        }
    }

    async fn call(self) -> Retryable<T, E> where Self: Sized + 'static
    {
        self.execute().await
    }

    fn use_policy(self, policy: RetryPolicy) -> Retryer<T, E> where Self: Sized + 'static
    {
        Retryer {
            policy,
            count: 0,
            function: Box::new(self)
        }
    }
}

pub type AsyncFunction<T, E> = Box<dyn Executor<T, E>>;


#[derive(Debug)]
pub enum Retryable<T, E> {
    Success(T),
    Retry,
    Abort(E),
}

pub enum RetryLimit {
    Unlimited,
    Limited(usize),
}

impl Default for RetryLimit {
    fn default() -> Self {
        RetryLimit::Unlimited
    }
}

pub struct RetryPolicy {
    limit: RetryLimit,
    base_delay: u64,
    next_delay: fn (&RetryPolicy, count:usize)->u64
}

impl RetryPolicy {
    pub async fn wait(&self, count:usize) -> () {
        let t = (self.next_delay)(self,count);
        let t = std::time::Duration::from_millis(t);
        tokio::time::sleep(t).await;
    }
}

fn default_next_delay(policy: &RetryPolicy, _count:usize) -> u64 {
    policy.base_delay
}

pub struct Retryer<T, E> {
    pub policy: RetryPolicy,
    count: usize, /* not pub, meant to be internal only */
    pub function: AsyncFunction<T, E>,
}

impl<T, E> Retryer<T, E> {
    pub async fn run(&mut self)-> Result<T,E> {

        let f = &self.function;
        let policy = &self.policy;
        self.count = 0;
        loop {
            self.count += 1;

            let r = f.execute().await;

            match r {
                Retryable::Success(v) => {
                    return Ok(v)
                }

                Retryable::Abort(v) => {
                    return Err(v)
                }
                Retryable::Retry => {
                    policy.wait(self.count).await
                }
            }


        }


    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        RetryPolicy {
            limit: Default::default(),
            base_delay: 1000,
            next_delay: default_next_delay,
        }
    }
}

pub fn success<T, E>(v: T) -> Retryable<T, E> {
    Retryable::Success(v)
}
pub fn retry<T, E>() -> Retryable<T, E> {
    Retryable::Retry
}
pub fn abort<T, E>(e: E) -> Retryable<T, E> {
    Retryable::Abort(e)
}

impl<T, E> Retryer<T, E> {
    pub fn set_policy(&mut self, policy: RetryPolicy) {
        self.policy = policy;
    }
    pub fn count(&self) -> usize {
        self.count
    }
}

#[cfg(test)]
mod tests {
    use rand::Rng;
    use crate::Retryable::{Abort, Retry, Success};
    use super::*;


    fn generate_random_number() -> u8 {
        let mut rng = rand::rng();
        rng.random_range(1..=100)
    }


    #[tokio::test]
    async fn api_testing() {
        struct TestExecutor;

        #[async_trait]
        impl Executor<usize, String> for TestExecutor {
            async fn execute(&self) -> Retryable<usize, String> {
                success(1)
            }
        }

        let mut ex = TestExecutor.default_retry_policy();

        let p = RetryPolicy {
            limit: Default::default(),
            base_delay: 500,
            next_delay: default_next_delay,
        };

        ex.set_policy(p);


        let r = ex.run().await;

        dbg!(r);


    }


    #[tokio::test]
    async fn rng_testing() {
        struct PreparedExampleFunction(u32, String);

        #[async_trait]
        impl Executor<String, String> for PreparedExampleFunction {
            async fn execute(&self) -> Retryable<String, String> {
                let (_0,_1) = (self.0.clone(),self.1.clone());
                example_function(_0,_1).await
            }
        }

        async fn example_function(v:u32, s:String) -> Retryable<String,String> {
            let mut rng = generate_random_number();
            println!("RNG: {}", rng);
            if rng == 100 {
                let data_1 = v;
                let data_2 =  s;
                let s = format!("{data_1}_{data_2}");
                let _ = tokio::fs::write("./tmp_file.txt", &s).await;
                success(s)
            } else if rng < 5 {
                abort("simulated error".to_string())
            } else {
                retry()
            }
        }



        let mut ex = PreparedExampleFunction(1u32,"something".to_string()).default_retry_policy();

        let p = RetryPolicy {
            limit: Default::default(),
            base_delay: 500,
            next_delay: default_next_delay,
        };

        ex.set_policy(p);


        let r = ex.run().await;

        dbg!(r);


    }
}
