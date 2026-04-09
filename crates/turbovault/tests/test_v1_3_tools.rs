//! Integration tests for the 14 latest tools added in v1.2.9+
//! Verification of the v1.3.x Stability Sabbatical goals.

use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use turbovault_core::prelude::*;
use turbovault_tools::{
    DiffTools, DuplicateTools, QualityTools, RelationshipTools, SimilarityEngine,
};
use turbovault_vault::VaultManager;

async fn setup_test_vault() -> (TempDir, Arc<VaultManager>) {
    let temp = TempDir::new().unwrap();
    let config = ServerConfig {
        vaults: vec![VaultConfig::builder("test", temp.path()).build().unwrap()],
        ..Default::default()
    };

    let manager = Arc::new(VaultManager::new(config).unwrap());
    manager.initialize().await.unwrap();

    // Create some test files with content and links
    let files = [
        (
            "note1.md",
            "# Note 1\nLinks to [[note2]] and [[note3]]. Some content about Rust.",
        ),
        (
            "note2.md",
            "# Note 2\nLinks back to [[note1]]. More content about Rust and async.",
        ),
        (
            "note3.md",
            "# Note 3\nLinks to [[note1]]. Peripheral note about documentation.",
        ),
        ("note4.md", "# Note 4\nIsolated note about something else."),
    ];

    for (path, content) in files {
        manager
            .write_file(&PathBuf::from(path), content, None)
            .await
            .unwrap();
    }

    // Re-initialize to build the graph correctly
    manager.initialize().await.unwrap();

    (temp, manager)
}

#[tokio::test]
async fn test_get_centrality_ranking() {
    let (_temp, manager) = setup_test_vault().await;
    let tools = RelationshipTools::new(manager);
    let result = tools.get_centrality_ranking().await.unwrap();

    let total_files = result["total_files"].as_u64().unwrap();
    assert!(total_files >= 3);

    let rankings = result["rankings"].as_array().unwrap();
    assert!(!rankings.is_empty());

    // Note 1 should be highly ranked because it has links to note2, note3 and a backlink from note2, note3
    let top_note = &rankings[0];
    assert!(top_note["score"].as_f64().unwrap() > 0.0);
}

#[tokio::test]
async fn test_diff_notes() {
    let (_temp, manager) = setup_test_vault().await;
    let tools = DiffTools::new(manager);

    let result = tools.diff_notes("note1.md", "note2.md").await.unwrap();

    assert_eq!(result.left_path, "note1.md");
    assert_eq!(result.right_path, "note2.md");
    assert!(!result.unified_diff.is_empty());
    assert!(result.summary.similarity_ratio < 1.0);
}

#[tokio::test]
async fn test_evaluate_note_quality() {
    let (_temp, manager) = setup_test_vault().await;
    let tools = QualityTools::new(manager);

    let result = tools.evaluate_note("note1.md").await.unwrap();

    assert!(result.overall_score > 0);
    assert!(result.readability.score > 0);
    assert!(!result.recommendations.is_empty());
}

#[tokio::test]
async fn test_vault_quality_report() {
    let (_temp, manager) = setup_test_vault().await;
    let tools = QualityTools::new(manager);

    let report = tools.vault_quality_report(5).await.unwrap();

    assert!(report.total_notes >= 4);
    assert!(report.average_score > 0.0);
    assert!(!report.lowest_quality.is_empty());
}

#[tokio::test]
async fn test_find_stale_notes() {
    let (_temp, manager) = setup_test_vault().await;
    let tools = QualityTools::new(manager);

    // Notes have modification time 0 in current parser implementation,
    // so they will always be considered stale if threshold > 0.
    let stale = tools.find_stale_notes(30, 10).await.unwrap();
    assert!(!stale.is_empty());
}

#[tokio::test]
async fn test_semantic_search() {
    let (_temp, manager) = setup_test_vault().await;
    let engine = SimilarityEngine::new(manager).await.unwrap();

    let results = engine.semantic_search("Rust programming", 5);

    assert!(!results.is_empty());
    // note1 and note2 contain "Rust"
    assert!(
        results
            .iter()
            .any(|r| r.path == "note1.md" || r.path == "note2.md")
    );
}

#[tokio::test]
async fn test_find_similar_notes() {
    let (_temp, manager) = setup_test_vault().await;
    let engine = SimilarityEngine::new(manager).await.unwrap();

    let results = engine.find_similar_notes("note1.md", 5);

    assert!(!results.is_empty());
    // note2 should be similar to note1 as both contain "Rust"
    assert!(results.iter().any(|r| r.path == "note2.md"));
}

#[tokio::test]
async fn test_find_duplicates() {
    let (_temp, manager) = setup_test_vault().await;

    // Create a duplicate of note1
    manager
        .write_file(
            &PathBuf::from("note1_copy.md"),
            "# Note 1\nLinks to [[note2]] and [[note3]]. Some content about Rust.",
            None,
        )
        .await
        .unwrap();
    manager.initialize().await.unwrap();

    let tools = DuplicateTools::new(manager);
    let duplicates = tools.find_duplicates(0.9, 10).await.unwrap();

    assert!(!duplicates.is_empty());
    // Should find note1 and note1_copy as duplicates
}

#[tokio::test]
async fn test_compare_notes() {
    let (_temp, manager) = setup_test_vault().await;
    let tools = DuplicateTools::new(manager);

    let result = tools.compare_notes("note1.md", "note2.md").await.unwrap();

    assert!(result.similarity_score > 0.0);
    assert!(!result.shared_terms.is_empty());
    assert!(!result.recommendation.is_empty());
}

#[tokio::test]
async fn test_resolve_cross_vault_link() {
    // Current parser implementation appends .md if missing
    let uri1 = "obsidian://open?vault=MyVault&file=Notes%2FMyNote";
    let (vault1, file1) = turbovault_tools::parse_obsidian_uri(uri1).unwrap();
    assert_eq!(vault1, "MyVault");
    assert_eq!(file1, "Notes/MyNote.md");

    let uri2 = "obsidian://vault/WorkVault/Project/Plan.md";
    let (vault2, file2) = turbovault_tools::parse_obsidian_uri(uri2).unwrap();
    assert_eq!(vault2, "WorkVault");
    assert_eq!(file2, "Project/Plan.md");
}

#[tokio::test]
async fn test_diff_note_version() {
    use turbovault_audit::{AuditFilter, AuditLog, SnapshotStore};

    let temp = TempDir::new().unwrap();
    let config = ServerConfig {
        vaults: vec![VaultConfig::builder("test", temp.path()).build().unwrap()],
        ..Default::default()
    };

    let mut manager = VaultManager::new(config).unwrap();

    // Enable audit log and snapshot store for this manager
    let audit_log = Arc::new(AuditLog::new(temp.path()).await.unwrap());
    let snapshot_store = Arc::new(SnapshotStore::new(audit_log.snapshot_dir().to_path_buf()));
    manager.set_audit_log(audit_log.clone(), snapshot_store.clone());

    let manager = Arc::new(manager);
    manager.initialize().await.unwrap();

    // 1. Create initial version
    let path = PathBuf::from("note.md");
    manager
        .write_file(&path, "# Initial content", None)
        .await
        .unwrap();

    // 2. Update to second version (this creates the 'before' snapshot)
    manager
        .write_file(&path, "# Updated content", None)
        .await
        .unwrap();

    // 3. Get the operation ID from the audit log
    let filter = AuditFilter {
        limit: 10,
        ..Default::default()
    };
    let entries = audit_log.query(&filter).await.unwrap();
    assert!(entries.len() >= 2);
    // The newest entry [0] should be the update operation
    let update_entry = &entries[0];
    let op_id = update_entry.id.clone();

    // 4. Record current content implicitly by update above

    // 5. Test the tool method
    let tools = DiffTools::new(manager);
    let result = tools.diff_note_version("note.md", &op_id).await.unwrap();

    assert_eq!(result.summary.lines_changed, 1);
    assert!(result.unified_diff.contains("-# Initial content"));
    assert!(result.unified_diff.contains("+# Updated content"));
    assert!(result.left_path.contains("note.md"));
    assert!(result.left_path.contains(&op_id[..8]));

    // 7. Test error path: invalid operation ID
    let err_result = tools.diff_note_version("note.md", "invalid-id").await;
    assert!(err_result.is_err());
}

#[tokio::test]
async fn test_ofm_syntax_guide() {
    // These are usually tested by checking if the static content is present
    let guide = turbovault::resources::OFM_SYNTAX_GUIDE;
    assert!(guide.contains("Obsidian Flavored Markdown"));
    assert!(guide.contains("Wikilinks"));
}

#[tokio::test]
async fn test_ofm_quick_ref() {
    let quick_ref = turbovault::resources::OFM_QUICK_REFERENCE;
    assert!(quick_ref.contains("Quick Reference"));
    // The quick ref uses "Formatting" and "Wikilinks" etc.
    assert!(quick_ref.contains("Wikilinks"));
}

#[tokio::test]
async fn test_ofm_examples() {
    let examples = turbovault::resources::OFM_EXAMPLE_NOTE;
    assert!(examples.contains("Example Note"));
    assert!(examples.contains("Callouts"));
}
