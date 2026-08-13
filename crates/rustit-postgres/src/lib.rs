//! PostgreSQL persistence boundaries.
//!
//! v0.0.1 defines the contract without selecting a driver, schema layout, or
//! migration framework. Those choices should follow the project-model RFC.

use rustit_core::{Project, ProjectId};
use std::{future::Future, pin::Pin};
use thiserror::Error;

pub type RepositoryFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, RepositoryError>> + Send + 'a>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectVersion(pub u64);

#[derive(Debug, Clone, PartialEq)]
pub struct StoredProject {
    pub project: Project,
    pub version: ProjectVersion,
}

/// Persistence port used by application services and headless clients.
pub trait ProjectRepository: Send + Sync {
    fn load<'a>(&'a self, id: ProjectId) -> RepositoryFuture<'a, Option<StoredProject>>;

    fn save<'a>(
        &'a self,
        project: &'a Project,
        expected_version: Option<ProjectVersion>,
    ) -> RepositoryFuture<'a, StoredProject>;
}

/// Non-secret connection settings for a PostgreSQL-backed repository.
///
/// The URL itself is read from the named environment variable at runtime and
/// must never be committed to the repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostgresOptions {
    pub database_url_env: String,
    pub schema: String,
    pub application_name: String,
}

impl Default for PostgresOptions {
    fn default() -> Self {
        Self {
            database_url_env: "RUSTIT_DATABASE_URL".to_owned(),
            schema: "rustit".to_owned(),
            application_name: "rustit".to_owned(),
        }
    }
}

/// Marker contract for repositories implemented on PostgreSQL/PostGIS.
pub trait PostgresProjectRepository: ProjectRepository {
    fn options(&self) -> &PostgresOptions;
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RepositoryError {
    #[error("project {0} was not found")]
    NotFound(ProjectId),
    #[error("project was updated by another transaction")]
    VersionConflict,
    #[error("persistence backend is unavailable: {0}")]
    Unavailable(String),
    #[error("persistence operation failed: {0}")]
    Backend(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_keep_database_secrets_out_of_source() {
        let options = PostgresOptions::default();

        assert_eq!(options.database_url_env, "RUSTIT_DATABASE_URL");
        assert_eq!(options.schema, "rustit");
    }
}
