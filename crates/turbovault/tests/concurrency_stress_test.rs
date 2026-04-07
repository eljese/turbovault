//! Concurrency stress tests for turbovault

use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::task;
use turbovault_core::prelude::*;
use turbovault_vault::{VaultManager, compute_hash};

#[tokio::test]
async fn test_concurrent_writes_with_expected_hash() {
    let temp = TempDir::new().unwrap();
    let config = ServerConfig {
        vaults: vec![VaultConfig::builder("test", temp.path()).build().unwrap()],
        ..Default::default()
    };

    let manager = Arc::new(VaultManager::new(config).unwrap());
    manager.initialize().await.unwrap();

    let path = PathBuf::from("concurrency.md");
    let initial_content = "Initial content\n";
    manager
        .write_file(&path, initial_content, None)
        .await
        .unwrap();

    let num_agents: usize = 10;
    let mut handles = vec![];

    for i in 0..num_agents {
        let m = manager.clone();
        let p = path.clone();
        handles.push(task::spawn(async move {
            let mut attempts = 0;
            loop {
                attempts += 1;
                // Read current content and hash
                let content: String = m.read_file(&p).await.unwrap();
                let hash = compute_hash(&content);

                // Append agent's ID
                let new_content = format!("{}Agent {}\n", content, i);

                // Try to write with expected_hash
                match m.write_file(&p, &new_content, Some(&hash)).await {
                    Ok(_) => {
                        return (i, true, attempts);
                    }
                    Err(e) => {
                        if let Error::ConcurrencyError { .. } = e {
                            // Concurrency error, wait a bit and retry
                            tokio::time::sleep(tokio::time::Duration::from_millis(5 * (i as u64)))
                                .await;
                            if attempts > 200 {
                                return (i, false, attempts);
                            }
                            continue;
                        } else {
                            // Unexpected error
                            panic!("Agent {} failed with unexpected error: {:?}", i, e);
                        }
                    }
                }
            }
        }));
    }

    let mut results = vec![];
    for handle in handles {
        results.push(handle.await.unwrap());
    }

    // Verify all agents eventually succeeded
    for (id, success, attempts) in &results {
        assert!(
            *success,
            "Agent {} failed to write after {} attempts",
            id, attempts
        );
    }

    // Verify final content contains all agents' IDs and no data loss
    let final_content: String = manager.read_file(&path).await.unwrap();
    for i in 0..num_agents {
        let expected = format!("Agent {}\n", i);
        assert!(
            final_content.contains(&expected),
            "Final content missing Agent {} output",
            i
        );
    }

    // Count lines. Initial + num_agents
    let lines: Vec<&str> = final_content.lines().collect();
    assert_eq!(
        lines.len(),
        num_agents + 1,
        "Final content has incorrect number of lines"
    );
}
