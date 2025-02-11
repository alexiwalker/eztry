use rand::Rng;
use retriers_lib::*;
use retriers_macro::{retry, retry_prepare};
fn generate_random_number() -> u8 {
    let mut rng = rand::rng();
    rng.random_range(1..=100)
}


#[retry_prepare]
async fn prepared(
    v: u32,
    s: String,
    b: bool,
    f: f32,
) -> RetryResult<(String,u32), String> {
    let mut rng = generate_random_number();
    if rng < 30 {
        let data_1 = v;
        let data_2 = s;
        let s = format!("{data_1}_{data_2}_{b}::{f}");
        let _ = tokio::fs::write("./tmp_file.txt", &s).await;
        success((s, data_1))
    } else if rng > 95 {
        abort("simulated error".to_string())
    } else {
        retry("simulated retry".to_string())
    }
}

fn default_policy()->RetryPolicy {
    RetryPolicyBuilder::new_with_defaults()
        .delay_calculator(|policy, attempt| {
            let multiplier = 2u64.pow(attempt as u32 - 1);
            let delay = policy.base_delay * multiplier;
            delay as u64
        })
        .limit(RetryLimit::Limited(5))
        .base_delay(250)
        .build_with_defaults()
}

#[tokio::main]
async fn main() {
    let x = prepared(1u32, "something".to_string(), true, 0.01).retry_with_default_policy().await;
    // let x = retry_func(1u32, "something".to_string(), true, 0.01).retry_with_policy(default_policy()).await;
    match x {
        Ok(v) => {
            println!("Success: {:?}", v);
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }

    let x = retryable(1u32, "something".to_string(), true, 0.01).await;
    // let x = retry_func(1u32, "something".to_string(), true, 0.01).retry_with_policy(default_policy()).await;
    match x {
        Ok(v) => {
            println!("Success: {:?}", v);
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }
}


#[retry]
async fn retryable(
    v: u32,
    s: String,
    b: bool,
    f: f32,
) -> RetryResult<(String, u32), String> {
    let mut rng = generate_random_number();
    if rng < 30 {
        let data_1 = v;
        let data_2 = s;
        let s = format!("{data_1}_{data_2}_{b}::{f}");
        let _ = tokio::fs::write("./tmp_file.txt", &s).await;
        success((s, data_1))
    } else if rng > 95 {
        abort("simulated error".to_string())
    } else {
        retry("simulated retry".to_string())
    }
}

/*
#[allow(non_camel_case_types)]
struct retry_func(
    u32,
    String,
    bool,
    f32,
);
#[async_trait]
impl Executor<(String, u32), String> for retry_func {
    async fn execute(&self) -> RetryResult<(String, u32), String> {
        __RETRIERS__INTERNAL::retry_func_inner(
            self.0.clone(), self.1.clone(), self.2.clone(), self.3.clone()).await
    }
}
#[doc(hidden)]
mod __RETRIERS__INTERNAL {
    use super::*;
    #[doc(hidden)]
    pub async fn retry_func_inner(
        v: u32,
        s: String,
        b: bool,
        f: f32,
    ) -> RetryResult<(String, u32), String> {
        let mut rng = generate_random_number();
        if rng < 30 {
            let data_1 = v;
            let data_2 = s;
            let s = format!("{data_1}_{data_2}_{b}::{f}");
            let _ = tokio::fs::write("./tmp_file.txt", &s).await;
            success((s, data_1))
        } else if rng > 95 {
            abort("simulated error".to_string())
        } else {
            retry("simulated retry".to_string())
        }
    }
}

*/
//
// async fn __example_function(v: u32, s: String, b: bool, f: f32) ->
// Result<(String, u32), String>
// {
//     #[allow(non_camel_case_types)]
//     struct
//     __inner__struct(u32, String, bool, f32);
//     async fn
//     __inner__(v: u32, s: String, b: bool, f: f32) -> RetryResult<
//         (String, u32), String>
//     {
//         let mut rng = generate_random_number();
//         if rng < 30
//         {
//             let data_1 = v;
//             let data_2 = s;
//             let s = format!("{data_1}_{data_2}_{b}::{f}");
//             let _ = tokio::fs::
//             write("./tmp_file.txt", &s).await;
//             success((s, data_1))
//         } else if rng > 95 { abort("simulated error".to_string()) } else { retry("simulated retry".to_string()) }
//     }
//     #[async_trait]
//     impl Executor<(String, u32), String> for
//     __inner__struct
//     {
//         async fn execute(&self) -> RetryResult<(String, u32), String>
//         {
//             __inner__(self.0.clone(), self.1.clone(), self.2.clone(),
//                       self.3.clone()).await
//         }
//     }
//     let ex = __inner__struct(v, s, b, f);
//     ex.retry_with_default_policy().await
// }