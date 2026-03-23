#[cfg(test)]
mod tests {
    use crate::VaultManager;
    use turbovault_core::prelude::*;
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;

    fn create_test_config(vault_dir: &Path) -> ServerConfig {
        let mut config = ServerConfig::new();
        let vault_config = VaultConfig::builder("test_vault", vault_dir)
            .build()
            .unwrap();
        config.vaults.push(vault_config);
        config
    }

    #[tokio::test]
    async fn test_acquire_lock() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(temp_dir.path());
        let manager = VaultManager::new(config).unwrap();

        let path = Path::new("locked_file.md");
        let owner = "Agent-Alice";

        // Acquire lock
        let lock = manager.acquire_lock(path, owner, None).await.unwrap();
        assert_eq!(lock.owner, owner);

        // Check lock
        let current_lock = manager.get_lock(path).await.unwrap();
        assert!(current_lock.is_some());
        assert_eq!(current_lock.unwrap().owner, owner);
    }

    #[tokio::test]
    async fn test_release_lock() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(temp_dir.path());
        let manager = VaultManager::new(config).unwrap();

        let path = Path::new("locked_file.md");
        let owner = "Agent-Alice";

        manager.acquire_lock(path, owner, None).await.unwrap();
        manager.release_lock(path, owner).await.unwrap();

        let current_lock = manager.get_lock(path).await.unwrap();
        assert!(current_lock.is_none());
    }

    #[tokio::test]
    async fn test_lock_conflict() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(temp_dir.path());
        let manager = VaultManager::new(config).unwrap();

        let path = Path::new("locked_file.md");
        
        manager.acquire_lock(path, "Agent-Alice", None).await.unwrap();
        
        // Try to acquire same lock with different owner
        let result = manager.acquire_lock(path, "Agent-Bob", None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_lock_expiration() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(temp_dir.path());
        let manager = VaultManager::new(config).unwrap();

        let path = Path::new("expiring_file.md");
        
        // Lock with 0.1s timeout
        manager.acquire_lock(path, "Agent-Alice", Some(0.1)).await.unwrap();
        
        // Wait for expiration
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        
        // Should be able to acquire now
        let lock = manager.acquire_lock(path, "Agent-Bob", None).await.unwrap();
        assert_eq!(lock.owner, "Agent-Bob");
    }

    #[tokio::test]
    async fn test_write_locked_file() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(temp_dir.path());
        let manager = VaultManager::new(config).unwrap();

        let path = Path::new("locked_file.md");
        manager.acquire_lock(path, "Agent-Alice", None).await.unwrap();

        // Write with same owner - should succeed
        let result = manager.write_file_with_lock(path, "content", Some("Agent-Alice")).await;
        assert!(result.is_ok());

        // Write with different owner - should fail
        let result = manager.write_file_with_lock(path, "content", Some("Agent-Bob")).await;
        assert!(result.is_err());

        // Write with no owner (anonymous) - should fail if locked
        let result = manager.write_file_with_lock(path, "content", None).await;
        assert!(result.is_err());
    }
}
