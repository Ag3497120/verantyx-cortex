<div align="center">
  <h1>Verantyx-Cortex: 4-AI Swarm Hierarchy & Spatial Memory</h1>
  <p><b>A self-evolving Neuro-Symbolic Swarm Agent for macOS. Verantyx orchestrates a 4-AI architecture (Local SLM + Web Gemini) driven by a 25+ Million line `.jcross` Spatial Memory engine and Model Context Protocol (MCP) integrations.</b></p>
</div>

## 🌐 Overview

**Verantyx-Cortex** (and its CLI integration) introduces a revolutionary 4-node agentic architecture designed to autonomously execute tasks, analyze massive codebases, and maintain self-reflexive spatial memory.

### 🏗️ The 4-AI Hierarchical Swarm

Our system is structured into four distinct cognitive roles:
1. **The Brain / Orchestrator (Web Gemini Pro)**: Acts as the overarching reasoning engine for complex spatial architecture and high-level decision-making.
2. **The Local Worker (qwen2.5:1.5b / Local SLM)**: Operates locally to handle file system edits, manage memory payloads, and sync the Gemini update cycle to prevent context dilution.
3. **The Master Observer (Web Gemini)**: Monitors the local AI and injects anti-patterns and logic constraints. Points out critical flaws in memory synthesis.
4. **The Apprentice Observer (Web Gemini)**: Shadows the Master. Receives memory from the local AI per conversational turn and proposes secondary memory consolidations.

**The 5-Turn Sync Cycle**: Every 5 turns, the Local Worker gathers time-series memories from both the Master and Apprentice. It integrates them (heavily prioritizing the Master's insights) and overwrites the agent's active spatial memory, ensuring hallucination-free long-term continuity.

---

## 🧠 JCross Spatial Memory & Nightwatch

Verantyx does not rely on typical RAG. It utilizes **JCross (.jcross)**—a highly condensed, semantic "Spatial Tensor" memory format. 
* The system actively holds **over 25 million lines** of `.jcross` data.
* This includes massive ontology dictionaries (WordNet), hundreds of automated reasoning benchmarks, and continuous anti-pattern learning.
* **Nightwatch Protocol**: A background daemon that awakens to observe file modifications and losslessly compresses them into `.jcross` spatial nodes using local SLMs (e.g., `gemma4:26b`).

---

## 🚀 Setup Instructions

Say goodbye to chaotic dependency hell. Verantyx unifies everything under a **Single Rust Entrypoint** using `cargo`.

### 1. Prerequisites
- **macOS** (Apple Silicon recommended for local SLM execution / Safari Automation)
- **Rust Toolchain** (`cargo`)
- **Node.js & pnpm** (For building the MCP layers)
- **Ollama** installed and running on localhost.
  * *Required Models:* `qwen2.5:1.5b` (Worker), `gemma4:26b` (Nightwatch observer suite)

### 2. Bootstrapping the Engine
```bash
# 1. Clone the repository
git clone https://github.com/Ag3497120/verantyx-cli.git
cd verantyx-cli/verantyx-browser

# 2. Start the primary Verantyx interactive Swarm
cargo run -p ronin-hive --example interactive_chat
```

---

## 🔌 MCP (Model Context Protocol) Setup

Verantyx leverages MCP to securely bridge the Swarm AI with local file systems and deep memory compilation.

1. **Install MCP dependencies**:
   Run the build script in the root directory or specifically for memory:
   ```bash
   pnpm install
   pnpm build
   ```
2. **Register MCP Servers**:
   The system utilizes custom MCP servers like `verantyx-compiler` (for Tri-Layer Memory) and `jcross-memory` (for direct node reads). Ensure your native AI Gateway/Client points to the built MCP entry points.
3. In runtime, the `StealthGemini` worker will autonomously interface with MCP to execute robust File I/O operations and geometric extraction.

---

## 🔮 The `vera` Memory Trigger

Verantyx features a manual, human-in-the-loop memory compilation override:

When you include the keyword **`vera`** or **`verantyx`** anywhere in your prompt, the AI will intercept the conversation and silently trigger the **`compile_trilayer_memory`** MCP tool.

* **What it does**: It forces the Swarm to stop, extract the critical context, structural decisions, and active codebase states from the current turn, and permanently compile them into the JCross Spatial Drive (Front Memory).
* **Usage Example**: Simply type your request like *"Update the README with our setup configurations. vera"* during a chat session to guarantee the action is permanently indexed.

---

## 📝 Project Duality & Design Philosophy

> *I am a student. This codebase is my Lab—a raw, messy accumulation of every experiment I have poured my soul into to challenge ARC-AGI and overcome API limits. It fuses local LLMs with Browser automation as an experimental mockup to bypass API fees. It's not perfectly clean, but embedded within it is a relentless drive to break through the limits of AI Structural Reasoning.*

**License**: Proprietary. Belongs to the Verantyx spatial intelligence framework.
