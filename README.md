# TurboVault

## 🍴 Custom Fork Details

This repository is a managed fork of the official [Epistates/turbovault](https://github.com/epistates/turbovault).

- **Upstream Sync:** Merged with official releases to maintain feature parity.
- **Enhanced Tools:** Includes custom agentic tools for Director and Locking workflows.
- **Optimized for LLMs:** Fine-tuned for autonomous agent interactions with Obsidian vaults.

TurboVault is a dual-purpose toolkit designed for both developers and users. It provides a robust, modular **Rust SDK** for building applications that consume markdown directories, and a **full-featured MCP server** that works out of the box with AI agents.

---

## Architecture

Built as a modular Rust workspace:

- `turbovault-core`: Core models and types
- `turbovault-parser`: Obsidian Flavored Markdown parsing
- `turbovault-graph`: Link graph analysis
- `turbovault-tools`: MCP tool implementations
- `turbovault-audit`: Transaction logging and snapshots
- `turbovault`: Main MCP server entry point

## License

MIT License
