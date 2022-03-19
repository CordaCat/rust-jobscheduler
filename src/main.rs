#![feature(proc_macro_hygiene, decl_macro)]
#![allow(unused)] // silence unused warnings
#[macro_use]
extern crate rocket;

use sqlx::postgres::{PgPoolOptions, PgRow};
use sqlx::{types::Json, FromRow, Row};
mod db;
mod error;
mod postgres;
mod queue;
pub use error::Error;
use futures::{stream, StreamExt};
use postgres::PostgresQueue;
use queue::{Job, Message, Queue};
use rocket::routes;
use std::{sync::Arc, time::Duration};
use uuid::Uuid;
const CONCURRENCY: usize = 5000;
#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // DATABASE SETUP
    // Grab the DB url from env and print a success message in the terminal
    let database_url = std::env::var("DATABASE_URL").map_err(|_| {
        Error::BadConfig("DATABASE_URL IS NOT FOUND! PLEASE SET AS AN ENV VARIABLE".to_string())
    })?;
    println!("Connected to Database, DB URL: {:?},", database_url);
    // Use db url to connect to database and initiate migration
    let db = db::connect(&database_url).await?;
    db::migrate(&db).await?;
    rocket().launch();

    // let queue = Arc::new(PostgresQueue::new(db.clone()));
    // tokio::spawn(async move { run_worker(queue).await });
    // tokio::time::sleep(Duration::from_secs(2)).await;

    Ok(())
}

async fn run_worker(queue: Arc<dyn Queue>) {
    loop {
        let jobs = match queue.pull(CONCURRENCY as u32).await {
            Ok(jobs) => jobs,
            Err(err) => {
                println!("worker is pulling jobs: {}", err);
                tokio::time::sleep(Duration::from_millis(500)).await;
                Vec::new()
            }
        };

        let number_of_jobs = jobs.len();

        if number_of_jobs > 0 {
            println!("Fetched {:?} jobs", number_of_jobs);
        } else {
            println!("NO MORE JOBS! WAITING FOR MORE JOBS....");
            tokio::time::sleep(Duration::from_secs(5)).await;
        }

        println!("WORKER STARTING...");
        // Worker starts processing jobs here
        stream::iter(jobs)
            .for_each_concurrent(CONCURRENCY, |job| async {
                let job_id = job.id;
                let res = match handle_job(job).await {
                    Ok(_) => queue.delete_job(job_id).await,
                    Err(err) => {
                        println!("run_worker: handling job({}): {}", job_id, &err);
                        queue.fail_job(job_id).await
                    }
                };

                match res {
                    Ok(_) => {}
                    Err(err) => {
                        println!("run_worker: deleting / failing job: {}", &err);
                    }
                }
            })
            .await;
        println!("STREAM ENDS HERE");

        // sleep not to overload our database
        tokio::time::sleep(Duration::from_millis(5000)).await;
    }
}

// ================================= HANDLE JOB FUNCTION =================================
async fn handle_job(job: Job) -> Result<(), crate::Error> {
    match job.message {
        message @ Message::Detail { .. } => {
            println!("Job Message: {:?}", &message);
        }
    };

    Ok(())
}

#[get("/")]
fn get_jobs() -> String {
    String::from("Jobs")
}

fn rocket() -> rocket::Rocket {
    rocket::ignite().mount("/jobs", routes![get_jobs])
}
