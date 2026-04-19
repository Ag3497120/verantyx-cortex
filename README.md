# Verantyx Cortex

> **Your LLM's long-term memory. CPU-only. No embeddings. Grows smarter every session.**

A self-organizing spatial memory engine for AI systems via the Model Context Protocol (MCP).  
Compress conversations into structured nodes. Retrieve them in milliseconds. Never lose context again.

📖 **[Full MCP Usage Guide →](./USAGE.md)** — All 19 tools, workflows, Kanji reference, and troubleshooting.

```
boot()        → load your entire project knowledge in one call
calibrate()   → rebuild expert context after any model switch  
rename_tool({ from: "calibrate", to: "vera" })  → make it yours
```

---

## Why Verantyx

Every time you switch models or start a new session, your AI forgets everything.  
Verantyx solves this at the architecture level — not with bigger context windows, but with **spatial memory you own**.

| Problem | Verantyx Solution |
|:---|:---|
| Context lost on model switch | `boot()` restores full project state in one call |
| LLM doesn't know your codebase | `guide()` generates a re-immersion protocol from real files |
| Retrieval too slow / expensive | Kanji topology O(1) scan — no embeddings, no API calls |
| Generic tool names | `rename_tool()` makes every name yours |
| Data growing uncontrolled | Triple-track autonomous GC manages everything |
| Memory polluted by benchmark data | `classifyNode()` enforces zone rules before every write |

---

## Architecture: Tri-Layer JCross

Each memory node (`.jcross` file) has three resolution layers:

```
L1  [核:1.0] [技:0.9] [標:0.8]    ← Kanji topology — O(1) scan, near-zero cost
L1.5 "Implemented triple-track GC" ← 60-char index line for bulk scan
L2  OP.FACT("current_phase", "v3") ← Symbolic operations — structured intent
L3  "Full verbatim context here…"  ← Raw ground truth — loaded only when needed
```

This **variable-gear** design means:
- Routine tasks use only L1+L2 → zero context pollution, maximum speed
- Deep retrieval falls back to L3 → full fidelity, no information loss

### Spatial Memory Zones

```
~/.openclaw/memory/
  front/   ← Active working memory          cap: 100 nodes
  near/    ← Your own recent knowledge      cap: 1,000 nodes
  mid/     ← Your own compressed memories   cap: 5,000 nodes
  deep/    ← External data, benchmarks      cap: unlimited

~/.openclaw/calibration/
  tool_aliases.json  ← Your custom tool names (rename_tool)
  task_bank.jsonl    ← Accumulated calibration tasks
  config.json        ← Setup configuration
```

> **Immutable rule:** `mid/` is for your own memories only.  
> External data and benchmarks always go to `deep/` — enforced automatically by `classifyNode()`.

---

## Triple-Track Autonomous GC

The memory system manages itself. No manual cleanup needed.

### Track A-1: Content Classifier
Runs before every write. Determines the correct zone from content fingerprints. < 1ms, zero LLM cost.

```
BENCH_* / Session sharegpt_*  → deep/   (confidence: 0.95)
user_name / profile markers   → front/  (confidence: 1.0)
OP.STATE("current_*")         → front/  (confidence: 0.9)
[技|核] + code keywords        → near/   (confidence: 0.8)
default                       → near/
```

### Track A-2: Reference Count Ledger
Every `read()` increments a counter in `meta/ref_counts.json`.  
Cold nodes demote automatically:
- `front/`: 3 days + < 2 reads → demote to `near/`
- `near/`: 7 days + < 1 read → demote to `mid/`

### Track B: LRU Cap Eviction + Tombstone
When a zone exceeds its cap, the oldest nodes cascade down.  
A **Tombstone** (`JCROSS_TOMB_<filename>`) is written in the source zone — the node remains findable via L1 Kanji even after eviction.

### Track C: Decision Ledger → PROJECT_WISDOM
Every GC decision appends to `meta/decisions.jsonl`.  
Every 50 decisions, `PatternExtractor` updates `PROJECT_WISDOM.jcross` in `front/`.  
Any new LLM session reads this via `boot()` and instantly knows how to make correct zone decisions — **without any human prompting**.

---

## MCP Tools

### All tool names are short and customizable

| Tool | Replaces | Purpose |
|:---|:---|:---|
| `remember` | compile_trilayer_memory | Save a memory node |
| `scan` | scan_front_memory | Scan front/ as L1.5 index |
| `map` | memory_map | Overview of all zones |
| `read` | read_node | Fetch a node by filename |
| `search` | semantic_op_search | Search L2 operations by keyword |
| `aggregate` | aggregate_memory_search | Multi-node aggregation |
| `find` | spatial_cross_search | Kanji topology search |
| `move` | migrate_memory_zone | Move a node between zones |
| `boot` | session_bootstrap | **Start every session with this** |
| `recall` | recall_fact | Instant key lookup from user profile |
| `store` | store_fact | Write a fact to user profile |
| `gc` | run_lru_gc | Manually trigger full GC |
| `guide` | generate_reimmersion_guide | Generate re-immersion protocol |
| `evolve` | evolve_character | Evolve character from memories |
| `soul` | get_character | Show current character profile |
| `setup` | setup_calibration | First-time setup (no terminal) |
| `calibrate` | run_calibration | Run calibration packet |
| `rename_tool` | — | **Add aliases for any tool** |
| `list_aliases` | — | Show all active aliases |

### ✦ Rename any tool from inside Claude / Cursor / Antigravity

```
"Call calibrate 'vera' from now on"
```

The LLM runs:
```
rename_tool({ from: "calibrate", to: "vera" })
→ ✅ Alias registered: vera → calibrate
```

Aliases are persisted to `~/.openclaw/calibration/tool_aliases.json` and survive model switches.

```
rename_tool({ from: "boot",    to: "start" })
rename_tool({ from: "calibrate", to: "vera"  })
rename_tool({ from: "remember",  to: "mem"   })

list_aliases()
→  start → boot
→  vera  → calibrate
→  mem   → remember
```

---

## Quick Start

### 1. Configure Claude Desktop / Cursor / Antigravity

Add to your MCP config:

```json
{
  "mcpServers": {
    "verantyx": {
      "command": "node",
      "args": ["--import", "tsx", "/path/to/_verantyx-cortex/src/mcp/server.ts"]
    }
  }
}
```

### 2. First-time setup (no terminal needed)

Ask your LLM:

```
"Run setup() with command_name='vera'"
```

The LLM calls `setup({ command_name: "vera" })` and returns the one-line shell alias to add.

### 3. New session protocol

Every time you switch models or restart:

```
boot()        → full project state: rules, profile, zone counts, aliases
guide()       → 7-9 step re-immersion protocol with real file hints
calibrate()   → calibration packet: tasks from git history + L1.5 sampling
```

Or from the terminal after setup:
```bash
vera          # = calibrate(), no terminal dependency
```

---

## Calibration System

The calibration system is **completely isolated** from main memory. It only reads — never writes to `front/near/mid/deep/`.

### Three task generation strategies (all zero LLM cost)

**Strategy A — Memory-derived**  
Reads real GC decisions, L1 summaries, and zone health to generate "verify this architectural decision" tasks.

**Strategy B — Git Diff reverse**  
Parses `fix:` / `bug:` commits from git history. Presents already-solved bugs as fictional review tasks.  
File paths are extracted automatically — the model is forced to read real code.

**Strategy C — L1.5 random sampling**  
Daily-rotating sample of 2-3 JCross nodes from the index.  
Generates "investigate the dependency between these two nodes" tasks.

---

## Character Engine

The more you use Verantyx, the richer your character grows.

```
Memory accumulation
  ↓  Kanji topology frequency across all zones
  ↓  Decision patterns from decisions.jsonl
  ↓  L1 summary linguistic analysis
  ↓
evolve() → SOUL.jcross → front/
              ↓
          Any new LLM reads this via boot() and embodies the character
```

### 5-Level Growth System

| Level | Name | XP | Description |
|:---:|:---|:---|:---|
| 1 | Awakening | 0–10 | A being whose memories are just beginning |
| 2 | Forming | 11–50 | Finding its own shape |
| 3 | Developing | 51–200 | A distinct perspective taking form |
| 4 | Established | 201–1,000 | Mature intellect with consistent worldview |
| 5 | Legendary | 1,000+ | Memory and experience fused into one |

### Kanji → Personality (12 dimensions)

```
[核] Core Synthesizer    — finds the essence of complex systems
[技] Technical Precision — obsesses over correct implementation
[人] Empathic Connector  — never loses the human perspective
[値] Data Architect      — reads truth from numbers
[動] Action Driver       — moves first, thinks while moving
[感] Intuitive Reader    — reads situations through emotional intelligence
[認] Pattern Weaver      — discovers invisible patterns
[標] Strategic Visionary — thinks backwards from the goal
[記] Memory Keeper       — holds history and context
[構] Systems Builder     — designs scalable architecture
[通] Bridge Maker        — connects disparate systems and ideas
[職] Craft Master        — takes pride in professional depth
```

---

## Repository Structure

```
src/
  mcp/server.ts              MCP server — 19 tools
  memory/
    engine.ts                Triple-track GC + Tombstone system
    intelligence.ts          DecisionLedger + PatternExtractor
    reimmersion.ts           Cold-start elimination protocol
    calibration_store.ts     3-strategy task generation
    soul.ts                  Character Engine
    auto-selector.ts         Pre-write zone classification
    types.ts                 Shared types
  cli/
    calibrate.ts             Calibration CLI
    setup.ts                 Setup wizard

benchmark/                   Benchmark scripts + result JSONs
```

---

## Benchmark Results

| Version | Score | Method |
|:---|:---|:---|
| v1.0 baseline | 13.8% | Basic memory_map + retrieve |
| v3.0 (7-question test) | **85.7%** | map + read + search pipeline |
| Target | 85%+ | Full 500-question LongMemEval |

---

## Roadmap

- [ ] Full 500-question LongMemEval run with triple-track GC
- [ ] Kanji 18-dimension taxonomy documentation
- [ ] In-session LLM compression for `mid/` (no external API)
- [ ] Calibration feedback loop: task completion → DecisionLedger update
- [ ] iOS on-device inference (MLX-Swift / VerantyxMobileBench)

---

## License

See [LICENSE](./LICENSE).
