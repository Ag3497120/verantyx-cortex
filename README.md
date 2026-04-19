# Verantyx Cortex — Tri-Layer JCross Memory Engine

> **Pure CPU · LLM-agnostic · MCP-native**  
> 永続的な空間記憶システム。コンテキストウィンドウを信頼するな — MCPを信頼せよ。

---

## 概要

Verantyx Cortex は、LLMのコンテキストウィンドウ制限を根本的に解消します。  
会話・決定・知識を **3層JCross形式** に圧縮してディスクに永続保存し、あらゆるAI（Claude・Gemini・GPT・Cursor）がMCP経由でゼロ設定で利用できます。

**新規LLMセッションが `boot()` → `guide()` の2コールで専門家レベルのコンテキストを再構築できます。**

---

## 🏗️ アーキテクチャ：3層JCross記憶

各メモリノード (`.jcross` ファイル) は3層構造です：

| 層 | 名称 | 内容 | コスト |
|:---|:---|:---|:---|
| **L1** | 漢字トポロジー | `[核:1.0] [技:0.9]` — O(1)スキャン用超圧縮タグ | ほぼゼロ |
| **L1.5** | インデックス行 | 60文字の一行サマリー。bulk scanに使用 | ほぼゼロ |
| **L2** | 操作ロジック | `OP.FACT/STATE/ENTITY` — 意図・決定・事実を構造化 | 低 |
| **L3** | 生テキスト | 元の会話・ドキュメント。高精度タスクのみ使用 | 高（オンデマンド）|

### 空間メモリゾーン

```
~/.openclaw/memory/
  front/   ← アクティブ作業記憶 (上限: 100ノード)
  near/    ← ユーザー自身の最近の記憶 (上限: 1,000)
  mid/     ← ユーザー自身の長期記憶 (上限: 5,000)
  deep/    ← 外部データ・ベンチデータ・コールドストレージ (上限なし)
  meta/
    decisions.jsonl   ← GC決定ログ (Track C)
    ref_counts.json   ← 参照カウンター (Track A-2)

~/.openclaw/calibration/   ← 本体記憶を汚染しない独立ストア
  config.json              ← セットアップ設定
  tool_aliases.json        ← ツールエイリアス (rename_tool で追加)
  snapshot.json            ← 非破壊メモリスナップショット
  task_bank.jsonl          ← 蓄積キャリブレーションタスク
  sessions/                ← セッション履歴
```

> **ゾーン設計原則（不変）**  
> `mid/` = ユーザー自身の記憶 **のみ**。外部データ・ベンチデータは必ず `deep/` へ。  
> `classifyNode()` がすべての書き込み前に自動的にこれを実施します。

---

## ⚙️ Triple-Track 自律GC

記憶システムは3つの独立した仕組みで自律管理されます：

### Track A-1: コンテンツ分類器 (`classifyNode`)
書き込み前に実行。内容フィンガープリントから正しいゾーンを決定。LLMコールなし・< 1ms。

```
BENCH_* / Session sharegpt_*  → deep/   (conf: 0.95)
user_name / profile           → front/  (conf: 1.0)
OP.STATE("current_*")         → front/  (conf: 0.9)
[技|核] + コードキーワード    → near/   (conf: 0.8)
デフォルト                    → near/
```

### Track A-2: 参照カウンターGC
`read()` 呼び出しのたびにカウンターをインクリメント。  
コールドノード（低参照 + 古いmtime）を自動降格：
- `front/`：3日 + 2回未満 → `near/`
- `near/`：7日 + 1回未満 → `mid/`

### Track B: LRU上限エビクション + Tombstone
ゾーンが上限を超えると最古ノードをpush。  
**Tombstone** (`JCROSS_TOMB_<filename>`) を移動元に残し、エビクト後もL1漢字経由で発見可能。

### Track C: 決定台帳 → PROJECT_WISDOM
50決定ごとに `PatternExtractor` が集計し `PROJECT_WISDOM.jcross` を更新。  
新規LLMが `boot()` でこれを読むことで、プロジェクト専門家レベルの判断が可能になります。

---

## 🔧 MCPツール一覧

### ツール名は短くシンプル

| ツール名 | 旧名 | 用途 |
|:---|:---|:---|
| `remember` | compile_trilayer_memory | 記憶を保存 |
| `scan` | scan_front_memory | front/をスキャン |
| `map` | memory_map | 全ゾーン概観 |
| `read` | read_node | ノードを読み込む |
| `search` | semantic_op_search | L2セマンティック検索 |
| `aggregate` | aggregate_memory_search | 複数ノード集計 |
| `find` | spatial_cross_search | 漢字トポロジー検索 |
| `move` | migrate_memory_zone | ゾーン間移動 |
| `boot` | session_bootstrap | **セッション起動（必ず最初に）** |
| `recall` | recall_fact | 事実を即座に照会 |
| `store` | store_fact | 事実を永続保存 |
| `gc` | run_lru_gc | GCを手動実行 |
| `guide` | generate_reimmersion_guide | 再没入プロトコル生成 |
| `evolve` | evolve_character | キャラクター進化 |
| `soul` | get_character | キャラクター表示 |
| `setup` | setup_calibration | 初回設定 |
| `calibrate` | run_calibration | キャリブレーション実行 |
| `rename_tool` | — | **ツール名を変更** ✨新機能 |
| `list_aliases` | — | エイリアス一覧 |

### ✨ ツール名を自由に変更できます

LLMに話しかけるだけで変更できます：

```
「calibrateというツールをveraという名前で呼べるようにして」
```

LLMは以下を実行します：

```
rename_tool({ from: "calibrate", to: "vera" })
```

→ 以後、`vera()` が `calibrate()` と同じ動作をします。  
エイリアスは `~/.openclaw/calibration/tool_aliases.json` に永続保存されます。

```bash
# 現在のエイリアス一覧
list_aliases()

# 例: 複数のエイリアス
rename_tool({ from: "boot",      to: "start" })
rename_tool({ from: "calibrate", to: "vera"  })
rename_tool({ from: "remember",  to: "mem"   })
```

エイリアスをチェーンすることも可能です：`vera → calibrate`、`v → vera → calibrate` のように解決されます。

---

## 🚀 新規セッション プロトコル

モデル切り替え・セッション再開時の標準フロー：

```
1. boot()         ← PROJECT_WISDOM + user_profile + zone counts
2. guide()        ← 再没入プロトコル（7-9ステップ）
3. calibrate()    ← キャリブレーションパケット（タスク生成）
```

または `setup()` でターミナルコマンド（例: `vera`）を登録しておけば：

```bash
vera    # = calibrate() を端末から実行
```

### ターミナル不要の運用

Claude Desktop / Cursor / Antigravity から直接：

| 操作 | MCPツール |
|:---|:---|
| 初回設定 | `setup(command_name="vera")` |
| セッション起動 | `boot()` |
| 再没入 | `guide()` |
| キャリブレーション | `calibrate()` |
| ツール名変更 | `rename_tool(from="calibrate", to="vera")` |
| キャラクター進化 | `evolve()` |

---

## 👤 キャラクターエンジン

MCPを使うほど成長するキャラクターシステム。

```
記憶の蓄積
  ↓ 漢字トポロジー頻度
  ↓ 意思決定パターン
  ↓ L1サマリーの語感
  ↓
evolve()  → SOUL.jcross → front/
               ↓
          boot() で次のLLMが継承
```

**5段階レベル：**

| Level | 名称 | XP | 説明 |
|:---:|:---|:---|:---|
| 1 | Awakening | 0-10 | 記憶が芽吹き始めた存在 |
| 2 | Forming | 11-50 | 自分の形を見つけようとしている |
| 3 | Developing | 51-200 | 独自の視点と価値観が固まりつつある |
| 4 | Established | 201-1000 | 成熟した知性と一貫した世界観を持つ |
| 5 | Legendary | 1000+ | 記憶と経験が深く融合した伝説的存在 |

**漢字 → 性格マッピング（15次元）：**

```
[核] Core Synthesizer    — 本質を直感的に掴む
[技] Technical Precision — 技術的精度へのこだわり
[人] Empathic Connector  — 他者との関係性を大切に
[値] Data Architect      — データから真実を読む
[動] Action Driver       — 考えるより動く
[感] Intuitive Reader    — 直感と感情的知性
[認] Pattern Weaver      — 見えないパターンを発見
[標] Strategic Visionary — ゴールから逆算する
[記] Memory Keeper       — 歴史と文脈を保持する
[構] Systems Builder     — スケーラブルな構造を設計
[通] Bridge Maker        — 異なる概念を繋ぐ
[職] Craft Master        — 仕事への誇りと専門性
```

---

## 📁 ファイル構成

```
src/
  mcp/server.ts              MCPサーバー (19ツール)
  memory/
    engine.ts                Triple-track GC + Tombstone
    intelligence.ts          DecisionLedger + PatternExtractor
    reimmersion.ts           コールドスタート解消
    calibration_store.ts     3戦略タスク生成
    soul.ts                  キャラクターエンジン
    auto-selector.ts         pre-write zone分類
    types.ts                 共有型定義
  cli/
    calibrate.ts             キャリブレーションCLI
    setup.ts                 セットアップウィザード

benchmark/                   ベンチマークスクリプト + 結果
```

---

## 📊 ベンチマーク実績

| バージョン | スコア | 手法 |
|:---|:---|:---|
| v1.0 (flash_agent) | 13.8% | 基本的な memory_map + retrieve |
| v3.0 (7問テスト) | 85.7% | map + read + search の組み合わせ |
| 目標 | 85%+ | 500問 LongMemEval フル実行 |

---

## 🗺️ ロードマップ

- [ ] 500問 LongMemEval フル再実行（Triple-track GC込み）
- [ ] 漢字18次元タクソノミー文書化
- [ ] `mid/` 昇華：セッション内LLM圧縮（外部APIなし）
- [ ] キャリブレーションフィードバックループ（タスク完了 → DecisionLedger更新）
- [ ] iOS on-device推論統合 (MLX-Swift / VerantyxMobileBench)
