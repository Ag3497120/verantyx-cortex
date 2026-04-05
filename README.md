<div align="center">
  <img src="ronin-banner.png" alt="Ronin AI Agent OS" width="100%" />

  <h3>The Masterless Autonomous Hacker Agent</h3>

  <p>
    Ronin is a drastically minimal, terminal-native AI assistant designed exclusively for unified memory systems (macOS). It fuses the lightning-fast privacy of local LLMs with the deep reasoning of stealth-evasion cloud models—without ever needing an API key.
  </p>

  <blockquote>
    ⚠️ <strong>Disclaimer</strong>: <i>Ronin is forged by a single developer in their spare time. It is currently a raw, untested blade. The repository serves partly as a live backup of an evolving thought process. It might slice through your tasks, or it might crash your terminal. If you are brave enough to compile it, expect sharp edges. Contributions and issue reports are highly welcomed as I tame this beast.</i>
  </blockquote>

  <p>
    <a href="https://github.com/verantyx/ronin-cli/blob/main/LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License: MIT"></a>
    <a href="https://rustup.rs/"><img src="https://img.shields.io/badge/Rust-1.75+-orange.svg" alt="Rust: 1.75+"></a>
    <img src="https://img.shields.io/badge/Cost-$0_API_Fees-success.svg" alt="Cost: $0 API Fees">
    <img src="https://img.shields.io/badge/Platform-macOS_Native-lightgrey.svg" alt="macOS Native">
  </p>
  
  [Website](#) · [Docs](#) · [DeepWiki](#) · [Getting Started](#) · [Discord](#)
</div>

---

## ⚡ Quick Start (TL;DR)

No massive Docker dependencies. No external API keys to configure. Just pure, compiled Rust hitting your Mac's unified memory.

```bash
# Install via Cargo
cargo install ronin-cli

# Or download the pre-compiled Mac binary
curl -sSL https://ronin.sh/install | bash

# Wake up the Ronin
ronin start --stealth
```

---

## 🔪 Highlights

* **Masterless Architecture:** You own the infrastructure. Zero dependency on expensive OpenAI or Anthropic API providers. The core reasoning loops run on your local hardware via Gemma 4.
* **Stealth Web Driver:** When deep reasoning is required, Ronin dynamically spawns an invisible WKWebView window to visually scrape and interact with Gemini Ultra's free web tier. Complete evasion of Google's bot detection.
* **JCross Spatial Memory:** Say goodbye to dumb VectorDB chunking. Ronin organizes contextual shards geographically (Front/Mid/Deep memory zones), allowing it to remember 70,000+ lines of code without blowing up the context window.
* **Unix Pipeline Philosophy:** Built for the terminal. Ronin inhales standard input beautifully. Connect it to your existing CLI workflows:
  ```bash
  cat core_dump.log | ronin exec "Find the memory leak and patch the C++ source."
  ```

---

## 🧠 Architecture (How it Works)

Ronin routes intents dynamically based on Tier Profiling. Simple tasks stay local. Complex tasks utilize the Stealth Web Protocol.

```text
┌───────────────┐     Local      ┌────────────────────┐
│  Terminal UX  │ ─────────────▶ │ Gemma 4 (8b-it)    │ (Fast/Free)
│ (Fabric/Unix) │                └────────────────────┘
└───────┬───────┘
        │ Stealth Protocol (Mode 2)
        ▼
┌───────────────┐     DOM        ┌────────────────────┐
│  WKWebView    │ ◀────────────▶ │ Gemini Ultra (Web) │ (Deep Reasoner)
│ (Transparent) │   Scraping     └────────────────────┘
```

---

## 🛡️ Security Defaults (Untrusted Inputs & Data)

In the era of agentic software, **security is non-negotiable.**

* **No API Keys, No Data Exfiltration:** Your codebase, secret `.env` variables, and SSH keys are never bundled and blindly sent to an offshore API endpoint. The reasoning engine lives on your Mac. 
* **Human-in-the-Loop Safe:** Autonomous edits to your physical file system utilize `ronin-diff-ux`. Destructive changes are staged locally with colorized Git diffs. You hold the final `Y/N` permission key.
* **Blind VFS Gatekeeper:** Ronin is strictly jailed to the target workspace directory. Path-traversing outside the authorized perimeter is physically impossible at the VFS execution level.

---

## ⚙️ Build from Source

For those who want to compile the 70k+ lines of autonomous capability themselves:

```bash
git clone https://github.com/verantyx/ronin-cli.git
cd ronin-cli

# Recommended: build in release mode for max inference speed
cargo build --release

./target/release/ronin start
```

### Community
See `CONTRIBUTING.md` for guidelines, maintainers, and how to submit PRs. Hackers welcome. 🤖

*Stay dangerous. Let Ronin handle the code.*
