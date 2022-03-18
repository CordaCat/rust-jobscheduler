mod db;
mod error;
mod postgres;
mod queue;
use std::{sync::Arc, time::Duration};

pub use error::Error;
use futures::{stream, StreamExt};
use postgres::PostgresQueue;
use queue::{Job, Message, Queue};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    println!("Hello, world!");
    Ok(())
}
