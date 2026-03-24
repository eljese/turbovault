//! Lock management tools for collaborative editing

use std::path::PathBuf;
use std::sync::Arc;
use turbovault_vault::VaultManager;
use turbovault_core::prelude::*;

/// Lock tools context
#[derive(Clone)]
pub struct LockTools {
    pub manager: Arc<VaultManager>,
}

impl LockTools {
    /// Create new lock tools
    pub fn new(manager: Arc<VaultManager>) -> Self {
        Self { manager }
    }

    /// Acquire a lock on a file
    pub async fn acquire_lock(
        &self,
        path: String,
        owner: String,
        timeout: Option<u64>,
    ) -> Result<Lock> {
        let file_path = PathBuf::from(path);
        // Convert u64 seconds to f64 for implementation compatibility
        let timeout_f64 = timeout.map(|t| t as f64);
        self.manager
            .acquire_lock(&file_path, &owner, timeout_f64)
            .await
    }

    /// Release a lock on a file
    pub async fn release_lock(&self, path: String, owner: String) -> Result<()> {
        let file_path = PathBuf::from(path);
        self.manager
            .release_lock(&file_path, &owner)
            .await
    }

    /// Check the lock status of a file
    pub async fn check_lock(&self, path: String) -> Result<Option<Lock>> {
        let file_path = PathBuf::from(path);
        self.manager
            .get_lock(&file_path)
            .await
    }
}
