use crate::{
    db::DB,
    queue::{Job, Message, Queue},
};
use chrono;
use sqlx::{self, types::Json};
use ulid::Ulid;
use uuid::Uuid;

// We create a PostgresQueue type, it defines the actual database and the maximum tries
#[derive(Debug, Clone)]
pub struct PostgresQueue {
    db: DB,
    max_attempts: u32,
}
const MAX_FAILED_ATTEMPTS: i32 = 3;

// We create a PostgresJob type, and derive features that allow us to copy, debug and convert from a pg row type
//  to a PostgresJob type
#[derive(sqlx::FromRow, Debug, Clone)]
struct PostgresJob {
    id: Uuid,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    scheduled_for: chrono::DateTime<chrono::Utc>,
    // repeat: i32,
    failed_attempts: i32,
    status: PostgresJobStatus,
    message: Json<Message>,
}

// We assign job status as an i32 (in postgres an INT)
// This is done to improve performance
// We also derive additional features for this data type
// using the derive macro
#[derive(Debug, Clone, sqlx::Type, PartialEq)]
#[repr(i32)]
enum PostgresJobStatus {
    Queued,
    Running,
    Failed,
}

// This implementation allows us to convert from a PostgresJob type
// to a Job type (The Job type is defined in queue.rs).
impl From<PostgresJob> for Job {
    fn from(item: PostgresJob) -> Self {
        Job {
            id: item.id,
            message: item.message.0,
            // repeat,
        }
    }
}

// We create a new function to initialize a new instance of the PostgresQueue type
impl PostgresQueue {
    pub fn new(db: DB) -> PostgresQueue {
        let queue = PostgresQueue {
            db,
            max_attempts: 5,
        };

        queue
    }
}

// Here we implement the Queue trait from queue.rs...This allows us to have a concrete implementations
// of Queue (push, pull, delete, cancel and clear) for the PostgresQueue type
#[async_trait::async_trait]
impl Queue for PostgresQueue {
    // Add create and update functions for Rocket API

    async fn push(
        &self,
        job: Message,
        date: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<(), crate::Error> {
        let scheduled_for = date.unwrap_or(chrono::Utc::now());
        let failed_attempts: i32 = 0;
        // let repeat: i32 = 0;
        let message = Json(job);
        let status = PostgresJobStatus::Queued;
        let now = chrono::Utc::now();
        let job_id: Uuid = Ulid::new().into();
        let query = "INSERT INTO queue
            (id, created_at, updated_at, scheduled_for, failed_attempts, status, message)
            VALUES ($1, $2, $3, $4, $5, $6, $7)";

        sqlx::query(query)
            .bind(job_id)
            .bind(now)
            .bind(now)
            .bind(scheduled_for)
            // .bind(repeat)
            .bind(failed_attempts)
            .bind(status)
            .bind(message)
            .execute(&self.db)
            .await?;
        Ok(())
    }

    async fn delete_job(&self, job_id: Uuid) -> Result<(), crate::Error> {
        let query = "DELETE FROM queue WHERE id = $1";

        sqlx::query(query).bind(job_id).execute(&self.db).await?;
        Ok(())
    }

    async fn fail_job(&self, job_id: Uuid) -> Result<(), crate::Error> {
        let now = chrono::Utc::now();
        let query = "UPDATE queue
            SET status = $1, updated_at = $2, failed_attempts = failed_attempts + 1
            WHERE id = $3";

        sqlx::query(query)
            .bind(PostgresJobStatus::Queued)
            .bind(now)
            .bind(job_id)
            .execute(&self.db)
            .await?;
        Ok(())
    }

    async fn pull(&self, number_of_jobs: u32) -> Result<Vec<Job>, crate::Error> {
        let number_of_jobs = if number_of_jobs > 100 {
            10000
        } else {
            number_of_jobs
        };
        let now = chrono::Utc::now();
        let query = "UPDATE queue
            SET status = $1, updated_at = $2
            WHERE id IN (
                SELECT id
                FROM queue
                WHERE status = $3 AND scheduled_for <= $4 AND failed_attempts < $5
                ORDER BY scheduled_for
                FOR UPDATE SKIP LOCKED
                LIMIT $6
            )
            RETURNING *";

        let jobs: Vec<PostgresJob> = sqlx::query_as::<_, PostgresJob>(query)
            .bind(PostgresJobStatus::Running)
            .bind(now)
            .bind(PostgresJobStatus::Queued)
            .bind(now)
            .bind(MAX_FAILED_ATTEMPTS)
            .bind(number_of_jobs)
            .fetch_all(&self.db)
            .await?;
        Ok(jobs.into_iter().map(Into::into).collect())
    }

    async fn clear(&self) -> Result<(), crate::Error> {
        let query = "DELETE FROM queue";

        sqlx::query(query).execute(&self.db).await?;
        Ok(())
    }
}
