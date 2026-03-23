//! Audit trail tools for querying operation history and managing rollbacks

use std::sync::Arc;
use turbovault_audit::{AuditEntry, AuditFilter, AuditLog, AuditStats};
use turbovault_audit::{RollbackEngine, RollbackPreview, RollbackResult, SnapshotStore};
use turbovault_core::prelude::*;

/// Audit tools for querying and managing the operation audit trail
pub struct AuditTools {
    audit_log: Arc<AuditLog>,
    snapshot_store: Arc<SnapshotStore>,
}

impl AuditTools {
    pub fn new(audit_log: Arc<AuditLog>, snapshot_store: Arc<SnapshotStore>) -> Self {
        Self {
            audit_log,
            snapshot_store,
        }
    }

    /// Query audit log with filters
    pub async fn query_log(&self, filter: &AuditFilter) -> Result<Vec<AuditEntry>> {
        self.audit_log.query(filter).await
    }

    /// Get audit statistics
    pub async fn stats(&self) -> Result<AuditStats> {
        self.audit_log.stats().await
    }

    /// Preview a rollback (dry run)
    pub async fn rollback_preview(
        &self,
        operation_id: &str,
        vault_path: &std::path::Path,
    ) -> Result<RollbackPreview> {
        let engine = RollbackEngine::new(self.audit_log.clone(), self.snapshot_store.clone());
        engine.preview(operation_id, vault_path).await
    }

    /// Execute a rollback
    pub async fn rollback_execute(
        &self,
        operation_id: &str,
        vault_path: &std::path::Path,
    ) -> Result<RollbackResult> {
        let engine = RollbackEngine::new(self.audit_log.clone(), self.snapshot_store.clone());
        engine.execute(operation_id, vault_path).await
    }

    /// Get the snapshot store (for diff_note_version integration)
    pub fn snapshot_store(&self) -> &SnapshotStore {
        &self.snapshot_store
    }

    /// Get the audit log (for direct access)
    pub fn audit_log(&self) -> &AuditLog {
        &self.audit_log
    }
}
