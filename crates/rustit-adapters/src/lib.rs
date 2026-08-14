//! Vendor-neutral boundaries for schedule and model interoperability.

use rustit_ifc::IfcSchemaVersion;
use rustit_model::BimModel;
use rustit_schedule::Schedule;
use serde::{Deserialize, Serialize};
use std::{future::Future, pin::Pin};
use thiserror::Error;
use uuid::Uuid;

pub type AdapterFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, AdapterError>> + Send + 'a>>;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExternalSystem {
    PrimaveraP6,
    OraclePrimaveraCloud,
    BentleySynchro,
    Ifc(IfcSchemaVersion),
    Other(String),
}

/// Maps a stable Rustit object identity to a vendor-owned identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalIdentity {
    pub object_id: Uuid,
    pub system: ExternalSystem,
    pub external_id: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AdapterCapabilities {
    pub can_import: bool,
    pub can_create: bool,
    pub can_update: bool,
    pub can_delete: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SyncPlan {
    pub creates: usize,
    pub updates: usize,
    pub deletes: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportResult<T> {
    pub value: T,
    pub identities: Vec<ExternalIdentity>,
    pub warnings: Vec<String>,
}

/// Adapter implemented by P6, OPC, SYNCHRO, and future schedule connectors.
pub trait ScheduleAdapter: Send + Sync {
    fn system(&self) -> ExternalSystem;
    fn capabilities(&self) -> AdapterCapabilities;
    fn import_schedule<'a>(&'a self) -> AdapterFuture<'a, ImportResult<Schedule>>;
    fn plan_export<'a>(&'a self, schedule: &'a Schedule) -> AdapterFuture<'a, SyncPlan>;
    fn apply_export<'a>(&'a self, schedule: &'a Schedule) -> AdapterFuture<'a, SyncPlan>;
}

/// Adapter implemented by IFC and future model interchange providers.
pub trait ModelAdapter: Send + Sync {
    fn system(&self) -> ExternalSystem;
    fn capabilities(&self) -> AdapterCapabilities;
    fn import_model<'a>(&'a self) -> AdapterFuture<'a, ImportResult<BimModel>>;
    fn plan_export<'a>(&'a self, model: &'a BimModel) -> AdapterFuture<'a, SyncPlan>;
    fn apply_export<'a>(&'a self, model: &'a BimModel) -> AdapterFuture<'a, SyncPlan>;
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AdapterError {
    #[error("adapter authentication failed")]
    Authentication,
    #[error("adapter operation is not supported")]
    Unsupported,
    #[error("external data is invalid: {0}")]
    InvalidData(String),
    #[error("external system is unavailable: {0}")]
    Unavailable(String),
    #[error("adapter operation failed: {0}")]
    Other(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supports_future_vendor_systems_without_changing_core_types() {
        let system = ExternalSystem::Other("Borant (fictional)".to_owned());

        assert_eq!(
            system,
            ExternalSystem::Other("Borant (fictional)".to_owned())
        );
    }
}
