#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
mod db;
mod error;
mod postgres;
mod queue;
use std::{sync::Arc, time::Duration};

pub use error::Error;
use futures::{stream, StreamExt};
use postgres::PostgresQueue;
use queue::{Job, Message, Queue};
use rocket::routes;
const CONCURRENCY: usize = 50;
#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Start Rocket Server
    rocket::ignite().mount("/", routes![index]).launch();

    // Grab the DB url from env and print a success message in the terminal
    let database_url = std::env::var("DATABASE_URL").map_err(|_| {
        Error::BadConfig("DATABASE_URL IS NOT FOUND! PLEASE SET AS AN ENV VARIABLE".to_string())
    })?;
    println!("Connected to Database, DB URL: {:?},", database_url);

    // Use db url to connect to database and initiate migration
    let db = db::connect(&database_url).await?;
    db::migrate(&db).await?;

    // Create a new PostgresQueue wrapped in an atomic reference counter
    let queue = Arc::new(PostgresQueue::new(db.clone()));

    let queue_1 = queue.clone();

    // Spawn a Tokio green thread
    tokio::spawn(async move { run_worker(queue_1).await });

    // TEST JOB
    let job = Message::Detail {
        item: "JOB DETAIL HERE".to_string(),
    };

    // Add API logic to allow user to add jobs to the db
    // Push jobs to queue

    let _ = queue.push(job, None).await;
    tokio::time::sleep(Duration::from_secs(2)).await;

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
            println!("Fetched {} jobs", number_of_jobs);
        }

        stream::iter(jobs)
            .for_each_concurrent(CONCURRENCY, |job| async {
                let job_id = job.id;
                // add repeat interval logic in match need to add a repeat time (duration from)
                // see: https://docs.rs/chrono/0.4.0/chrono/struct.DateTime.html
                // if datetime is not now then skip
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

        // sleep not to overload our database
        tokio::time::sleep(Duration::from_millis(125)).await;
    }
}

async fn handle_job(job: Job) -> Result<(), crate::Error> {
    match job.message {
        message @ Message::Detail { .. } => {
            println!("Job Message: {:?}", &message);
        }
    };

    Ok(())
}

// ROCKET CONFIGURATION BELOW

#[get("/")]
fn index() -> &'static str {
    "RUST JOB SCHEDULER"
}
