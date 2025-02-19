use crate::executor::{AsyncFunction};
use crate::policy::RetryPolicy;
use crate::retry_result::RetryResult;
use crate::util;

pub struct Retryer<'a, T, E> {
    pub(crate) policy: util::OwnedOrRef<'a, RetryPolicy>,
    pub(crate) count: usize, /* not pub, meant to be internal only */
    pub(crate) function: AsyncFunction<'a, T, E>,
}

impl<T, E> Retryer<'_, T, E> {
    pub async fn run(mut self) -> Result<T, E> {
        let f = &self.function;
        let policy = self.policy.as_ref();
        self.count = 0;
        loop {
            self.count += 1;
            let r = f.execute().await;
            match r {
                RetryResult::Success(v) => return Ok(v),
                RetryResult::Abort(v) => return Err(v),
                RetryResult::Retry(e) => {

                    //eg limit(1)
                    //start at 0 outside loop -> 1 inside loop
                    // 1>=1 -> true -> retries exhausted -> return Err(e)

                    //as opposed to if it was c>l
                    //start at 0 outside loop -> 1 inside loop
                    // 1>1 -> false -> continue
                    // 2>1 -> true -> retries exhausted -> return Err(e) -> -> ran twice when limit was  1

                    if self.count >= policy.limit {
                        return Err(e);
                    }
                    policy.wait(self.count).await
                }
            }
        }
    }

    pub fn set_policy(&mut self, policy: RetryPolicy) {
        self.policy = util::OwnedOrRef::Owned(policy);
    }
    pub fn count(&self) -> usize {
        self.count
    }
}