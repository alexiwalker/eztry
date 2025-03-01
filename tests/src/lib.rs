#[cfg(test)]
mod agent;
#[cfg(test)]
mod tests {
    use crate::agent::*;
    use rand::Rng;
    use retry_rs::global;
    use retry_rs::prelude::*;

    type DemoStructWithAsync = MutableAgent;
    /*the function here should always pass, it's to make sure that what I am passing can be passed to async functions*/
    async fn takes_an_agent(agent: DemoStructWithAsync) -> FallibleResult {
        let mut guard = agent.lock().await;
        guard.execute_async().await
    }

    fn retry_5_times() -> RetryPolicy {
        RetryPolicyBuilder::new()
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

    #[retry]
    async fn default_executor(agent: DemoStructWithAsync) -> RetryResult<u32, u32> {
        let mut guard = agent.lock().await;
        let res = guard.execute_async().await;
        match res {
            Ok(val) => Success(val.get().unwrap() as u32),
            Err(val) => Retry(val.get().unwrap() as u32),
        }
    }

    #[tokio::test]
    async fn agent_succeeds() {
        let agent = FallibleAgent::mutable(FallibleBehaviour::AlwaysSucceed);
        let res = takes_an_agent(agent).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn agent_fails() {
        let agent = FallibleAgent::mutable(FallibleBehaviour::AlwaysFail);
        let res = agent_executor(agent).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn agent_succeeds_after_5() {
        let agent = FallibleAgent::mutable(FallibleBehaviour::SucceedAfter(5));
        let res = agent_executor(agent.clone()).await;
        let c = agent.count().await;
        assert!(res.is_ok());
        assert_eq!(c, 5);
    }

    #[retry_prepare]
    async fn prepared_executor(agent: DemoStructWithAsync) -> RetryResult<u32, u32> {
        let mut guard = agent.lock().await;
        let res = guard.execute_async().await;
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
        let agent = get_async_demo_agent();
        let res = prepared_executor(agent).retry_with_default_policy().await;
        assert!(res.is_ok())
    }

    #[tokio::test]
    async fn prepared_functions_use_global_default_policy() {
        const DEFAULT_RETRIES: u64 = 50;

        /* SETUP DEFAULTS */

        let policy = RetryPolicy::builder()
            .limit(RetryLimit::Limited(50))
            .backoff_policy(constant_backoff)
            .base_delay(1) //1 ms delay so it runs quickly
            .build_with_defaults();
        global::set_default_policy(policy);



        /* TEST WITH NEW DEFAULT */

        let agent = get_delayed_success_agent(DEFAULT_RETRIES);

        let res = prepared_executor(agent.clone())
            .retry_with_default_policy()
            .await;
        assert!(&res.is_ok());
        let count = agent.count().await;
        assert_eq!(count, DEFAULT_RETRIES);


        /* RESET DEFAULTS */

        global::reset_default_policy();

        /* TEST WITH RESTORED DEFAULTS */

        let agent = get_delayed_success_agent(5); /* reduced number of counts, default policy has a longer delay so dont want it to take too long*/
        let res = prepared_executor(agent.clone())
            .retry_with_default_policy()
            .await;
        assert!(&res.is_ok());
        let count = agent.count().await;
        assert_eq!(count, 5);
    }

    #[tokio::test]
    async fn retry_functions_use_global_default_policy() {
        const DEFAULT_RETRIES: u64 = 50;

        /* SETUP DEFAULTs*/
        let policy = RetryPolicy::builder()
            .limit(RetryLimit::Limited(DEFAULT_RETRIES))
            .backoff_policy(constant_backoff)
            .base_delay(1) //1 ms delay so it runs quickly
            .build_with_defaults();

        global::set_default_policy(policy);

        /* TEST WITH NEW DEFAULT */

        let agent = get_delayed_success_agent(DEFAULT_RETRIES);
        let res = default_executor(agent.clone()).await;
        let count = agent.count().await;
        assert!(&res.is_ok());
        assert_eq!(count, DEFAULT_RETRIES);

        /* RESET DEFAULTS */

        global::reset_default_policy();

        /* TEST WITH RESTORED DEFAULTS */

        let agent = get_delayed_success_agent(5); /* reduced number of counts, default policy has a longer delay so dont want it to take too long*/
        let res = default_executor(agent.clone()).await;
        let count = agent.count().await;
        assert!(&res.is_ok());
        assert_eq!(count, 5);
    }

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


    #[tokio::test]
    async fn closures_use_global_default_policy() {
        const DEFAULT_RETRIES: u64 = 50;

        /* SETUP DEFAULTS */
        let policy = RetryPolicy::builder()
            .limit(RetryLimit::Limited(DEFAULT_RETRIES))
            .backoff_policy(constant_backoff)
            .base_delay(1) //1 ms delay so it runs quickly
            .build_with_defaults();

        global::set_default_policy(policy);

        /* Policy being tested after change by other tests*, so can reset immediately */

        global::reset_default_policy();

        /* TEST WITH RESTORED DEFAULTS */

        let agent = get_delayed_success_agent(5); /* reduced number of counts, default policy has a longer delay so dont want it to take too long*/

        let res = (|| async {
            match agent.execute().await {
                Ok(_v) => Success(()),
                Err(_e) => Retry(()),
            }
        }).retry_with_default_policy().await;


        let count = agent.count().await;
        assert!(&res.is_ok());
        assert_eq!(count, 5);
    }

    #[tokio::test]
    async fn runs_only_specified_number() {
        fn policy() -> RetryPolicy {
            RetryPolicyBuilder::new()
                .limit(RetryLimit::Limited(1))
                .backoff_policy(constant_backoff)
                .base_delay(1)
                .build()
        }

        #[retry(policy)]
        async fn f(agent: MutableAgent) -> RetryResult<(), ()> {
            let _ = agent.execute().await;
            Retry(())
        }

        let agent = FallibleAgent::mutable(FallibleBehaviour::AlwaysFail);

        let res = f(agent.clone()).await;

        assert!(res.is_err());
        assert_eq!(agent.count().await, 1);
    }

    #[tokio::test]
    async fn runs_only_specified_number_again() {
        fn policy() -> RetryPolicy {
            RetryPolicyBuilder::new()
                .limit(RetryLimit::Limited(100))
                .backoff_policy(constant_backoff)
                .base_delay(1)
                .build()
        }

        #[retry(policy)]
        async fn f(agent: MutableAgent) -> RetryResult<(), ()> {
            let r = agent.execute().await;
            match r {
                Ok(_) => Success(()),
                Err(_) => Retry(()),
            }
        }

        let agent = FallibleAgent::mutable(FallibleBehaviour::AlwaysFail);

        let res = f(agent.clone()).await;

        assert!(res.is_err());
        assert_eq!(agent.count().await, 100);
    }

    #[tokio::test]
    async fn wont_run_multiple_times_on_success() {
        fn policy() -> RetryPolicy {
            RetryPolicyBuilder::new()
                .limit(RetryLimit::Limited(2))
                .backoff_policy(constant_backoff)
                .base_delay(15)
                .build()
        }

        #[retry(policy)]
        async fn f(agent: MutableAgent) -> RetryResult<(), ()> {
            let r = agent.execute().await;
            match r {
                Ok(_) => Success(()),
                Err(_) => Retry(()),
            }
        }

        let agent = FallibleAgent::mutable(FallibleBehaviour::AlwaysSucceed);

        let res = f(agent.clone()).await;

        assert!(res.is_ok());
        assert_eq!(agent.count().await, 1);
    }

    #[retry_prepare]
    pub async fn ref_function(agent: MutableAgent, _: &str) -> RetryResult<(), ()> {
        let r = agent.execute().await;
        match r {
            Ok(_) => Success(()),
            Err(_) => Retry(()),
        }
    }

    #[tokio::test]
    async fn can_run_prepared_function_on_policy() {
        fn policy() -> RetryPolicy {
            RetryPolicy::builder()
                .limit(RetryLimit::Limited(2))
                .backoff_policy(constant_backoff)
                .base_delay(15)
                .build()
        }
        #[retry_prepare]
        pub async fn f(agent: MutableAgent) -> RetryResult<(), ()> {
            let r = agent.execute().await;
            match r {
                Ok(_) => Success(()),
                Err(_) => Retry(()),
            }
        }

        #[retry]
        pub async fn f3(agent: MutableAgent) -> RetryResult<(), ()> {
            let r = agent.execute().await;
            match r {
                Ok(_) => Success(()),
                Err(_) => Retry(()),
            }
        }

        let agent = FallibleAgent::mutable(FallibleBehaviour::AlwaysSucceed);
        let v = f(agent.clone());
        let res = policy().call(v).await;
        assert!(res.is_ok());
        assert_eq!(agent.count().await, 1);
        let agent = FallibleAgent::mutable(FallibleBehaviour::AlwaysSucceed);
        let some_str = "hello";
        let v = ref_function(agent.clone(), some_str);
        let res = policy().call(v).await;

        assert!(res.is_ok());
        assert_eq!(agent.count().await, 1);

        let _ = f3(agent.clone()).await;

        // policy().call()
    }

    fn get_async_demo_agent() -> DemoStructWithAsync {
        FallibleAgent::mutable(FallibleBehaviour::AlwaysSucceed)
    }

    fn get_delayed_success_agent(tries: u64) -> DemoStructWithAsync {
        FallibleAgent::mutable(FallibleBehaviour::SucceedAfter(tries))
    }

    #[tokio::test]
    async fn async_closure_testing() {
        //retry_function
        let policy = RetryPolicy::builder()
            .limit(RetryLimit::Limited(20))
            .backoff_policy(constant_backoff)
            .base_delay(15)
            .build();

        let agent =
            FallibleAgent::mutable(FallibleBehaviour::SucceedAfter(15));

        let f = || async {
            match agent.execute().await {
                Ok(_v) => Success(()),
                Err(_e) => Retry(()),
            }
        };

        let res = policy.call_closure(f).await;

        let count = agent.count().await;

        assert!(res.is_ok());
        assert_eq!(count, 15);
    }

    #[tokio::test]
    async fn async_closure_testing_impl_on_closure() {
        //retry_function
        let policy = RetryPolicy::builder()
            .limit(RetryLimit::Limited(20))
            .backoff_policy(constant_backoff)
            .base_delay(15)
            .build();

        let agent =
            FallibleAgent::mutable(FallibleBehaviour::SucceedAfter(15));

        let res = (|| async {
            match agent.execute().await {
                Ok(_v) => Success(()),
                Err(_e) => Retry(()),
            }
        })
        .retry(&policy)
        .await;

        let count = agent.count().await;

        assert!(res.is_ok());
        assert_eq!(count, 15);
    }

    fn generate_random_number() -> u8 {
        let mut rng = rand::rng();
        rng.random_range(1..=100)
    }

    const INTERMITTENT_ERROR: &str = "intermittent error";
    const SIMULATED_ERROR: &str = "simulated error";

    const INPUT_STR: &str = "test string";

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
                abort(SIMULATED_ERROR.to_string())
            } else {
                retry(INTERMITTENT_ERROR.to_string())
            }
        }

        let func = PreparedExampleFunction(1u32, INPUT_STR.to_string());
        let mut ex = func.prepare();

        let p = RetryPolicy {
            limit: RetryLimit::Limited(10),
            base_delay: 500,
            delay_time: constant_backoff,
        };

        ex.set_policy(p);

        let r = ex.run().await;

        match r {
            Ok(v) => {
                assert_eq!(v, format!("1_{INPUT_STR}"));
                println!("Success: {}", v);
            }
            Err(e) => {
                if e.as_str() == SIMULATED_ERROR {
                    return;
                }

                //if we're here on an intermittent error, it means we have exhausted the retries
                //so c == 10
                assert_eq!(ex.count(), 10);
            }
        }
    }
}
