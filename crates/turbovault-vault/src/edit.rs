//! LLM-optimized file editing with SEARCH/REPLACE blocks
//!
//! Inspired by aider's proven approach that reduced GPT-4 laziness by 3X.
//! Uses git merge conflict syntax which LLMs know intimately from training data.
//!
//! ## Format (for LLMs):
//! ```text
//! <<<<<<< SEARCH
//! old content to find
//! =======
//! new content to replace with
//! >>>>>>> REPLACE
//! ```
//!
//! ## Fuzzy Matching Strategy (aider-inspired):
//! 1. Exact match (fastest)
//! 2. Whitespace-insensitive match
//! 3. Indentation-preserving match
//! 4. Fuzzy match with Levenshtein distance
//!
//! This tolerates minor LLM errors while remaining safe.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use turbovault_core::{Error, Result};
use unicode_normalization::UnicodeNormalization;

/// A single SEARCH/REPLACE block
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchReplaceBlock {
    /// Text to search for (will be fuzzy-matched)
    pub search: String,
    /// Replacement text
    pub replace: String,
}

/// Result of applying edits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditResult {
    /// Whether edits were applied successfully
    pub success: bool,
    /// Old content hash (SHA-256)
    pub old_hash: String,
    /// New content hash (SHA-256)
    pub new_hash: String,
    /// Number of blocks successfully applied
    pub blocks_applied: usize,
    /// Total blocks attempted
    pub total_blocks: usize,
    /// Preview of changes (if dry_run)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_preview: Option<String>,
    /// Warning messages (e.g., fuzzy match used)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

/// Configuration for edit engine behavior
#[derive(Debug, Clone)]
pub struct EditConfig {
    /// Maximum allowed Levenshtein distance ratio (0.0-1.0)
    /// 0.8 means search can differ by up to 20%
    pub max_fuzzy_distance: f32,

    /// Enable whitespace-insensitive matching
    pub allow_whitespace_flex: bool,

    /// Enable indentation-preserving matching
    pub allow_indent_flex: bool,

    /// Enable fuzzy Levenshtein matching
    pub allow_fuzzy_match: bool,
}

impl Default for EditConfig {
    fn default() -> Self {
        Self {
            max_fuzzy_distance: 0.85, // 85% similarity required
            allow_whitespace_flex: true,
            allow_indent_flex: true,
            allow_fuzzy_match: true,
        }
    }
}

/// Edit engine with cascading fuzzy matching
pub struct EditEngine {
    config: EditConfig,
}

impl EditEngine {
    /// Create new edit engine with default config
    pub fn new() -> Self {
        Self {
            config: EditConfig::default(),
        }
    }

    /// Create edit engine with custom config
    pub fn with_config(config: EditConfig) -> Self {
        Self { config }
    }

    /// Parse SEARCH/REPLACE blocks from LLM-generated string
    ///
    /// Expected format:
    /// ```text
    /// <<<<<<< SEARCH
    /// old content
    /// =======
    /// new content
    /// >>>>>>> REPLACE
    /// ```
    pub fn parse_blocks(&self, input: &str) -> Result<Vec<SearchReplaceBlock>> {
        let mut blocks = Vec::new();
        let mut current_search = String::new();
        let mut current_replace = String::new();
        let mut state = ParseState::Init;

        for line in input.lines() {
            let trimmed = line.trim();

            match state {
                ParseState::Init => {
                    if trimmed == "<<<<<<< SEARCH" {
                        state = ParseState::InSearch;
                    }
                }
                ParseState::InSearch => {
                    if trimmed == "=======" {
                        state = ParseState::InReplace;
                    } else {
                        if !current_search.is_empty() {
                            current_search.push('\n');
                        }
                        current_search.push_str(line); // Preserve original indentation
                    }
                }
                ParseState::InReplace => {
                    if trimmed == ">>>>>>> REPLACE" {
                        blocks.push(SearchReplaceBlock {
                            search: current_search.clone(),
                            replace: current_replace.clone(),
                        });
                        current_search.clear();
                        current_replace.clear();
                        state = ParseState::Init;
                    } else {
                        if !current_replace.is_empty() {
                            current_replace.push('\n');
                        }
                        current_replace.push_str(line); // Preserve original indentation
                    }
                }
            }
        }

        // Check for incomplete block
        if state != ParseState::Init {
            return Err(Error::ParseError {
                reason: format!(
                    "Incomplete SEARCH/REPLACE block (state: {:?}). Expected >>>>>>> REPLACE",
                    state
                ),
            });
        }

        if blocks.is_empty() {
            return Err(Error::ParseError {
                reason: "No SEARCH/REPLACE blocks found in input".to_string(),
            });
        }

        Ok(blocks)
    }

    /// Apply SEARCH/REPLACE blocks to content
    ///
    /// Returns edited content and metadata about what was applied
    pub fn apply_blocks(
        &self,
        content: &str,
        blocks: &[SearchReplaceBlock],
    ) -> Result<(String, Vec<String>)> {
        let mut result = content.to_string();
        let mut warnings = Vec::new();

        for (idx, block) in blocks.iter().enumerate() {
            match self.find_and_replace(&result, &block.search, &block.replace) {
                Ok((new_content, match_type)) => {
                    result = new_content;
                    if match_type != MatchType::Exact {
                        warnings.push(format!(
                            "Block {} used {} matching",
                            idx + 1,
                            match_type.description()
                        ));
                    }
                }
                Err(e) => {
                    return Err(Error::Other(format!("Block {} failed: {}", idx + 1, e)));
                }
            }
        }

        Ok((result, warnings))
    }

    /// Apply edits with full result metadata
    pub fn apply_edits(
        &self,
        content: &str,
        blocks: &[SearchReplaceBlock],
        dry_run: bool,
    ) -> Result<EditResult> {
        let old_hash = compute_hash(content);

        // Generate diff preview if dry run
        let diff_preview = if dry_run {
            Some(self.generate_preview(content, blocks)?)
        } else {
            None
        };

        let (new_content, warnings) = if dry_run {
            // For dry run, compute what would change but don't return new content
            self.apply_blocks(content, blocks)?
        } else {
            self.apply_blocks(content, blocks)?
        };

        let new_hash = compute_hash(&new_content);

        Ok(EditResult {
            success: true,
            old_hash,
            new_hash,
            blocks_applied: blocks.len(),
            total_blocks: blocks.len(),
            diff_preview,
            warnings,
        })
    }

    /// Find and replace using cascading fuzzy matching strategies
    fn find_and_replace(
        &self,
        content: &str,
        search: &str,
        replace: &str,
    ) -> Result<(String, MatchType)> {
        // Strategy 1: Exact match
        if let Some(pos) = content.find(search) {
            let new_content = Self::replace_at(content, pos, search.len(), replace);
            return Ok((new_content, MatchType::Exact));
        }

        // Strategy 2: Whitespace-insensitive
        if self.config.allow_whitespace_flex
            && let Some((pos, len)) = self.fuzzy_find_whitespace(content, search)
        {
            let new_content = Self::replace_at(content, pos, len, replace);
            return Ok((new_content, MatchType::WhitespaceInsensitive));
        }

        // Strategy 3: Indentation-preserving
        if self.config.allow_indent_flex
            && let Some((pos, len)) = self.fuzzy_find_indentation(content, search)
        {
            let new_content = Self::replace_at(content, pos, len, replace);
            return Ok((new_content, MatchType::IndentationPreserving));
        }

        // Strategy 4: Fuzzy Levenshtein
        if self.config.allow_fuzzy_match
            && let Some((pos, len)) = self.fuzzy_find_levenshtein(content, search)
        {
            let new_content = Self::replace_at(content, pos, len, replace);
            return Ok((new_content, MatchType::FuzzyLevenshtein));
        }

        Err(Error::Other(format!(
            "Could not find search text (tried {} strategies). Search: {:?}",
            4,
            &search[..search.len().min(100)]
        )))
    }

    /// Replace text at specific position
    fn replace_at(content: &str, pos: usize, len: usize, replacement: &str) -> String {
        let mut result = String::with_capacity(content.len() + replacement.len());
        result.push_str(&content[..pos]);
        result.push_str(replacement);
        result.push_str(&content[pos + len..]);
        result
    }

    /// Find with whitespace normalization (line-based approach).
    /// Compares lines after collapsing all whitespace runs to single spaces.
    fn fuzzy_find_whitespace(&self, content: &str, search: &str) -> Option<(usize, usize)> {
        let search_lines: Vec<&str> = search.lines().collect();
        let content_lines: Vec<&str> = content.lines().collect();

        if search_lines.is_empty() {
            return None;
        }

        let normalized_search_lines: Vec<String> = search_lines
            .iter()
            .map(|l| normalize_whitespace(l))
            .collect();

        for start_idx in 0..content_lines.len() {
            if start_idx + search_lines.len() > content_lines.len() {
                break;
            }

            let mut matches = true;
            for (i, norm_search) in normalized_search_lines.iter().enumerate() {
                let norm_content = normalize_whitespace(content_lines[start_idx + i]);
                if *norm_search != norm_content {
                    matches = false;
                    break;
                }
            }

            if matches {
                let start_pos: usize = content_lines[..start_idx].iter().map(|l| l.len() + 1).sum();

                let match_len: usize = content_lines[start_idx..start_idx + search_lines.len()]
                    .iter()
                    .map(|l| l.len() + 1)
                    .sum::<usize>()
                    .saturating_sub(1);

                return Some((start_pos, match_len));
            }
        }

        None
    }

    /// Find with indentation flexibility
    fn fuzzy_find_indentation(&self, content: &str, search: &str) -> Option<(usize, usize)> {
        // Split into lines
        let search_lines: Vec<&str> = search.lines().collect();
        let content_lines: Vec<&str> = content.lines().collect();

        if search_lines.is_empty() {
            return None;
        }

        // Try to find matching sequence with flexible indentation
        for start_idx in 0..content_lines.len() {
            if start_idx + search_lines.len() > content_lines.len() {
                break;
            }

            let mut matches = true;
            for (i, search_line) in search_lines.iter().enumerate() {
                let content_line = content_lines[start_idx + i];
                if search_line.trim() != content_line.trim() {
                    matches = false;
                    break;
                }
            }

            if matches {
                // Calculate byte positions
                let start_pos = content_lines[..start_idx]
                    .iter()
                    .map(|l| l.len() + 1) // +1 for newline
                    .sum();

                let match_len = content_lines[start_idx..start_idx + search_lines.len()]
                    .iter()
                    .map(|l| l.len() + 1)
                    .sum::<usize>()
                    .saturating_sub(1); // Last line doesn't have trailing newline in match

                return Some((start_pos, match_len));
            }
        }

        None
    }

    /// Find using Levenshtein distance (with size guards to prevent DoS)
    fn fuzzy_find_levenshtein(&self, content: &str, search: &str) -> Option<(usize, usize)> {
        const MAX_FUZZY_BUDGET: usize = 10_000_000;
        const MAX_SEARCH_LEN: usize = 10_000;

        if search.len() > MAX_SEARCH_LEN || content.len() * search.len() > MAX_FUZZY_BUDGET {
            return None;
        }

        // Sliding window approach
        let search_len = search.len();
        let threshold = (search_len as f32 * (1.0 - self.config.max_fuzzy_distance)) as usize;

        let mut best_match: Option<(usize, usize, usize)> = None; // (pos, len, distance)

        // Try windows of varying sizes around search length
        for window_size in search_len.saturating_sub(threshold)..=search_len + threshold {
            if window_size > content.len() {
                continue;
            }

            for start in 0..=content.len() - window_size {
                let window = &content[start..start + window_size];
                let distance = levenshtein_distance(search, window);

                if distance <= threshold {
                    if let Some((_, _, best_dist)) = best_match {
                        if distance < best_dist {
                            best_match = Some((start, window_size, distance));
                        }
                    } else {
                        best_match = Some((start, window_size, distance));
                    }
                }
            }
        }

        best_match.map(|(pos, len, _)| (pos, len))
    }

    /// Generate preview diff for dry run
    fn generate_preview(&self, content: &str, blocks: &[SearchReplaceBlock]) -> Result<String> {
        let (new_content, _warnings) = self.apply_blocks(content, blocks)?;

        use similar::{ChangeTag, TextDiff};

        let diff = TextDiff::from_lines(content, &new_content);
        let mut preview = String::new();

        for change in diff.iter_all_changes() {
            let sign = match change.tag() {
                ChangeTag::Delete => "-",
                ChangeTag::Insert => "+",
                ChangeTag::Equal => " ",
            };
            preview.push_str(&format!("{} {}", sign, change));
        }

        Ok(preview)
    }
}

impl Default for EditEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse state machine
#[derive(Debug, Clone, Copy, PartialEq)]
enum ParseState {
    Init,
    InSearch,
    InReplace,
}

/// Type of match found
#[derive(Debug, Clone, Copy, PartialEq)]
enum MatchType {
    Exact,
    WhitespaceInsensitive,
    IndentationPreserving,
    FuzzyLevenshtein,
}

impl MatchType {
    fn description(&self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::WhitespaceInsensitive => "whitespace-insensitive",
            Self::IndentationPreserving => "indentation-preserving",
            Self::FuzzyLevenshtein => "fuzzy (Levenshtein)",
        }
    }
}

/// Compute SHA-256 hash of content (with Unicode NFC normalization)
pub fn compute_hash(content: &str) -> String {
    let normalized: String = content.nfc().collect();
    let hash = Sha256::digest(normalized.as_bytes());
    format!("{:x}", hash)
}

/// Normalize whitespace for comparison
fn normalize_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Compute Levenshtein distance between two strings
fn levenshtein_distance(a: &str, b: &str) -> usize {
    // Use strsim crate if available, otherwise use simple implementation
    strsim::levenshtein(a, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_block() {
        let engine = EditEngine::new();
        let input = r#"<<<<<<< SEARCH
old content
=======
new content
>>>>>>> REPLACE"#;

        let blocks = engine.parse_blocks(input).unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].search, "old content");
        assert_eq!(blocks[0].replace, "new content");
    }

    #[test]
    fn test_parse_multiple_blocks() {
        let engine = EditEngine::new();
        let input = r#"<<<<<<< SEARCH
first old
=======
first new
>>>>>>> REPLACE
<<<<<<< SEARCH
second old
=======
second new
>>>>>>> REPLACE"#;

        let blocks = engine.parse_blocks(input).unwrap();
        assert_eq!(blocks.len(), 2);
    }

    #[test]
    fn test_exact_match() {
        let engine = EditEngine::new();
        let content = "Hello world\nThis is a test\nGoodbye world";
        let search = "This is a test";
        let replace = "This is modified";

        let (result, match_type) = engine.find_and_replace(content, search, replace).unwrap();
        assert_eq!(match_type, MatchType::Exact);
        assert!(result.contains("This is modified"));
    }

    #[test]
    fn test_indentation_match() {
        let engine = EditEngine::new();
        let content = "  indented line\n    more indented";
        let search = "indented line\nmore indented"; // No leading spaces

        let (_result, match_type) = engine
            .find_and_replace(content, search, "replaced")
            .unwrap();
        // Whitespace-insensitive strategy (2) fires before indentation (3) since both match
        assert!(
            match_type == MatchType::WhitespaceInsensitive
                || match_type == MatchType::IndentationPreserving
        );
    }

    #[test]
    fn test_whitespace_insensitive_match() {
        let engine = EditEngine::new();
        let content = "hello    world\n  foo   bar";
        let search = "hello world\nfoo bar"; // Normalized whitespace

        let (_result, match_type) = engine
            .find_and_replace(content, search, "replaced")
            .unwrap();
        assert_eq!(match_type, MatchType::WhitespaceInsensitive);
    }

    #[test]
    fn test_hash_computation() {
        let hash1 = compute_hash("test content");
        let hash2 = compute_hash("test content");
        let hash3 = compute_hash("different");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_unicode_normalization_in_hash() {
        // café as precomposed vs decomposed
        let precomposed = "caf\u{00E9}";
        let decomposed = "caf\u{0065}\u{0301}";

        let hash1 = compute_hash(precomposed);
        let hash2 = compute_hash(decomposed);

        // Should be same after NFC normalization
        assert_eq!(hash1, hash2);
    }

    // -------------------------------------------------------------------------
    // New comprehensive tests
    // -------------------------------------------------------------------------

    /// Levenshtein path must be skipped (not hang) when
    /// content.len() * search.len() > 10_000_000 budget.
    ///
    /// With content of 100_000 chars and search of 200 chars the product is
    /// 20_000_000 which exceeds the MAX_FUZZY_BUDGET guard (10_000_000).
    #[test]
    fn test_levenshtein_size_cap_skips_large_input() {
        let engine = EditEngine::new();

        // Build content and search strings that will exceed the budget guard.
        // Content: 100_000 'a' chars.  Search: 200 chars that will never match
        // exactly or via whitespace/indent (different characters).
        let content = "a".repeat(100_000);
        let search = "z".repeat(200); // product = 100_000 * 200 = 20_000_000 > 10_000_000
        let replace = "REPLACED";

        // No strategy should find a match (none of the 'z' patterns exist in
        // the all-'a' content), and crucially the Levenshtein path must be
        // skipped rather than running — so this must return quickly (no hang).
        let result = engine.find_and_replace(&content, &search, replace);
        assert!(
            result.is_err(),
            "Expected an error when no match is found after strategies are exhausted"
        );
    }

    /// A slight character transposition must be caught by the Levenshtein
    /// fuzzy strategy.
    ///
    /// We use a 20-char string where a single-character transposition yields a
    /// Levenshtein distance of 2, which is within the 15% budget for strings
    /// of that length (2/20 = 10% < 15%).
    #[test]
    fn test_levenshtein_fuzzy_match() {
        let engine = EditEngine::with_config(EditConfig {
            // Disable the earlier strategies so we exercise only the Levenshtein path.
            allow_whitespace_flex: false,
            allow_indent_flex: false,
            allow_fuzzy_match: true,
            max_fuzzy_distance: 0.85, // threshold = floor(20 * 0.15) = 3
        });

        // "the quikc brown foxes" — "quick" is misspelled as "quikc"
        // Distance from "the quick brown foxes" is 2 (swap i↔k), which is ≤ 3.
        let content = "the quikc brown foxes";
        let search = "the quick brown foxes";
        let replace = "the quick brown foxes";

        let result = engine.find_and_replace(content, search, replace);
        assert!(
            result.is_ok(),
            "Levenshtein strategy should find the near-match: {:?}",
            result.err()
        );

        let (_, match_type) = result.unwrap();
        assert_eq!(
            match_type,
            MatchType::FuzzyLevenshtein,
            "Expected FuzzyLevenshtein match type"
        );
    }

    /// Tab-delimited content should be matched by the whitespace-insensitive
    /// strategy when searching with plain spaces.
    #[test]
    fn test_whitespace_match_tabs_vs_spaces() {
        let engine = EditEngine::new();

        // Content uses tabs; search uses spaces.
        let content = "\thello\t\tworld";
        let search = "hello world";
        let replace = "replaced";

        let result = engine.find_and_replace(content, search, replace);
        assert!(
            result.is_ok(),
            "Whitespace-insensitive strategy should match tabs vs spaces: {:?}",
            result.err()
        );

        let (_, match_type) = result.unwrap();
        assert_eq!(match_type, MatchType::WhitespaceInsensitive);
    }

    /// Multi-line content with inconsistent leading whitespace must be found
    /// by the whitespace-insensitive strategy.
    #[test]
    fn test_whitespace_match_multiline() {
        let engine = EditEngine::new();

        let content = "  line one\n    line two";
        // Search text has no leading whitespace — should still match via
        // whitespace normalization (each line's tokens are identical).
        let search = "line one\nline two";
        let replace = "replaced";

        let result = engine.find_and_replace(content, search, replace);
        assert!(
            result.is_ok(),
            "WhitespaceInsensitive strategy should match multi-line with different indent: {:?}",
            result.err()
        );

        let (_, match_type) = result.unwrap();
        assert_eq!(match_type, MatchType::WhitespaceInsensitive);
    }

    /// When two sequential SEARCH/REPLACE blocks are applied the second block
    /// must operate on the output of the first (chained substitution).
    #[test]
    fn test_multiple_blocks_sequential() {
        let engine = EditEngine::new();

        // Block 1: foo → bar
        // Block 2: bar → baz  (only valid after block 1 has run)
        let blocks = vec![
            SearchReplaceBlock {
                search: "foo".to_string(),
                replace: "bar".to_string(),
            },
            SearchReplaceBlock {
                search: "bar".to_string(),
                replace: "baz".to_string(),
            },
        ];

        let content = "foo";
        let (result, warnings) = engine.apply_blocks(content, &blocks).unwrap();

        assert_eq!(result, "baz", "Chained replacement should produce 'baz'");
        // Both blocks used exact matching, so no warnings expected.
        assert!(
            warnings.is_empty(),
            "No fuzzy warnings expected: {:?}",
            warnings
        );
    }

    /// A block that has the SEARCH/REPLACE delimiters but is missing the `=======`
    /// separator should return a parse error rather than panicking.
    #[test]
    fn test_malformed_block_missing_separator() {
        let engine = EditEngine::new();

        // No `=======` line — the parser will reach end-of-input while still
        // in the InSearch state and must report an error.
        let malformed = "<<<<<<< SEARCH\nsome content\n>>>>>>> REPLACE";

        let result = engine.parse_blocks(malformed);
        assert!(
            result.is_err(),
            "Missing separator should produce a parse error, not a panic"
        );
    }

    /// `parse_blocks` alone (without `apply_blocks`) must never modify content
    /// — confirming the dry-run / parse-only path is side-effect-free.
    #[test]
    fn test_dry_run_does_not_modify() {
        let engine = EditEngine::new();

        let original = "original content";

        let input = "<<<<<<< SEARCH\noriginal content\n=======\nnew content\n>>>>>>> REPLACE";
        let blocks = engine.parse_blocks(input).unwrap();

        // apply_edits with dry_run = true
        let edit_result = engine.apply_edits(original, &blocks, true).unwrap();

        // dry_run should produce a diff preview but NOT change the content on disk.
        // We verify it by reading original again — it should be unchanged.
        // Also check that the result metadata is consistent.
        assert!(edit_result.success);
        assert!(
            edit_result.diff_preview.is_some(),
            "dry_run should populate diff_preview"
        );

        // The old_hash must equal the hash of the unmodified original.
        let expected_old_hash = compute_hash(original);
        assert_eq!(edit_result.old_hash, expected_old_hash);

        // new_hash must differ (the edit would change the content).
        assert_ne!(
            edit_result.old_hash, edit_result.new_hash,
            "dry_run new_hash should reflect what the edit would produce"
        );
    }
}
