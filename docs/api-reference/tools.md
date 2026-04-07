# MCP Tools Reference

Complete reference of all 58 MCP tools available in TurboVault v1.3.x.

## File Operations (7 tools)

- `read_full_note` - Read entire note content with hash
- `write_note` - Create or update notes (atomic)
- `edit_note` - Apply targeted SEARCH/REPLACE edits
- `delete_note` - Remove notes (confirmation protected)
- `move_note` - Rename or move notes
- `move_file` - Move attachments (images, PDFs)
- `get_notes_info` - Batch metadata retrieval (size, mtime)

## Search & Discovery (6 tools)

- `search_vault_summaries` - Fast discovery search (200 char snippets)
- `advanced_search` - Filtered search (tags, metadata, dates)
- `semantic_search` - Conceptual similarity search (TF-IDF)
- `find_similar_notes` - Content-based similarity discovery
- `recommend_related` - ML-powered link recommendations
- `query_metadata` - SQL-like frontmatter querying

## Graph Analysis (9 tools)

- `get_backlinks` - Incoming link analysis
- `get_forward_links` - Outgoing link analysis
- `get_related_notes` - N-hop neighborhood discovery
- `get_hub_notes` - Centrality analysis (degree)
- `get_dead_end_notes` - Orphan and leaf note detection
- `get_isolated_clusters` - Community detection
- `get_link_strength` - Connection weight calculation
- `suggest_links` - AI-powered link suggestion
- `get_centrality_ranking` - Advanced centrality (betweenness, closeness)

## Health & Validation (5 tools)

- `quick_health_check` - 0-100 vault health score
- `full_health_analysis` - Comprehensive diagnostic report
- `get_broken_links` - Broken link detection with fix suggestions
- `detect_cycles` - Circular reference detection
- `explain_vault` - Holistic LLM-friendly vault summary

## Template System (4 tools)

- `list_templates` - Available vault templates
- `get_template` - Template definition and schema
- `create_from_template` - Generate notes from templates
- `find_notes_from_template` - Reverse template lookup

## Vault Management (7 tools)

- `list_vaults` - Registered vaults
- `add_vault` - Register existing vault
- `create_vault` - Scaffold new vault
- `remove_vault` - Unregister vault
- `get_active_vault` - Get current context
- `set_active_vault` - Switch active vault
- `get_vault_config` - View vault settings

## Export & Reporting (4 tools)

- `export_health_report` - Structured health data (JSON/CSV)
- `export_broken_links` - Broken link report
- `export_vault_stats` - Aggregated vault metrics
- `export_analysis_report` - Combined holistic report

## Comparison & Diff (4 tools)

- `diff_notes` - Side-by-side note comparison
- `diff_note_version` - Compare current note with audit version
- `compare_notes` - Similarity and merge analysis
- `find_duplicates` - SimHash duplicate detection

## Quality Analysis (3 tools)

- `evaluate_note_quality` - Single note readability and structure
- `vault_quality_report` - Aggregate quality metrics
- `find_stale_notes` - Neglected content detection

## Knowledge & Utility (9 tools)

- `batch_execute` - Atomic multi-operation transactions
- `get_metadata_value` - Deep property extraction
- `update_frontmatter` - Atomic property updates
- `manage_tags` - Batch tag management
- `get_link_strength` - (Re-listed)
- `resolve_cross_vault_link` - Multi-vault link resolution
- `get_ofm_syntax_guide` - Complete OFM reference
- `get_ofm_quick_ref` - OFM cheat sheet
- `get_ofm_examples` - OFM pattern library

See the [Documentation Guide](../DOCUMENTATION_GUIDE.md) for more details.
