use std::path::PathBuf;
use std::sync::Arc;
use turbovault_core::prelude::*;
use turbovault_tools::RelationshipTools;
use turbovault_vault::VaultManager;

async fn run_on_actual_vault() {
    let config = ServerConfig {
        vaults: vec![VaultConfig::builder("management", "/home/eljese/management").build().unwrap()],
        ..Default::default()
    };

    let manager = Arc::new(VaultManager::new(config).unwrap());
    println!("--- INITIALIZING VAULT (515+ files) ---");
    manager.initialize().await.unwrap();

    let tools = RelationshipTools::new(manager);
    
    println!("--- RUNNING TOPOLOGICAL ANALYSIS ---");
    let report = tools.find_vault_god_nodes().await.expect("Failed to generate report");
    
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}

#[tokio::main]
async fn main() {
    run_on_actual_vault().await;
}
