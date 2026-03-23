//! Link graph implementation for Obsidian vaults.
//!
//! Provides a high-performance directed graph of files and their connections,
//! supporting case-insensitive resolution, aliases, and fast lookups.

use crate::error::{Error, Result};
use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use turbovault_core::models::{Link, VaultFile};

/// A directed graph representing the connections between files in a vault.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkGraph {
    /// The underlying graph structure
    graph: DiGraph<PathBuf, ()>,

    /// Index from file stem (lowercase) to node indices.
    /// Supports case-insensitive matching for [[Note]] style links.
    file_index: HashMap<String, Vec<NodeIndex>>,

    /// Index from alias (lowercase) to node indices.
    /// Supports [[Alias]] style links.
    alias_index: HashMap<String, Vec<NodeIndex>>,

    /// Index from absolute path to node index.
    path_index: HashMap<PathBuf, NodeIndex>,

    /// Links that could not be resolved to a target file, grouped by source path.
    /// Used by HealthAnalyzer for broken link detection.
    unresolved_links: HashMap<PathBuf, Vec<Link>>,
}

impl LinkGraph {
    /// Create a new link graph
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            file_index: HashMap::new(),
            alias_index: HashMap::new(),
            path_index: HashMap::new(),
            unresolved_links: HashMap::new(),
        }
    }

    /// Total number of unresolved links across all source files.
    pub fn unresolved_link_count(&self) -> usize {
        self.unresolved_links.values().map(|v| v.len()).sum()
    }

    /// Add a file to the graph
    pub fn add_file(&mut self, file: &VaultFile) -> Result<()> {
        let path = file.path.clone();

        // Create node if not exists
        let node_idx = if let Some(&idx) = self.path_index.get(&path) {
            idx
        } else {
            let idx = self.graph.add_node(path.clone());
            self.path_index.insert(path.clone(), idx);

            // Add to file_index by stem (lowercased for case-insensitive resolution)
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                self.file_index
                    .entry(stem.to_lowercase())
                    .or_default()
                    .push(idx);
            }

            idx
        };

        // Register aliases from frontmatter (lowercased for case-insensitive resolution).
        // Guard against duplicates: add_file may be called multiple times for the
        // same path (e.g. on every write_file), so only push if not already present.
        if let Some(fm) = &file.frontmatter {
            for alias in fm.aliases() {
                let entries = self.alias_index.entry(alias.to_lowercase()).or_default();
                if !entries.contains(&node_idx) {
                    entries.push(node_idx);
                }
            }
        }

        Ok(())
    }

    /// Remove a file from the graph.
    ///
    /// **Important**: petgraph's `remove_node` uses swap-remove — the last node
    /// in the graph is moved into the removed node's slot. We must update all
    /// external index maps (`path_index`, `file_index`, `alias_index`) to reflect
    /// the swapped node's new `NodeIndex`.
    pub fn remove_file(&mut self, path: &PathBuf) -> Result<()> {
        if let Some(&idx) = self.path_index.get(path) {
            // Remove the target node from all indices
            self.path_index.remove(path);
            self.unresolved_links.remove(path);

            // Filter out this node from stem index
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                if let Some(indices) = self.file_index.get_mut(&stem.to_lowercase()) {
                    indices.retain(|&i| i != idx);
                }
            }

            // Filter out this node from alias index
            // Note: In a real implementation, we might want to track which aliases
            // belong to which file to avoid a full scan, but here we just clean up.
            for indices in self.alias_index.values_mut() {
                indices.retain(|&i| i != idx);
            }

            // Perform the swap-remove
            if let Some(swapped_path) = self.graph.remove_node(idx) {
                // remove_node returns the weight of the removed node (path).
                // If the removed node was NOT the last node, another node was moved into its slot.
                // The moved node's index was `last_index`, and is now `idx`.
                let last_index = NodeIndex::new(self.graph.node_count());

                if idx != last_index {
                    // Update indices for the swapped node
                    self.update_node_index(last_index, idx, &swapped_path);
                }
            }
        }
        Ok(())
    }

    /// Update external indices when a NodeIndex changes due to swap-remove.
    fn update_node_index(&mut self, old_idx: NodeIndex, new_idx: NodeIndex, path: &PathBuf) {
        // Update path index
        if let Some(val) = self.path_index.get_mut(path) {
            *val = new_idx;
        }

        // Update stem index
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            if let Some(indices) = self.file_index.get_mut(&stem.to_lowercase()) {
                for i in indices.iter_mut() {
                    if *i == old_idx {
                        *i = new_idx;
                    }
                }
            }
        }

        // Update alias index
        for indices in self.alias_index.values_mut() {
            for i in indices.iter_mut() {
                if *i == old_idx {
                    *i = new_idx;
                }
            }
        }
    }

    /// Rebuild all edges based on current file contents and resolved links.
    pub fn rebuild_edges(&mut self, files: &HashMap<PathBuf, VaultFile>) -> Result<()> {
        self.graph.clear_edges();
        self.unresolved_links.clear();

        for (source_path, file) in files {
            let source_idx = *self
                .path_index
                .get(source_path)
                .ok_or_else(|| Error::graph_error(format!("Source node missing: {:?}", source_path)))?;

            for link in &file.links {
                if let Some(target_idx) = self.resolve_link(&link.target) {
                    self.graph.add_edge(source_idx, target_idx, ());
                } else {
                    // Track unresolved links for broken link detection
                    let mut broken = link.clone();
                    broken.is_valid = false;
                    self.unresolved_links
                        .entry(source_path.clone())
                        .or_default()
                        .push(broken);
                }
            }
        }

        Ok(())
    }

    /// Resolve a wikilink target to a file path and node index.
    /// Resolution is case-insensitive to match Obsidian's behaviour.
    fn resolve_link(&self, target: &str) -> Option<NodeIndex> {
        // Remove block/heading references
        let clean_target = target.split('#').next()?.trim();
        let clean_lower = clean_target.replace('\\', "/").to_lowercase();

        // Try direct stem match (case-insensitive, first-found wins)
        if let Some(indices) = self.file_index.get(&clean_lower) {
            if let Some(&idx) = indices.first() {
                return Some(idx);
            }
        }

        // Try alias match (case-insensitive, first-found wins)
        if let Some(indices) = self.alias_index.get(&clean_lower) {
            if let Some(&idx) = indices.first() {
                return Some(idx);
            }
        }

        // Try path-like match (folder/Note) with case-insensitive comparison.
        // Obsidian wikilinks omit the .md extension, so we strip it from path
        // components before comparing.
        let target_parts: Vec<String> = clean_lower
            .split('/')
            .filter(|p| !p.is_empty())
            .collect();

        if target_parts.is_empty() {
            return None;
        }

        // Search through all paths in the index
        for (path, &idx) in self.path_index.iter() {
            let mut path_parts: Vec<String> = path
                .iter()
                .filter_map(|p| p.to_str())
                .map(|p| p.to_lowercase())
                .collect();

            // Strip .md extension from the last component to match Obsidian's
            // extension-free wikilink convention (e.g. [[folder/Note]] resolves
            // to folder/Note.md)
            if let Some(last) = path_parts.last_mut() {
                if let Some(stripped) = last.strip_suffix(".md") {
                    *last = stripped.to_string();
                }
            }

            if path_parts.len() >= target_parts.len() {
                // Compare from the end of the path
                let start = path_parts.len() - target_parts.len();
                let actual_tail = &path_parts[start..];

                let mut matches = true;
                for i in 0..target_parts.len() {
                    let t_part = &target_parts[i];
                    let a_part = &actual_tail[i];

                    if i == target_parts.len() - 1 {
                        // Last part: compare stem (ignore .md or other extensions)
                        let a_stem = Path::new(a_part)
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or(a_part);
                        
                        // Match if stem matches or full name matches (in case target had extension)
                        if a_stem != t_part && a_part != t_part {
                            matches = false;
                            break;
                        }
                    } else if a_part != t_part {
                        matches = false;
                        break;
                    }
                }

                if matches {
                    return Some(idx);
                }
            }
        }

        None
    }

    /// Find all nodes that link TO this file.
    pub fn get_backlinks(&self, path: &PathBuf) -> Vec<PathBuf> {
        if let Some(&idx) = self.path_index.get(path) {
            self.graph
                .neighbors_directed(idx, petgraph::Incoming)
                .map(|neighbor| self.graph[neighbor].clone())
                .collect()
        } else {
            vec![]
        }
    }

    /// Find all nodes that this file links TO.
    pub fn get_forward_links(&self, path: &PathBuf) -> Vec<PathBuf> {
        if let Some(&idx) = self.path_index.get(path) {
            self.graph
                .neighbors_directed(idx, petgraph::Outgoing)
                .map(|neighbor| self.graph[neighbor].clone())
                .collect()
        } else {
            vec![]
        }
    }

    /// Find all notes connected within N hops in the link graph.
    pub fn get_related_notes(&self, path: &PathBuf, max_hops: usize) -> Vec<PathBuf> {
        if let Some(&start_idx) = self.path_index.get(path) {
            let mut visited = HashMap::new();
            let mut queue = std::collections::VecDeque::new();

            visited.insert(start_idx, 0);
            queue.push_back((start_idx, 0));

            while let Some((curr_idx, dist)) = queue.pop_front() {
                if dist >= max_hops {
                    continue;
                }

                // Check both incoming and outgoing edges for "relatedness"
                for neighbor in self.graph.neighbors_undirected(curr_idx) {
                    if !visited.contains_key(&neighbor) {
                        visited.insert(neighbor, dist + 1);
                        queue.push_back((neighbor, dist + 1));
                    }
                }
            }

            visited
                .into_iter()
                .filter(|&(idx, _)| idx != start_idx)
                .map(|(idx, _)| self.graph[idx].clone())
                .collect()
        } else {
            vec![]
        }
    }

    /// Find nodes with incoming links but no outgoing links.
    pub fn get_dead_end_notes(&self) -> Vec<(PathBuf, usize)> {
        self.graph
            .node_indices()
            .filter(|&idx| {
                self.graph.neighbors_directed(idx, petgraph::Outgoing).count() == 0
                    && self.graph.neighbors_directed(idx, petgraph::Incoming).count() > 0
            })
            .map(|idx| {
                let backlink_count = self.graph.neighbors_directed(idx, petgraph::Incoming).count();
                (self.graph[idx].clone(), backlink_count)
            })
            .collect()
    }

    /// Find nodes with the most total connections (hub notes).
    pub fn get_hub_notes(&self, limit: usize) -> Vec<(PathBuf, usize)> {
        let mut hubs: Vec<(PathBuf, usize)> = self.graph
            .node_indices()
            .map(|idx| {
                let total_links = self.graph.neighbors_undirected(idx).count();
                (self.graph[idx].clone(), total_links)
            })
            .collect();

        hubs.sort_by(|a, b| b.1.cmp(&a.1));
        hubs.truncate(limit);
        hubs
    }

    /// Check if a circular reference chain exists between notes.
    pub fn detect_cycles(&self) -> Vec<Vec<PathBuf>> {
        let mut cycles = Vec::new();
        let scc = petgraph::algo::tarjan_scc(&self.graph);
        
        for component in scc {
            if component.len() > 1 {
                cycles.push(component.into_iter().map(|idx| self.graph[idx].clone()).collect());
            }
        }
        
        cycles
    }

    /// Get all unresolved links.
    pub fn unresolved_links(&self) -> &HashMap<PathBuf, Vec<Link>> {
        &self.unresolved_links
    }
}
