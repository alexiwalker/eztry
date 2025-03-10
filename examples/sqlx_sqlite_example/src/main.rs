use eztry::prelude::*;
use sqlx::{Executor, Pool, Sqlite};
use std::sync::Arc;

/// Simulating a Database connection that may be stored on a struct to represent the resources available to an application
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
        _other_value: &'_ str,
    ) -> RetryResult<String, sqlx::Error> {
        let result = sqlx::query("insert into test (id, name) values (random(), ?)")
            .bind(v)
            .execute(self.pool.as_ref())
            .await;

        let row_id = result.unwrap().last_insert_rowid();

        let res_str = format!("Row ID: {row_id}, input:{_other_value}");

        if row_id % 15 == 0 {
            Success(res_str)
        } else if row_id % 100 == 0 {
            Abort(sqlx::Error::PoolClosed)
        } else {
            Retry(sqlx::Error::RowNotFound)
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
/*

This example will randomly fail, succeed, or retry based on RNG

Used to demonstrate behaviour when things don't always work after N tries, and will exit eventually

*/
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
