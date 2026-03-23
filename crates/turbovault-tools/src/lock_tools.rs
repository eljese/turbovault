//! Lock management tools for collaborative editing

use std::path::PathBuf;
use std::sync::Arc;
use turbomcp::prelude::*;
use turbovault_vault::VaultManager;
use turbovault_core::models::Lock;

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
    #[tool("Acquire a lock on a file for collaborative editing. Locks prevent other users/agents from modifying the file until released or expired.")]
    pub async fn acquire_lock(
        &self,
        path: String,
        owner: String,
        timeout: Option<f64>,
    ) -> Result<Lock, McpError> {
        let file_path = PathBuf::from(path);
        self.manager
            .acquire_lock(&file_path, &owner, timeout)
            .await
            .map_err(|e| McpError::tool(e.to_string()))
    }

    /// Release a lock on a file
    #[tool("Release a previously acquired lock on a file. Only the lock owner can release it.")]
    pub async fn release_lock(&self, path: String, owner: String) -> Result<(), McpError> {
        let file_path = PathBuf::from(path);
        self.manager
            .release_lock(&file_path, &owner)
            .await
            .map_err(|e| McpError::tool(e.to_string()))
    }

    /// Check the lock status of a file
    #[tool("Check if a file is currently locked and by whom.")]
    pub async fn check_lock(&self, path: String) -> Result<Option<Lock>, McpError> {
        let file_path = PathBuf::from(path);
        
        self.manager
            .get_lock(&file_path)
            .await
            .map_err(|e| McpError::tool(e.to_string()))
    }
}
