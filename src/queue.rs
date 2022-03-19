use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use uuid::Uuid;

// We define a trait to represent our job queue.  Tasks
// that are spawned by the Tokio runtime must implement Send and Sync
// so that they may be shared between threads.
#[async_trait::async_trait]
pub trait Queue: Send + Sync + Debug {
    //  Push a job into the queue

    async fn push(
        &self,
        job: Message,
        scheduled_for: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<(), crate::Error>;
    // Get jobs
    async fn pull(&self, number_of_jobs: u32) -> Result<Vec<Job>, crate::Error>;
    // Delete job
    async fn delete_job(&self, job_id: Uuid) -> Result<(), crate::Error>;
    // Cancel job
    async fn fail_job(&self, job_id: Uuid) -> Result<(), crate::Error>;
    // Clear job queue
    async fn clear(&self) -> Result<(), crate::Error>;
}

// We define a struct that will represent instances of a job . We also derive
// additional features that allow us to debug, copy and serialize/de-serialize
// between data types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: Uuid,
    pub message: Message,
    // pub repeat: i32,
}

// We define a message data type to hold some kind of job related message.
// From here we can customize the payload of the job.  For this implementation
// we use a simple string.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    Detail { item: String },
    // We can add additional job functionality here
}
