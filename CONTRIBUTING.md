# Contributing to Verantyx-CLI

Welcome to the Verantyx Neural Lab! 🧠✨

Verantyx began as a fork of OpenClaw infrastructure but has evolved into a fundamentally distinct cognitive architecture focusing on **Spatial Reasoning (JCross)**, the **Free Energy Principle (Tension/Entropy)**, and the **Autonomous Epistemic Drive**. 

We heavily rely on OpenClaw's robust lower-level Gateway, CLI framework, and Sandbox integrations, but all contributions to Verantyx should be viewed through the lens of self-evolving, autonomous AI ecosystems.

## Quick Links

- **GitHub:** https://github.com/verantyx/verantyx-cli
- **Core Vision:** Autonomous Agents driven by mathematical Free Energy Principles, Self-Healing, and Knowledge Thirst.

## The Paradigm Shift

If you are contributing, keep these core principles in mind:

1. **Autonomous Knowledge Acquisition (Epistemic Drive):**
   Verantyx is not just waiting for string commands. It computes the "Structural Entropy" (`tension_score`) of its active memory graph (`JCross`). All new features should respect this autonomous nature: if the agent lacks data in a component you build, it should dynamically seek it rather than crash or passively wait for hardcoded answers.
2. **Carbon Paper UI (Anti-BotGuard Evasion):**
   We do not use headless Chrome automation tools (which are instantly categorized as bots). We interact via native OS workflows (`osascript`, native APIs, OS Clipboard ingestion) to ensure a zero-detection footprint.
3. **The Cold Judge Evaluation:**
   Before executing massive generative loops, Verantyx passes synthesized architecture proposals through a "Cold Judge" module to ruthlessly evaluate *what is physically missing*.
4. **Local-First (SLM routing):**
   The primary router and planner must always be small enough to run on local silicon without prematurely exposing full codebase contexts to cloud API endpoints.

## How to Contribute

1. **Bugs & small fixes** → Open a PR directly to `main`!
2. **Architectural Improvements** → If you are adjusting the Mathematical algorithms for `calculate_structural_tension` or adding new `.jcross` spatial topologies, please open an Issue first detailing your approach.
3. **Rust Crate Structure (`verantyx-browser`)** → 
   Most active AI behavioral logic lives in:
   - `crates/ronin-hive/examples/interactive_chat.rs` (The main Epistemic Autonomous Loop)
   - `crates/ronin-core/src/memory_bridge/spatial_index.rs` (JCross Tension & Semantic Graph Engine)
   - `crates/ronin-hive/src/roles/` (Gemini Supervisor / Apprentice / Architect Roles)

## Before You PR

- Ensure your changes perfectly compile: `cargo check --workspace`
- Test the interactive chat loop: `cd verantyx-browser && cargo run --example interactive_chat -p ronin-hive`
- **Provide screenshots** of the terminal output if your changes alter the visual formatting, System Nervous Alerts, or Autonomous Bypass logs.
- Remember to respect Safari caching behavior (we utilize physical 5-turn browser tab flushes to mitigate Gemini context window bloat and evasion tracking). Ensure your additions do not break these AppleScript UI evasion mechanisms.
- Keep PRs focused. Do not mix advanced spatial engine logic with unrelated low-level OpenClaw gateway patches.

## AI/Vibe-Coded PRs Welcome! 🤖

As a Neural Architecture project, we highly encourage writing code alongside your favorite LLMs.

Please specify in your PR:
- [ ] What LLM assisted you (e.g., Gemini 2.5 Pro, Claude-3.5)
- [ ] If the code modifies the Epistemic Drive, have you tested it against infinite looping? (e.g. implementing `last_auto_thirst_id` bypass protections)
- [ ] Are you introducing any new `JCross` Tags? (Please define the Kanji semantics in `kanji_ontology.rs` first)

## Report a Vulnerability

Because Verantyx executes massive autonomous code generation loops on the user's local machine, preventing catastrophic local file destruction is our top security priority. If you discover a vulnerability in the Agent's file-write gating or Sandbox constraints, please responsibly report it via an issue before the Epistemic Drive unintentionally reorganizes a user's entire desktop.

---

*“To build an AI that codes is easy. To build an AI that feels pain when it doesn't understand the code—and desperately explores the world to heal that pain—is Verantyx.”*
