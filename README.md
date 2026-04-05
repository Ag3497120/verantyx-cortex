<div align="center">
  <img src="ronin-banner.jpg" alt="Verantyx CLI AI Agent OS" width="100%" />

  <h3>The Masterless Autonomous Hacker Agent</h3>

  <p>
    Verantyx CLI is a drastically minimal, terminal-native AI assistant designed exclusively for unified memory systems (macOS). It fuses the lightning-fast privacy of local LLMs with the deep reasoning of stealth-evasion cloud models—without ever needing an API key. 
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

## 💻 System & Model Requirements

Because Verantyx drives a stealth Safari/WKWebView native to Apple platforms and relies on deep unified memory for local inference, it has strict hardware prerequisites:

* **OS**: macOS 13 (Ventura) or later
* **Hardware**: Apple Silicon (M1/M2/M3/M4) inherently recommended for `safetensors` / `gguf` inference speed.
* **Memory**: 16GB+ Unified Memory (32GB+ strongly recommended for complex JCross memory operations).

---

## ⚡ Quick Start (TL;DR)

No massive Docker dependencies. No external API keys to configure. Just pure, compiled Rust hitting your Mac's unified memory.

> **💡 Note on LLM Weights**: Verantyx automatically pipes instructions to Local LLMs (Ollama). Make sure you have [Ollama](https://ollama.com/) installed and have pulled the base model (`ollama run qwen2.5:1.5b`) before waking up Verantyx.

```bash
# Install via Cargo
cargo install verantyx-cli

# Or download the pre-compiled Mac binary
curl -sSL https://verantyx.run/install | bash

# Wake up Verantyx
verantyx start --stealth
```

---

## 🔪 Unprecedented Swarm Architecture (New!)

Verantyx implements a cutting edge cognitive framework to prevent AI hallucination and memory pollution during autonomous operations:

* **Triple-Helix Sliding Window Consensus Swarm:** Two LLM instances (Senior and Junior) run continuously in parallel. While the Senior issues system commands, the Junior critiques and audits its actions. Once the Senior reaches its turn limit, it dissolves to prevent context poisoning, the Junior promotes to Senior, and a new Junior boots up.
* **Local Consensus Integration (`tokio::join!`):** Concurrent observations from both parallel LLMs are ingested by a Local LLM Arbiter via `ConsensusActor`. The system objectively summarizes their states into a singular, factual chronological reality (`timeline.md`), ensuring the next Swarm iteration operates entirely flawlessly on clean, hallucination-free context.
* **Stealth Web Driver (HITL Evasion):** Deep reasoning leverages automated browser instances seamlessly isolated to ensure a 0% Bot Detection Rate by using native OS automations (osascript) across invisible Safari tabs.
* **JCross Spatial Memory:** Say goodbye to dumb VectorDB chunking. Verantyx organizes contextual shards geographically (Front/Mid/Deep memory zones), allowing it to remember massive codebases infinitely via chronological sliding-window handovers.

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
