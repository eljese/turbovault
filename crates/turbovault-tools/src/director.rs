//! Director Agent tools for swarm orchestration and Dynamic Swarm Recruitment (DSR)

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;
use turbovault_core::prelude::*;
use turbovault_vault::VaultManager;

/// Represents a coalition of agents recruited for a specific mission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmCoalition {
    /// The agent leading the swarm (typically Sentinel Orchestrator)
    pub lead_agent: String,
    /// Specialized sub-agents recruited for the task
    pub sub_agents: Vec<String>,
    /// Rationale for the recruitment decision
    pub reasoning: String,
    /// Parameters derived from the initial intent
    pub mission_parameters: Value,
}

/// Director Agent tools for managing agent swarms
pub struct DirectorTools {
    pub manager: Arc<VaultManager>,
}

impl DirectorTools {
    /// Create new Director tools
    pub fn new(manager: Arc<VaultManager>) -> Self {
        Self { manager }
    }

    /// Recruit a swarm coalition based on a prompt (DSR logic)
    ///
    /// Analyzes the prompt for expertise requirements and identifies the best sub-agents.
    pub async fn recruit_swarm(&self, prompt: &str) -> Result<Value> {
        let mut sub_agents = Vec::new();
        let lead_agent = "Sentinel Orchestrator".to_string();
        let mut reasoning = String::new();

        // Phase 1 DSR: Pattern-based expertise matching
        let prompt_lower = prompt.to_lowercase();

        // Chief of Staff: Finance, Tasks, Planning
        if prompt_lower.contains("finance") || 
           prompt_lower.contains("cost") || 
           prompt_lower.contains("budget") ||
           prompt_lower.contains("task") ||
           prompt_lower.contains("todo") {
            sub_agents.push("Chief of Staff".to_string());
            reasoning.push_str("Found planning/financial intent. ");
        }

        // The Global Architect: Infrastructure, Code, System Design
        if prompt_lower.contains("infrastructure") || 
           prompt_lower.contains("docker") || 
           prompt_lower.contains("portainer") || 
           prompt_lower.contains("rust") ||
           prompt_lower.contains("refactor") ||
           prompt_lower.contains("architecture") {
            sub_agents.push("The Global Architect".to_string());
            reasoning.push_str("Found structural/technical intent. ");
        }

        // The Scribe: Documentation, Memory
        if prompt_lower.contains("document") || 
           prompt_lower.contains("log") || 
           prompt_lower.contains("note") ||
           prompt_lower.contains("memory") ||
           prompt_lower.contains("record") {
            sub_agents.push("The Scribe".to_string());
            reasoning.push_str("Found documentation/archival intent. ");
        }

        // Fallback to Generalist
        if sub_agents.is_empty() {
            sub_agents.push("Generalist".to_string());
            reasoning.push_str("Broad intent detected. Recruited Generalist. ");
        }

        let coalition = SwarmCoalition {
            lead_agent: lead_agent.clone(),
            sub_agents: sub_agents.clone(),
            reasoning: reasoning.trim().to_string(),
            mission_parameters: json!({ "original_prompt": prompt }),
        };

        // IAC via Neo4j :MESSAGE nodes (Future: actual implementation)
        // For now, we return the decision which the Orchestrator can then persist
        
        Ok(json!({
            "operation": "DSR_RECRUITMENT",
            "status": "Success",
            "coalition": coalition,
            "next_action": "PERSIST_COALITION_IN_GRAPH",
            "agent_briefs": sub_agents.iter().map(|a| {
                match a.as_str() {
                    "Chief of Staff" => "Manage project resources and task synchronization.",
                    "The Global Architect" => "Execute technical implementation and maintain system integrity.",
                    "The Scribe" => "Document actions and synchronize knowledge graph.",
                    _ => "Execute sub-tasks as directed."
                }
            }).collect::<Vec<_>>()
        }))
    }

    /// Record a message in the Inter-Agent Communication (IAC) channel
    pub async fn post_swarm_message(&self, from: &str, to: &str, content: &str) -> Result<Value> {
        // This tool simulates the creation of a :MESSAGE node in Neo4j
        // Logic: (Agent {name: from})-[:SENT]->(m:Message {content: content})-[:TO]->(Agent {name: to})
        
        Ok(json!({
            "operation": "IAC_MESSAGE",
            "status": "Queued",
            "message": {
                "from": from,
                "to": to,
                "content": content,
                "timestamp": chrono::Utc::now().to_rfc3339()
            },
            "interpretation": format!("{} sent a mission update to {}", from, to)
        }))
    }
}
