<div align="center">
  <img src="ronin-banner.jpg" alt="Verantyx CLI AI Agent OS" width="100%" />

  <h3>The Masterless Autonomous Hacker Agent</h3>

  <p>
    Verantyx CLI is a drastically minimal, terminal-native AI assistant and <strong>Local Knowledge Distillation Foundry</strong> designed exclusively for unified memory systems (macOS). It fuses the lightning-fast privacy of local SLMs with deep spatial reasoning to build the next generation of autonomous ecosystems—without relying on heavy, centralized cloud compute. 
  </p>

  <blockquote>
    ⚠️ <strong>Disclaimer</strong>: <i>Verantyx CLI is forged by a single developer in their spare time. It is currently a raw, untested blade. The repository serves partly as a live backup of an evolving thought process. It might slice through your tasks, or it might crash your terminal. If you are brave enough to compile it, expect sharp edges. Contributions and issue reports are highly welcomed as I tame this beast.</i>
  </blockquote>

  <p>
    <a href="https://github.com/verantyx/verantyx-cli/blob/main/LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License: MIT"></a>
    <a href="https://rustup.rs/"><img src="https://img.shields.io/badge/Rust-1.75+-orange.svg" alt="Rust: 1.75+"></a>
    <img src="https://img.shields.io/badge/Cost-$0_API_Fees-success.svg" alt="Cost: $0 API Fees">
    <img src="https://img.shields.io/badge/Platform-macOS_Native-lightgrey.svg" alt="macOS Native">
  </p>
  
  [Website](#) · [Docs](#) · [DeepWiki](#) · [Getting Started](#) · [Discord](#)
</div>

---

## 🎯 Project Purpose

The true value of Verantyx lies in establishing a **Next-Generation Autonomous Agent and Data Generation Ecosystem**.
Rather than just serving as a tool to automate tasks or wrap external APIs, Verantyx aims to run continuous, deeply-reasoned exploratory workflows autonomously. As it operates, it generates high-quality intermediate representation (IR) "thought formulas" and structurally distills knowledge locally. This ultimately bridges the gap between raw hardware capabilities and localized, highly capable intelligent systems operating natively without reliance on heavy GPU matrices.

## 💻 System & Model Requirements

Because Verantyx drives a native ecosystem tied to Apple platforms and relies on deep unified memory for local inference, it has strict hardware prerequisites:

* **OS**: macOS 13 (Ventura) or later
* **Hardware**: Apple Silicon (M1/M2/M3/M4) inherently recommended for `safetensors` / `gguf` inference speed.
* **Memory**: 16GB+ Unified Memory (32GB+ strongly recommended for complex JCross memory operations).

---

## ⚡ Quick Start & Available Modes

No massive Docker dependencies. No external API keys to configure. Just pure, compiled Rust hitting your Mac's unified memory.

> **💡 Note on LLM Weights**: Verantyx automatically pipes instructions to Local LLMs. Ensure you have your desired Small Language Model (SLM) running (e.g., `ollama run qwen2.5:1.5b`) before waking up Verantyx.

### Method 1: Build & Launch Bridge Mode (Current Implementation)

To fire up the cutting-edge Stealth Web Orchestrator and JCross Engine, compile directly from Cargo:

```bash
git clone https://github.com/verantyx/verantyx-cli.git
cd verantyx-cli/verantyx-browser

# Boot to Bridge Mode (Active JCross UI / Native Orchestrator)
cargo run -p vx-browser -- --bridge
```

### Method 2: JCross Interactive Mode (Alpha)

To directly interact with the Neuro-Symbolic Agent and observe its `Reflex` engine routing logic manually:

```bash
cd verantyx-cli/verantyx-browser
cargo run --example interactive_chat
```

---

## 🔪 Core Value & Unprecedented Architecture

Verantyx implements a cutting-edge cognitive framework to escape the limitations of naive vector DB chunking and cloud-dependent execution:

* **Neuro-Symbolic IR Inference (JCross V4 Spatial Semantic Engine):** Gone are the days of naive VectorDB chunks and massive GPU matrix multiplications. Verantyx converts raw data into Semantic Intermediate Representation (IR), using Japanese Kanji as continuous operators (e.g., `[密:0.8]` for density, `[破:1.0]` for purging) to warp and route a physical, gravitational memory space. It runs high-level reasoning, hard-gate pruning, and `Reflex` (muscle memory bindings) natively on the CPU.
* **Knowledge Distillation Loop (Self-Evolving Memory):** By simply running Verantyx, the system autonomously distills high-quality reasoning pathways ("thought formulas") and persists them via Typed Relations (`node_X:因果:0.9`). The local agent gathers robust datasets through real-time execution, acting as an evergreen knowledge foundry.
* **Native macOS Browser Orchestration:** Verantyx seamlessly bridges the gap between local SLMs and web-based resources. By utilizing native OS APIs (`osascript`, `CGEvent`) rather than standard scraping libraries, the local agent can autonomously orchestrate Safari tabs as its own visual interface, enabling complex web interactions and Computer Use tasks directly on your local machine.
* **Triple-Helix Sliding Window Consensus Swarm:** Two LLM instances (Senior and Junior) run continuously in parallel. While the Senior issues system commands, the Junior critiques and audits its actions. Once the Senior reaches its turn limit, it dissolves to prevent context poisoning, the Junior promotes to Senior, and a new Junior boots up.
* **Local Consensus Integration (`tokio::join!`):** Concurrent observations from both parallel LLMs are ingested by a Local LLM Arbiter via `ConsensusActor`. The system objectively summarizes their states into a singular, factual chronological reality (`timeline.md`), ensuring the next Swarm iteration operates entirely flawlessly on clean, hallucination-free context.

---

## 🛡️ Security Defaults (Untrusted Inputs & Data)

In the era of agentic software, **security is non-negotiable.**

* **No API Keys, No Data Exfiltration:** Your codebase, secret `.env` variables, and SSH keys are never bundled and blindly sent to an offshore API endpoint. Your primary reasoning engine runs on local hardware. 
* **Human-in-the-Loop Safe:** Autonomous edits to your physical file system utilize diff-ux mechanisms. Destructive changes are intelligently staged locally.
* **Strict Sandbox Escaping:** Advanced sandbox policies govern OS access to prevent catastrophic loops. 

---

## ⚙️ Build from Source

For those who want to compile the core capabilities themselves:

```bash
git clone https://github.com/verantyx/verantyx-cli.git
cd verantyx-cli/verantyx-browser

# Build inside the Rust crate
cargo build --release

./target/release/verantyx start
```
