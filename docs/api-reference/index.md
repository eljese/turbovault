# API Reference

Complete reference for all 58 MCP tools available to AI agents.

## Tool Categories

### File Operations (7 tools)
- `read_full_note` - Read note content with hash
- `write_note` - Write/create note (atomic)
- `edit_note` - Targeted edits (SEARCH/REPLACE)
- `delete_note` - Delete note
- `move_note` - Move/rename note
- `move_file` - Move attachments
- `get_notes_info` - Batch metadata

### Search & Discovery (6 tools)
- `search_vault_summaries` - Fast discovery search
- `advanced_search` - Search with filters
- `semantic_search` - Conceptual similarity (TF-IDF)
- `find_similar_notes` - Content similarity
- `recommend_related` - Get recommendations
- `query_metadata` - Query frontmatter

### Link & Graph Analysis (9 tools)
- `get_backlinks` - Find incoming links
- `get_forward_links` - Find outgoing links
- `get_related_notes` - Find notes within N hops
- `get_hub_notes` - Find highly connected notes
- `get_dead_end_notes` - Find leaf notes
- `get_isolated_clusters` - Find isolated groups
- `get_link_strength` - Calculate connection weight
- `suggest_links` - AI-powered link suggestions
- `get_centrality_ranking` - Advanced centrality metrics

### Health & Validation (5 tools)
- `quick_health_check` - Fast health score
- `full_health_analysis` - Comprehensive report
- `get_broken_links` - Find broken links
- `detect_cycles` - Find circular references
- `explain_vault` - Holistic vault summary

### Template System (4 tools)
- `list_templates` - List available templates
- `get_template` - Get template details
- `create_from_template` - Create note from template
- `find_notes_from_template` - Find notes from template

### Batch Operations (1 tool)
- `batch_execute` - Atomic multi-operation transactions

### Export & Reporting (4 tools)
- `export_health_report` - Export health metrics
- `export_broken_links` - Export broken links
- `export_vault_stats` - Export vault statistics
- `export_analysis_report` - Export holistic analysis

### Comparison & Diff (4 tools)
- `diff_notes` - Side-by-side comparison
- `diff_note_version` - Compare with audit version
- `compare_notes` - Similarity and merge analysis
- `find_duplicates` - Duplicate detection (SimHash)

### Quality Analysis (3 tools)
- `evaluate_note_quality` - Individual note quality
- `vault_quality_report` - Aggregate quality report
- `find_stale_notes` - Neglected content detection

### Vault Lifecycle (7 tools)
- `create_vault` - Create new vault
- `add_vault` - Add existing vault
- `list_vaults` - List all vaults
- `get_active_vault` - Get current vault
- `set_active_vault` - Switch vault
- `remove_vault` - Unregister vault
- `get_vault_config` - View configuration

### Utility & Knowledge (4 tools)
- `get_metadata_value` - Property extraction
- `update_frontmatter` - Property updates
- `resolve_cross_vault_link` - Multi-vault links
- `get_ofm_syntax_guide` - OFM Reference

## Example Workflows

### Semantic Discovery
```python
# Find notes conceptually related to a topic
results = semantic_search("distributed systems architecture")

# Find similar notes to a specific finding
similar = find_similar_notes(results[0].path)
```

### Quality Audit
```python
# Generate vault quality report
report = vault_quality_report(bottom_n=10)

# Evaluate specific note and get recommendations
quality = evaluate_note_quality(report.lowest_quality[0].path)
```

### Multi-Vault Navigation
```python
# Encountered a cross-vault link in content
uri = "obsidian://vault/Secondary/Project/Plan"
target = resolve_cross_vault_link(uri)

# Switch context and read
set_active_vault(target.target_vault)
content = read_full_note(target.target_file)
```

## Data Types

### QualityScore
```rust
{
    path: String,
    overall_score: u8,         // 0-100
    readability: ReadabilityScore,
    structure: StructureScore,
    completeness: CompletenessScore,
    staleness: StalenessScore,
    recommendations: Vec<String>,
}
```

### DiffResult
```rust
{
    left_path: String,
    right_path: String,
    unified_diff: String,      // Standard diff format
    summary: {
        lines_added: usize,
        lines_removed: usize,
        similarity_ratio: f64, // 0.0-1.0
    }
}
```

See the full [MCP Tools Reference](./tools.md) for parameter details.
