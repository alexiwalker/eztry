use rand::Rng;
use retriers_lib::*;
use retriers_macro::retryable;

type RetryType = RetryResult<(String,u32), String>;
// type RetryType = RetryResult<String, String>;

#[retryable]
async fn retry_func(
    v: u32,
    s: String,
    b: bool,
    f: f32,
) ->RetryType {
// ) -> RetryResult<(String,u32), String> {
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

fn generate_random_number() -> u8 {
    let mut rng = rand::rng();
    rng.random_range(1..=100)
}

#[tokio::main]
async fn main() {
    let r = retry_func(1u32, "something".to_string(), true, 0.01);

    let x = r.retry_with_policy(default_policy()).await;
    match x {
        Ok(v) => {
            println!("Success: {:?}", v);
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
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