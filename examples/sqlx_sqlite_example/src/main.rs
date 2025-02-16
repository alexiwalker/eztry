use retry_rs::*;
use retry_rs_macros::retry;
use sqlx::{Executor, Pool, Sqlite};
use std::sync::Arc;

pub struct SqliteDb {
    pool: Arc<Pool<Sqlite>>,
}

impl SqliteDb {
    pub async fn new() -> Self {
        let pool: Pool<Sqlite> = Pool::connect("sqlite::memory:").await.unwrap();

        let _ = pool
            .execute("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)")
            .await;

        Self {
            pool: Arc::new(pool),
        }
    }


    /// A test query that may fail, retry, or abort based on the rowid of the inserted row
    /// Simulates a query that may fail for any reason, and must check the result of the async operation
    /// to determine if a retry is appropriate
    #[retry(policy)]
    pub async fn execute_test_query(
        &self,
        v: String,
        other_value: &'_ str,
    ) -> RetryResult<String, sqlx::Error> {
        println!(
            "executing the test query with a passed reference value with a lifetime: {}",
            other_value
        );

        let r = sqlx::query("insert into test (id, name) values (random(), ?)")
            .bind(v)
            .execute(self.pool.as_ref())
            .await;

        let rid = r.unwrap().last_insert_rowid();

        if rid % 15 == 0 {
            RetryResult::Success(rid.to_string())
        } else if rid % 100 == 0 {
            RetryResult::Abort(sqlx::Error::RowNotFound)
        } else {
            RetryResult::Retry(sqlx::Error::RowNotFound)
        }
    }
}

// simulating an application that may have more resources than just a sqlite db instance
pub struct AppResources {
    pub db: SqliteDb,
}

impl AppResources {
    pub async fn new() -> Self {
        let db = SqliteDb::new().await;
        Self { db }
    }
}

/// A limited attempt retry policy with a constant backoff
fn policy() -> RetryPolicy {
    RetryPolicyBuilder::new()
        .backoff_policy(constant_backoff)
        .base_delay(15)
        .limit(RetryLimit::Limited(10))
        .build()
}

#[tokio::main]
async fn main() {
    /* Simulating a bundle of resources that may be in a struct in a real-world API / Server*/
    let resources = AppResources::new().await;

    let ref_string = "hello";
    let res = resources
        .db
        .execute_test_query("EXAMPLE_NAME".to_string(), ref_string)
        .await;

    match res {
        Ok(_) => {
            println!("Success!");
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }
}
