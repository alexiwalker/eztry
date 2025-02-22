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

