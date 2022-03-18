// We define our own Error types in this file using the thiserror crate.
// The Error type below is our own custom error type
#[derive(thiserror::Error, Debug, Clone)]
pub enum Error {
    #[error("Bad config: {0}")]
    BadConfig(String),
    #[error("Connecting to database: {0}")]
    ConnectingToDatabase(String),
    #[error("Internal error: {0}")]
    Internal(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Migrating database: {0}")]
    DatabaseMigration(String),
}
// We define a from implementation to convert from a sql error to our
// own Error type (defined above)
// the from implementation below specfically converts from a sql row not found error
// to an Error::NotFound or Error::Internal type
impl std::convert::From<sqlx::Error> for Error {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => Error::NotFound("row not found".into()),
            _ => Error::Internal(err.to_string()),
        }
    }
}
// the from implementation below specfically converts from a sql migration error type
// to an Error::DatabaseMigration type
impl std::convert::From<sqlx::migrate::MigrateError> for Error {
    fn from(err: sqlx::migrate::MigrateError) -> Self {
        Error::DatabaseMigration(err.to_string())
    }
}
