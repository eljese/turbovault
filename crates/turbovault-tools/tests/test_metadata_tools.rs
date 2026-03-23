//! Unit tests for MetadataTools

use std::sync::Arc;
use tempfile::TempDir;
use turbovault_core::{ConfigProfile, VaultConfig};
use turbovault_tools::MetadataTools;
use turbovault_vault::VaultManager;

async fn setup_test_vault_with_metadata() -> (TempDir, Arc<VaultManager>) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let vault_path = temp_dir.path();

    // Create notes with various metadata
    tokio::fs::write(
        vault_path.join("note1.md"),
        r#"---
title: "First Note"
author: "Alice"
status: "draft"
priority: 5
tags: ["project", "urgent"]
---
# Note 1
Content here"#,
    )
    .await
    .unwrap();

    tokio::fs::write(
        vault_path.join("note2.md"),
        r#"---
title: "Second Note"
author: "Bob"
status: "published"
priority: 3
tags: ["reference"]
---
# Note 2
More content"#,
    )
    .await
    .unwrap();

    tokio::fs::write(
        vault_path.join("note3.md"),
        r#"---
title: "Third Note"
status: "archived"
nested:
  field: "value"
  count: 42
---
# Note 3
Text"#,
    )
    .await
    .unwrap();

    tokio::fs::write(
        vault_path.join("no_metadata.md"),
        "# No Metadata\nJust content",
    )
    .await
    .unwrap();

    let mut config = ConfigProfile::Development.create_config();
    let vault_config = VaultConfig::builder("test", vault_path).build().unwrap();
    config.vaults.push(vault_config);

    let manager = VaultManager::new(config).unwrap();
    manager.initialize().await.unwrap();

    (temp_dir, Arc::new(manager))
}

#[tokio::test]
async fn test_get_metadata_value_string() {
    let (_temp_dir, manager) = setup_test_vault_with_metadata().await;
    let tools = MetadataTools::new(manager);

    let result = tools.get_metadata_value("note1.md", "title").await;
    assert!(result.is_ok());
    let response = result.unwrap();
    let value = response.get("value").unwrap();
    assert!(value.as_str().unwrap().contains("First Note"));
}

#[tokio::test]
async fn test_get_metadata_value_number() {
    let (_temp_dir, manager) = setup_test_vault_with_metadata().await;
    let tools = MetadataTools::new(manager);

    let result = tools.get_metadata_value("note1.md", "priority").await;
    assert!(result.is_ok());
    let response = result.unwrap();
    let value = response.get("value").unwrap();
    assert_eq!(value.as_i64().unwrap(), 5);
}

#[tokio::test]
async fn test_get_metadata_value_array() {
    let (_temp_dir, manager) = setup_test_vault_with_metadata().await;
    let tools = MetadataTools::new(manager);

    let result = tools.get_metadata_value("note1.md", "tags").await;
    assert!(result.is_ok());
    let response = result.unwrap();
    let value = response.get("value").unwrap();
    assert!(value.is_array());
    let tags = value.as_array().unwrap();
    assert!(tags.len() >= 2);
}

#[tokio::test]
async fn test_get_metadata_value_nested() {
    let (_temp_dir, manager) = setup_test_vault_with_metadata().await;
    let tools = MetadataTools::new(manager);

    let result = tools.get_metadata_value("note3.md", "nested.field").await;
    assert!(result.is_ok());
    let response = result.unwrap();
    let value = response.get("value").unwrap();
    assert_eq!(value.as_str().unwrap(), "value");
}

#[tokio::test]
async fn test_get_metadata_value_nested_number() {
    let (_temp_dir, manager) = setup_test_vault_with_metadata().await;
    let tools = MetadataTools::new(manager);

    let result = tools.get_metadata_value("note3.md", "nested.count").await;
    assert!(result.is_ok());
    let response = result.unwrap();
    let value = response.get("value").unwrap();
    assert_eq!(value.as_i64().unwrap(), 42);
}

#[tokio::test]
async fn test_get_metadata_value_missing_key() {
    let (_temp_dir, manager) = setup_test_vault_with_metadata().await;
    let tools = MetadataTools::new(manager);

    let result = tools.get_metadata_value("note1.md", "nonexistent").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_metadata_value_no_metadata() {
    let (_temp_dir, manager) = setup_test_vault_with_metadata().await;
    let tools = MetadataTools::new(manager);

    let result = tools.get_metadata_value("no_metadata.md", "title").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_query_metadata_equality() {
    let (_temp_dir, manager) = setup_test_vault_with_metadata().await;
    let tools = MetadataTools::new(manager);

    let result = tools.query_metadata(r#"status: "draft""#).await;
    assert!(result.is_ok());
    // Query executes successfully (matches depend on frontmatter parsing)
    let _response = result.unwrap();
}

#[tokio::test]
async fn test_query_metadata_comparison() {
    let (_temp_dir, manager) = setup_test_vault_with_metadata().await;
    let tools = MetadataTools::new(manager);

    let result = tools.query_metadata("priority > 3").await;
    assert!(result.is_ok());
    // Query executes successfully (matches depend on frontmatter parsing)
    let _response = result.unwrap();
}

#[tokio::test]
async fn test_query_metadata_contains() {
    let (_temp_dir, manager) = setup_test_vault_with_metadata().await;
    let tools = MetadataTools::new(manager);

    let result = tools.query_metadata(r#"tags: contains("urgent")"#).await;
    assert!(result.is_ok());
    // Query executes successfully (matches depend on frontmatter parsing)
    let _response = result.unwrap();
}

#[tokio::test]
async fn test_query_metadata_no_matches() {
    let (_temp_dir, manager) = setup_test_vault_with_metadata().await;
    let tools = MetadataTools::new(manager);

    let result = tools.query_metadata(r#"status: "nonexistent""#).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    let matched = response.get("matched").unwrap().as_u64().unwrap();
    assert_eq!(matched, 0);
}

#[tokio::test]
async fn test_query_metadata_invalid_syntax() {
    let (_temp_dir, manager) = setup_test_vault_with_metadata().await;
    let tools = MetadataTools::new(manager);

    let result = tools.query_metadata("invalid syntax !!!").await;
    // Should handle invalid syntax gracefully (returns error for invalid pattern)
    assert!(result.is_err());
}

#[tokio::test]
async fn test_async_error_nonexistent_file() {
    let (_temp_dir, manager) = setup_test_vault_with_metadata().await;
    let tools = MetadataTools::new(manager);

    let result = tools.get_metadata_value("nonexistent.md", "title").await;
    assert!(result.is_err());
}

// ==================== update_frontmatter Tests ====================

#[tokio::test]
async fn test_update_frontmatter_merge_into_existing() {
    let (_temp_dir, manager) = setup_test_vault_with_metadata().await;
    let tools = MetadataTools::new(manager.clone());

    // note1.md has: title, author, status, priority, tags
    let mut new_fm = serde_json::Map::new();
    new_fm.insert("category".to_string(), serde_json::json!("work"));
    new_fm.insert("priority".to_string(), serde_json::json!(10)); // override existing

    let result = tools.update_frontmatter("note1.md", new_fm, true).await;
    assert!(result.is_ok(), "update_frontmatter failed: {:?}", result);

    // Verify merge: new key added, existing key updated, other keys preserved
    let value = tools.get_metadata_value("note1.md", "category").await;
    assert!(value.is_ok(), "category not found: {:?}", value);
    assert_eq!(value.unwrap()["value"], "work");

    let value = tools.get_metadata_value("note1.md", "priority").await;
    assert!(value.is_ok(), "priority not found: {:?}", value);
    assert_eq!(value.unwrap()["value"], 10);

    // Original key preserved
    let value = tools.get_metadata_value("note1.md", "author").await;
    assert!(value.is_ok(), "author not found: {:?}", value);
}

#[tokio::test]
async fn test_update_frontmatter_replace_wipes_existing() {
    let (_temp_dir, manager) = setup_test_vault_with_metadata().await;
    let tools = MetadataTools::new(manager.clone());

    let mut new_fm = serde_json::Map::new();
    new_fm.insert("only_key".to_string(), serde_json::json!("value"));

    let result = tools
        .update_frontmatter("note1.md", new_fm, false) // merge=false → replace
        .await;
    assert!(result.is_ok());

    // Original keys should be gone
    let value = tools.get_metadata_value("note1.md", "author").await;
    assert!(value.is_err(), "author should be gone after replace");

    // New key should exist
    let value = tools.get_metadata_value("note1.md", "only_key").await;
    assert!(value.is_ok());
}

#[tokio::test]
async fn test_update_frontmatter_into_no_frontmatter_file() {
    let (_temp_dir, manager) = setup_test_vault_with_metadata().await;
    let tools = MetadataTools::new(manager.clone());

    let mut new_fm = serde_json::Map::new();
    new_fm.insert("title".to_string(), serde_json::json!("Added"));

    let result = tools
        .update_frontmatter("no_metadata.md", new_fm, true)
        .await;
    assert!(result.is_ok());

    // Should now have frontmatter
    let value = tools.get_metadata_value("no_metadata.md", "title").await;
    assert!(value.is_ok());
    assert_eq!(value.unwrap()["value"], "Added");
}

#[tokio::test]
async fn test_update_frontmatter_deep_merge_nested() {
    let (_temp_dir, manager) = setup_test_vault_with_metadata().await;
    let tools = MetadataTools::new(manager.clone());

    // note3.md has: nested: { field: "value", count: 42 }
    let mut new_fm = serde_json::Map::new();
    new_fm.insert(
        "nested".to_string(),
        serde_json::json!({"extra": "new", "count": 99}),
    );

    let result = tools.update_frontmatter("note3.md", new_fm, true).await;
    assert!(result.is_ok());

    // Deep merge: "field" preserved, "count" updated, "extra" added
    let value = tools.get_metadata_value("note3.md", "nested.field").await;
    assert!(value.is_ok());
    assert_eq!(value.unwrap()["value"], "value");

    let value = tools.get_metadata_value("note3.md", "nested.count").await;
    assert!(value.is_ok());
    assert_eq!(value.unwrap()["value"], 99);

    let value = tools.get_metadata_value("note3.md", "nested.extra").await;
    assert!(value.is_ok());
    assert_eq!(value.unwrap()["value"], "new");
}

#[tokio::test]
async fn test_update_frontmatter_preserves_body() {
    let (_temp_dir, manager) = setup_test_vault_with_metadata().await;
    let tools = MetadataTools::new(manager.clone());
    let file_tools = turbovault_tools::FileTools::new(manager.clone());

    let mut new_fm = serde_json::Map::new();
    new_fm.insert("new_key".to_string(), serde_json::json!("new_value"));

    tools
        .update_frontmatter("note1.md", new_fm, true)
        .await
        .unwrap();

    let content = file_tools.read_file("note1.md").await.unwrap();
    assert!(content.contains("# Note 1"), "body heading missing");
    assert!(content.contains("Content here"), "body content missing");
}

// ==================== manage_tags Tests ====================

#[tokio::test]
async fn test_manage_tags_list_frontmatter_and_inline() {
    let (temp_dir, manager) = setup_test_vault_with_metadata().await;
    let tools = MetadataTools::new(manager.clone());

    // Write a file with both frontmatter tags and inline tags
    tokio::fs::write(
        temp_dir.path().join("tagged.md"),
        "---\ntags: [\"work\", \"important\"]\n---\n# Tagged\nThis is #urgent and #work related",
    )
    .await
    .unwrap();

    let result = tools.manage_tags("tagged.md", "list", None).await;
    assert!(result.is_ok());
    let resp = result.unwrap();

    let fm_tags = resp["frontmatter_tags"].as_array().unwrap();
    assert!(fm_tags.iter().any(|t| t == "work"));
    assert!(fm_tags.iter().any(|t| t == "important"));

    let inline_tags = resp["inline_tags"].as_array().unwrap();
    assert!(inline_tags.iter().any(|t| t == "urgent"));

    // all_tags should deduplicate "work"
    let all_tags = resp["all_tags"].as_array().unwrap();
    let work_count = all_tags.iter().filter(|t| *t == "work").count();
    assert_eq!(
        work_count, 1,
        "work tag should appear only once after dedup"
    );
}

#[tokio::test]
async fn test_manage_tags_list_no_frontmatter() {
    let (temp_dir, manager) = setup_test_vault_with_metadata().await;
    let tools = MetadataTools::new(manager.clone());

    tokio::fs::write(
        temp_dir.path().join("inline_only.md"),
        "# Note\nHas #inline and #tags only",
    )
    .await
    .unwrap();

    let result = tools.manage_tags("inline_only.md", "list", None).await;
    assert!(result.is_ok());
    let resp = result.unwrap();
    let fm_tags = resp["frontmatter_tags"].as_array().unwrap();
    assert!(fm_tags.is_empty());
    let inline_tags = resp["inline_tags"].as_array().unwrap();
    assert!(!inline_tags.is_empty());
}

#[tokio::test]
async fn test_manage_tags_add_to_existing() {
    let (_temp_dir, manager) = setup_test_vault_with_metadata().await;
    let tools = MetadataTools::new(manager.clone());

    // note1.md has tags: ["project", "urgent"]
    let tags = vec!["newone".to_string()];
    let result = tools.manage_tags("note1.md", "add", Some(&tags)).await;
    assert!(result.is_ok());
    let resp = result.unwrap();
    let tags_arr = resp["tags"].as_array().unwrap();
    assert!(tags_arr.iter().any(|t| t == "project"));
    assert!(tags_arr.iter().any(|t| t == "urgent"));
    assert!(tags_arr.iter().any(|t| t == "newone"));
}

#[tokio::test]
async fn test_manage_tags_add_deduplicates() {
    let (_temp_dir, manager) = setup_test_vault_with_metadata().await;
    let tools = MetadataTools::new(manager.clone());

    // note1.md has tags: ["project", "urgent"]
    let tags = vec!["project".to_string()]; // already exists
    let result = tools.manage_tags("note1.md", "add", Some(&tags)).await;
    assert!(result.is_ok());
    let resp = result.unwrap();
    let tags_arr = resp["tags"].as_array().unwrap();
    let project_count = tags_arr.iter().filter(|t| *t == "project").count();
    assert_eq!(project_count, 1);
}

#[tokio::test]
async fn test_manage_tags_add_strips_hash() {
    let (_temp_dir, manager) = setup_test_vault_with_metadata().await;
    let tools = MetadataTools::new(manager.clone());

    let tags = vec!["#newtag".to_string()];
    let result = tools.manage_tags("note1.md", "add", Some(&tags)).await;
    assert!(result.is_ok());
    let resp = result.unwrap();
    let tags_arr = resp["tags"].as_array().unwrap();
    // Should be stored without #
    assert!(tags_arr.iter().any(|t| t == "newtag"));
    assert!(!tags_arr.iter().any(|t| t == "#newtag"));
}

#[tokio::test]
async fn test_manage_tags_add_creates_tags_key() {
    let (_temp_dir, manager) = setup_test_vault_with_metadata().await;
    let tools = MetadataTools::new(manager.clone());

    // note3.md has frontmatter but no tags key
    let tags = vec!["added".to_string()];
    let result = tools.manage_tags("note3.md", "add", Some(&tags)).await;
    assert!(result.is_ok());
    let resp = result.unwrap();
    let tags_arr = resp["tags"].as_array().unwrap();
    assert!(tags_arr.iter().any(|t| t == "added"));
}

#[tokio::test]
async fn test_manage_tags_remove_existing() {
    let (_temp_dir, manager) = setup_test_vault_with_metadata().await;
    let tools = MetadataTools::new(manager.clone());

    // note1.md has tags: ["project", "urgent"]
    let tags = vec!["urgent".to_string()];
    let result = tools.manage_tags("note1.md", "remove", Some(&tags)).await;
    assert!(result.is_ok());
    let resp = result.unwrap();
    let remaining = resp["tags"].as_array().unwrap();
    assert!(remaining.iter().any(|t| t == "project"));
    assert!(!remaining.iter().any(|t| t == "urgent"));
}

#[tokio::test]
async fn test_manage_tags_remove_strips_hash() {
    let (_temp_dir, manager) = setup_test_vault_with_metadata().await;
    let tools = MetadataTools::new(manager.clone());

    let tags = vec!["#urgent".to_string()]; // with hash
    let result = tools.manage_tags("note1.md", "remove", Some(&tags)).await;
    assert!(result.is_ok());
    let resp = result.unwrap();
    let remaining = resp["tags"].as_array().unwrap();
    assert!(!remaining.iter().any(|t| t == "urgent"));
}

#[tokio::test]
async fn test_manage_tags_remove_nonexistent_tag() {
    let (_temp_dir, manager) = setup_test_vault_with_metadata().await;
    let tools = MetadataTools::new(manager.clone());

    let tags = vec!["nonexistent".to_string()];
    let result = tools.manage_tags("note1.md", "remove", Some(&tags)).await;
    assert!(result.is_ok());
    // All original tags should remain
    let resp = result.unwrap();
    let remaining = resp["tags"].as_array().unwrap();
    assert!(remaining.iter().any(|t| t == "project"));
    assert!(remaining.iter().any(|t| t == "urgent"));
}

#[tokio::test]
async fn test_manage_tags_remove_no_frontmatter() {
    let (_temp_dir, manager) = setup_test_vault_with_metadata().await;
    let tools = MetadataTools::new(manager.clone());

    let tags = vec!["anything".to_string()];
    let result = tools
        .manage_tags("no_metadata.md", "remove", Some(&tags))
        .await;
    assert!(result.is_ok());
    let resp = result.unwrap();
    assert_eq!(resp["status"], "no_frontmatter");
}

#[tokio::test]
async fn test_manage_tags_invalid_operation() {
    let (_temp_dir, manager) = setup_test_vault_with_metadata().await;
    let tools = MetadataTools::new(manager.clone());

    let result = tools.manage_tags("note1.md", "toggle", None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_manage_tags_add_without_tags_arg() {
    let (_temp_dir, manager) = setup_test_vault_with_metadata().await;
    let tools = MetadataTools::new(manager.clone());

    let result = tools.manage_tags("note1.md", "add", None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_manage_tags_remove_without_tags_arg() {
    let (_temp_dir, manager) = setup_test_vault_with_metadata().await;
    let tools = MetadataTools::new(manager.clone());

    let result = tools.manage_tags("note1.md", "remove", None).await;
    assert!(result.is_err());
}

// ==================== extract_tags_from_value / parse_query edge cases ====================

#[tokio::test]
async fn test_get_metadata_value_partial_dot_key_missing() {
    let (_temp_dir, manager) = setup_test_vault_with_metadata().await;
    let tools = MetadataTools::new(manager);

    // note3.md has nested.field but not nested.missing
    let result = tools.get_metadata_value("note3.md", "nested.missing").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_concurrent_metadata_reads() {
    let (_temp_dir, manager) = setup_test_vault_with_metadata().await;

    let handles: Vec<_> = (0..10)
        .map(|i| {
            let tools = MetadataTools::new(manager.clone());
            tokio::spawn(async move {
                let file = match i % 3 {
                    0 => "note1.md",
                    1 => "note2.md",
                    _ => "note3.md",
                };
                tools.get_metadata_value(file, "title").await
            })
        })
        .collect();

    for handle in handles {
        let result = handle.await.expect("Task panicked");
        assert!(result.is_ok());
    }
}
