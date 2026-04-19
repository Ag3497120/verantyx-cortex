/**
 * Verantyx Cortex — MCP Server v3.1
 *
 * Tool names are intentionally SHORT (e.g. "remember", "search", "boot").
 * Users can create ANY alias for any tool via rename_tool():
 *   rename_tool({ from: "calibrate", to: "vera" })
 *   → from now on, "vera" works exactly like "calibrate"
 *
 * Aliases are persisted to ~/.openclaw/calibration/tool_aliases.json
 * and loaded on every ListTools + CallTool request.
 */

import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { CallToolRequestSchema, ListToolsRequestSchema } from "@modelcontextprotocol/sdk/types.js";
import { compileTriLayerJCross } from "../memory/auto-selector.js";
import path from "path";
import fs from "fs";

const server = new Server(
    { name: "verantyx-trilayer-memory", version: "3.1.0" },
    { capabilities: { tools: {} } }
);

const ENGINE_ROOT = path.resolve(process.env.HOME || "~", ".openclaw/memory");
const ALIAS_FILE  = path.resolve(process.env.HOME || "~", ".openclaw/calibration/tool_aliases.json");

// ─── Alias helpers ────────────────────────────────────────────────────────────

function loadAliases(): Record<string, string> {
    try {
        if (fs.existsSync(ALIAS_FILE)) {
            return JSON.parse(fs.readFileSync(ALIAS_FILE, "utf-8"));
        }
    } catch { /* ignore */ }
    return {};
}

function saveAliases(aliases: Record<string, string>): void {
    const dir = path.dirname(ALIAS_FILE);
    if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });
    fs.writeFileSync(ALIAS_FILE, JSON.stringify(aliases, null, 2) + "\n", "utf-8");
}

/** Resolve a tool name through the alias map (supports chaining). */
function resolveToolName(name: string, aliases: Record<string, string>): string {
    const seen = new Set<string>();
    let current = name;
    while (aliases[current] && !seen.has(current)) {
        seen.add(current);
        current = aliases[current];
    }
    return current;
}

// ─── Core tool definitions ────────────────────────────────────────────────────

const CORE_TOOLS = [
    {
        name: "remember",
        description: [
            "【記憶を保存】会話・決定・文脈を永続的なJCrossメモリノードに圧縮して保存します。",
            "classifyNode()が自動的に正しいゾーンを選択します (front/near/mid/deep)。",
            "旧名: compile_trilayer_memory",
        ].join(" "),
        inputSchema: {
            type: "object",
            properties: {
                kanjiTags:       { type: "string",  description: "L1 Kanji topology tags. Example: '[核:1.0] [技:0.9] [標:0.8]'" },
                l1Summary:       { type: "string",  description: "1-2 sentence L1 summary." },
                midResOperations:{ type: "array", items: { type: "string" }, description: "L2 operation commands. Example: [\"OP.FACT(\\\"key\\\", \\\"value\\\")\"]" },
                rawText:         { type: "string",  description: "L3 verbatim raw text." },
            },
            required: ["kanjiTags", "l1Summary", "midResOperations", "rawText"],
        },
    },
    {
        name: "scan",
        description: "【front/スキャン】front/の全JCrossノードをL1.5インデックス行として返します。旧名: scan_front_memory",
        inputSchema: { type: "object", properties: {} },
    },
    {
        name: "map",
        description: "【メモリマップ】全ゾーンの概観をL1.5インデックスで返します。旧名: memory_map",
        inputSchema: {
            type: "object",
            properties: {
                queryText: { type: "string", description: "If provided, sort nodes by relevance to this query." },
                maxNodes:  { type: "number", description: "Max nodes per zone (default: 100)." },
                zones:     { type: "array", items: { type: "string" }, description: "Zones to scan (default: ['front'])." },
            },
        },
    },
    {
        name: "read",
        description: "【ノード読込】ファイル名でJCrossノードを取得します。L2+L3を返します。旧名: read_node",
        inputSchema: {
            type: "object",
            properties: {
                fileName: { type: "string", description: "Exact filename (e.g. 'TURN_1234.jcross')." },
                layer:    { type: "string", enum: ["l1", "l2", "l3", "l2l3"], description: "Which layers to return (default: l2l3)." },
                zone:     { type: "string", enum: ["front", "near", "mid", "deep"], description: "Zone hint to speed up lookup." },
            },
            required: ["fileName"],
        },
    },
    {
        name: "search",
        description: "【L2セマンティック検索】OP.FACT/ENTITY/STATEをキーワードで検索します。旧名: semantic_op_search",
        inputSchema: {
            type: "object",
            properties: {
                queryText:  { type: "string", description: "Natural language query." },
                topK:       { type: "number", description: "Max results (default: 5)." },
                zonesHint:  { type: "array", items: { type: "string" }, description: "Optional zone filter." },
            },
            required: ["queryText"],
        },
    },
    {
        name: "aggregate",
        description: "【集計検索】複数ノードをまとめて取得・集計します。旧名: aggregate_memory_search",
        inputSchema: {
            type: "object",
            properties: {
                queryText: { type: "string", description: "Aggregation query." },
                topK:      { type: "number", description: "Number of nodes to aggregate (default: 5, max: 10)." },
                zonesHint: { type: "array", items: { type: "string" }, description: "Optional zone filter." },
            },
            required: ["queryText"],
        },
    },
    {
        name: "find",
        description: "【L1漢字検索】ARC-SGI Gravity Z-Depthアルゴリズムで漢字トポロジー検索します。旧名: spatial_cross_search",
        inputSchema: {
            type: "object",
            properties: {
                queryKanji: {
                    type: "object",
                    description: "Kanji search vector. Example: {\"核\": 1.0, \"技\": 0.9}",
                    additionalProperties: { type: "number" },
                },
            },
            required: ["queryKanji"],
        },
    },
    {
        name: "move",
        description: "【ゾーン移動】ノードを別のゾーンへ移動します。移動元にTombstoneを残します。旧名: migrate_memory_zone",
        inputSchema: {
            type: "object",
            properties: {
                fileName:   { type: "string", description: "Filename to migrate." },
                targetZone: { type: "string", enum: ["front", "near", "mid", "deep"], description: "Destination zone." },
            },
            required: ["fileName", "targetZone"],
        },
    },
    {
        name: "boot",
        description: [
            "【セッション起動】セッション開始時に必ず最初に呼び出します。",
            "PROJECT_WISDOM + user_profile + front/ノード + ゾーンカウントを返します。",
            "旧名: session_bootstrap",
        ].join(" "),
        inputSchema: { type: "object", properties: {} },
    },
    {
        name: "recall",
        description: "【事実照会】user_profileから特定キーの値を即座に返します。旧名: recall_fact",
        inputSchema: {
            type: "object",
            properties: {
                key: { type: "string", description: "Fact key (e.g. 'user_name', 'current_focus')." },
            },
            required: ["key"],
        },
    },
    {
        name: "store",
        description: "【事実保存】user_profile.jcrossにkey-value事実を書き込みます。旧名: store_fact",
        inputSchema: {
            type: "object",
            properties: {
                key:   { type: "string", description: "Fact key (snake_case)." },
                value: { type: "string", description: "Fact value." },
            },
            required: ["key", "value"],
        },
    },
    {
        name: "gc",
        description: "【GC実行】Triple-track GC (classifyNode + RefCount + LRU) を手動実行します。旧名: run_lru_gc",
        inputSchema: {
            type: "object",
            properties: {
                front_cap: { type: "number", description: "Override cap for front/ (default: 100)." },
                near_cap:  { type: "number", description: "Override cap for near/ (default: 1000)." },
                mid_cap:   { type: "number", description: "Override cap for mid/ (default: 5000)." },
            },
        },
    },
    {
        name: "guide",
        description: [
            "【再没入ガイド生成】新規セッションでboot()の次に呼び出します。",
            "REIMMERSION_PROTOCOL.jcrossを生成し、5-9ステップの再コンテキスト化プロトコルを返します。",
            "旧名: generate_reimmersion_guide",
        ].join(" "),
        inputSchema: {
            type: "object",
            properties: {
                project_root: { type: "string", description: "Absolute path to project root. Default: auto-detected." },
            },
        },
    },
    {
        name: "evolve",
        description: "【キャラクター進化】蓄積した記憶からキャラクター人格を合成しSOUL.jcrossに保存します。旧名: evolve_character",
        inputSchema: { type: "object", properties: {} },
    },
    {
        name: "soul",
        description: "【キャラクター表示】現在のSOUL.jcrossを読み込んでキャラクタープロファイルを表示します。旧名: get_character",
        inputSchema: { type: "object", properties: {} },
    },
    {
        name: "setup",
        description: [
            "【初回設定】キャリブレーションシステムを設定します。",
            "ターミナル不要 — Claude Desktop / Cursor / Antigravity から直接呼び出せます。",
            "旧名: setup_calibration",
        ].join(" "),
        inputSchema: {
            type: "object",
            properties: {
                command_name: { type: "string", description: "Shell alias name (e.g. 'vera', 'cal', 'moto'). Default: 'cal'" },
                project_root: { type: "string", description: "Absolute path to project root. Default: auto-detected." },
                shell_rc:     { type: "string", description: "Shell RC file (e.g. '~/.zshrc'). Default: auto-detected." },
            },
        },
    },
    {
        name: "calibrate",
        description: [
            "【キャリブレーション実行】モデル切り替え・セッション再開時に使います。",
            "3戦略でタスクを生成 (記憶由来 + Git Diff逆算 + L1.5サンプリング)。",
            "ターミナル不要。旧名: run_calibration",
        ].join(" "),
        inputSchema: {
            type: "object",
            properties: {
                output_format: { type: "string", enum: ["text", "json"], description: "Output format. Default: text" },
                project_root:  { type: "string", description: "Project root. Default: auto-detected." },
            },
        },
    },
    {
        name: "rename_tool",
        description: [
            "【ツール名変更】任意のMCPツールにエイリアス(別名)を設定します。",
            "例: rename_tool({from: 'calibrate', to: 'vera'}) → 以後 'vera' で呼び出せます。",
            "エイリアスは ~/.openclaw/calibration/tool_aliases.json に保存されます。",
            "list_aliases で現在のエイリアス一覧を確認できます。",
        ].join(" "),
        inputSchema: {
            type: "object",
            properties: {
                from: { type: "string", description: "元のツール名 (または既存エイリアス名)。" },
                to:   { type: "string", description: "新しいエイリアス名。英数字とハイフン・アンダースコアのみ。" },
            },
            required: ["from", "to"],
        },
    },
    {
        name: "list_aliases",
        description: "【エイリアス一覧】現在登録されている全ツールエイリアスを表示します。",
        inputSchema: { type: "object", properties: {} },
    },
];

// ─── ListTools: core + aliases ────────────────────────────────────────────────

server.setRequestHandler(ListToolsRequestSchema, async () => {
    const aliases = loadAliases();
    const aliasTools = Object.entries(aliases).map(([alias, original]) => {
        const base = CORE_TOOLS.find(t => t.name === original);
        if (!base) return null;
        return {
            ...base,
            name: alias,
            description: `[エイリアス→${original}] ${base.description}`,
        };
    }).filter(Boolean);

    return { tools: [...CORE_TOOLS, ...aliasTools] };
});

// ─── CallTool ─────────────────────────────────────────────────────────────────

server.setRequestHandler(CallToolRequestSchema, async (request) => {
    const { name: rawName, arguments: args } = request.params;

    // Resolve alias chain
    const aliases = loadAliases();
    const name = resolveToolName(rawName, aliases);

    // ── remember ──────────────────────────────────────────────────────────────
    if (name === "remember") {
        const { kanjiTags, l1Summary, midResOperations, rawText } = args as any;
        try {
            await compileTriLayerJCross({ kanjiTags, l1Summary, midResOperations, rawText }, ENGINE_ROOT);
            return { content: [{ type: "text", text: "✅ Memory compiled and saved to JCross Drive." }] };
        } catch (e: any) {
            return { isError: true, content: [{ type: "text", text: `remember error: ${e.message}` }] };
        }
    }

    // ── scan ──────────────────────────────────────────────────────────────────
    if (name === "scan") {
        try {
            const frontDir = path.join(ENGINE_ROOT, "front");
            if (!fs.existsSync(frontDir)) return { content: [{ type: "text", text: "Front memory empty." }] };
            const files = fs.readdirSync(frontDir).filter(f => f.endsWith(".jcross"));
            const lines = files.map(f => {
                try {
                    const raw    = fs.readFileSync(path.join(frontDir, f), "utf-8");
                    const kanjiM = raw.match(/【空間座相】\s*\n([^\n【]+)/);
                    const l1M    = raw.match(/\[標\] := "([^"]{1,60})"/);
                    return `${f}: ${kanjiM?.[1].trim() ?? ""} | ${l1M?.[1] ?? ""}`;
                } catch { return `${f}: (unreadable)`; }
            });
            return { content: [{ type: "text", text: lines.join("\n") || "No nodes in front/." }] };
        } catch (e: any) {
            return { isError: true, content: [{ type: "text", text: `scan error: ${e.message}` }] };
        }
    }

    // ── map ───────────────────────────────────────────────────────────────────
    if (name === "map") {
        try {
            const { MemoryEngine } = await import("../memory/engine.js");
            const eng = new MemoryEngine(ENGINE_ROOT);
            const zones = (args as any)?.zones ?? ["front"];
            const maxN  = (args as any)?.maxNodes ?? 100;
            const lines: string[] = [];
            for (const zone of zones) {
                const dir = path.join(ENGINE_ROOT, zone);
                if (!fs.existsSync(dir)) continue;
                const files = fs.readdirSync(dir).filter(f => f.endsWith(".jcross")).slice(0, maxN);
                lines.push(`\n=== ${zone}/ (${files.length} nodes) ===`);
                for (const f of files) {
                    try {
                        const raw  = fs.readFileSync(path.join(dir, f), "utf-8");
                        const kM   = raw.match(/【空間座相】\s*\n([^\n【]+)/);
                        const l1M  = raw.match(/\[標\] := "([^"]{1,60})"/);
                        lines.push(`  ${f}: ${kM?.[1]?.trim() ?? ""} | ${l1M?.[1] ?? ""}`);
                    } catch { /* skip */ }
                }
            }
            return { content: [{ type: "text", text: lines.join("\n") || "No nodes found." }] };
        } catch (e: any) {
            return { isError: true, content: [{ type: "text", text: `map error: ${e.message}` }] };
        }
    }

    // ── read ──────────────────────────────────────────────────────────────────
    if (name === "read") {
        const { fileName, layer = "l2l3", zone } = args as any;
        try {
            const searchZones = zone ? [zone] : ["front", "near", "mid", "deep"];
            for (const z of searchZones) {
                const fp = path.join(ENGINE_ROOT, z, fileName);
                if (fs.existsSync(fp)) {
                    const raw = fs.readFileSync(fp, "utf-8");
                    // Increment ref count
                    try {
                        const { MemoryEngine } = await import("../memory/engine.js");
                        new MemoryEngine(ENGINE_ROOT).getLedger().touch(fileName);
                    } catch { /* non-fatal */ }
                    if (layer === "l1") {
                        const m = raw.match(/【空間座相】[\s\S]*?(?=【位相対応表】|【操作対応表】)/);
                        return { content: [{ type: "text", text: m?.[0] ?? raw.slice(0, 200) }] };
                    }
                    if (layer === "l2") {
                        const m = raw.match(/【操作対応表】[\s\S]*?(?=【生テキスト】|【META】|$)/);
                        return { content: [{ type: "text", text: m?.[0] ?? "(no L2 section)" }] };
                    }
                    if (layer === "l3") {
                        const m = raw.match(/【生テキスト】([\s\S]*?)(?=【META】|$)/);
                        return { content: [{ type: "text", text: m?.[1]?.trim() ?? "(no L3 section)" }] };
                    }
                    return { content: [{ type: "text", text: raw }] };
                }
            }
            return { content: [{ type: "text", text: `Node not found: ${fileName}` }] };
        } catch (e: any) {
            return { isError: true, content: [{ type: "text", text: `read error: ${e.message}` }] };
        }
    }

    // ── search ────────────────────────────────────────────────────────────────
    if (name === "search") {
        const { queryText, topK = 5, zonesHint } = args as any;
        try {
            const zones = zonesHint ?? ["front", "near", "mid", "deep"];
            const results: { file: string; zone: string; ops: string[] }[] = [];
            for (const z of zones) {
                const dir = path.join(ENGINE_ROOT, z);
                if (!fs.existsSync(dir)) continue;
                for (const f of fs.readdirSync(dir).filter(f => f.endsWith(".jcross"))) {
                    try {
                        const raw = fs.readFileSync(path.join(dir, f), "utf-8");
                        const ops = (raw.match(/OP\.[A-Z_]+\("[^"]+",\s*"[^"]+"\)/g) ?? [])
                            .filter(op => op.toLowerCase().includes(queryText.toLowerCase()));
                        if (ops.length > 0) results.push({ file: f, zone: z, ops });
                        if (results.length >= topK) break;
                    } catch { /* skip */ }
                }
                if (results.length >= topK) break;
            }
            if (results.length === 0) return { content: [{ type: "text", text: `No results for "${queryText}".` }] };
            const text = results.map(r => `[${r.zone}/${r.file}]\n  ${r.ops.join("\n  ")}`).join("\n\n");
            return { content: [{ type: "text", text: text }] };
        } catch (e: any) {
            return { isError: true, content: [{ type: "text", text: `search error: ${e.message}` }] };
        }
    }

    // ── aggregate ─────────────────────────────────────────────────────────────
    if (name === "aggregate") {
        const { queryText, topK = 5, zonesHint } = args as any;
        try {
            const zones = zonesHint ?? ["front", "near", "mid", "deep"];
            const nodes: string[] = [];
            for (const z of zones) {
                const dir = path.join(ENGINE_ROOT, z);
                if (!fs.existsSync(dir)) continue;
                for (const f of fs.readdirSync(dir).filter(f => f.endsWith(".jcross"))) {
                    try {
                        const raw = fs.readFileSync(path.join(dir, f), "utf-8");
                        if (raw.toLowerCase().includes(queryText.toLowerCase())) {
                            nodes.push(`=== ${z}/${f} ===\n${raw.slice(0, 800)}`);
                        }
                        if (nodes.length >= topK) break;
                    } catch { /* skip */ }
                }
                if (nodes.length >= topK) break;
            }
            return { content: [{ type: "text", text: nodes.join("\n\n") || `No matches for "${queryText}".` }] };
        } catch (e: any) {
            return { isError: true, content: [{ type: "text", text: `aggregate error: ${e.message}` }] };
        }
    }

    // ── find ──────────────────────────────────────────────────────────────────
    if (name === "find") {
        const { queryKanji } = args as any;
        try {
            const results: { file: string; zone: string; score: number; l1: string }[] = [];
            for (const z of ["front", "near", "mid", "deep"]) {
                const dir = path.join(ENGINE_ROOT, z);
                if (!fs.existsSync(dir)) continue;
                for (const f of fs.readdirSync(dir).filter(f => f.endsWith(".jcross"))) {
                    try {
                        const raw    = fs.readFileSync(path.join(dir, f), "utf-8");
                        const kanjiM = raw.match(/【空間座相】\s*\n([^\n【]+)/);
                        if (!kanjiM) continue;
                        let score = 0;
                        for (const [k, w] of Object.entries(queryKanji as Record<string, number>)) {
                            const m = kanjiM[1].match(new RegExp(`\\[${k}:(\\d+\\.?\\d*)\\]`));
                            if (m) score += parseFloat(m[1]) * w;
                        }
                        if (score > 0) {
                            const l1M = raw.match(/\[標\] := "([^"]{1,80})"/);
                            results.push({ file: f, zone: z, score, l1: l1M?.[1] ?? "" });
                        }
                    } catch { /* skip */ }
                }
            }
            results.sort((a, b) => b.score - a.score);
            const text = results.slice(0, 10).map(r =>
                `[${r.zone}/${r.file}] score=${r.score.toFixed(2)}\n  "${r.l1}"`
            ).join("\n\n");
            return { content: [{ type: "text", text: text || "No matching nodes." }] };
        } catch (e: any) {
            return { isError: true, content: [{ type: "text", text: `find error: ${e.message}` }] };
        }
    }

    // ── move ──────────────────────────────────────────────────────────────────
    if (name === "move") {
        const { fileName, targetZone } = args as any;
        try {
            const { MemoryEngine } = await import("../memory/engine.js");
            const eng = new MemoryEngine(ENGINE_ROOT);
            const moved = eng.move(fileName, targetZone);
            if (!moved) return { isError: true, content: [{ type: "text", text: `File not found: ${fileName}` }] };
            return { content: [{ type: "text", text: `✅ Moved ${fileName} → ${targetZone}/` }] };
        } catch (e: any) {
            return { isError: true, content: [{ type: "text", text: `move error: ${e.message}` }] };
        }
    }

    // ── boot ──────────────────────────────────────────────────────────────────
    if (name === "boot") {
        try {
            const summary: Record<string, any> = {
                project_wisdom: {},
                user_profile:   {},
                front_nodes:    [],
                zone_counts:    {},
            };

            for (const zone of ["front", "near", "mid", "deep"]) {
                const dir = path.join(ENGINE_ROOT, zone);
                summary.zone_counts[zone] = fs.existsSync(dir)
                    ? fs.readdirSync(dir).filter(f => f.endsWith(".jcross")).length : 0;
            }

            const frontDir = path.join(ENGINE_ROOT, "front");
            if (fs.existsSync(frontDir)) {
                for (const f of fs.readdirSync(frontDir).filter(f => f.endsWith(".jcross"))) {
                    try {
                        const raw = fs.readFileSync(path.join(frontDir, f), "utf-8");
                        for (const m of raw.matchAll(/OP\.(?:ENTITY|FACT|STATE)\("([^"]+)",\s*"([^"]+)"\)/g)) {
                            summary.user_profile[m[1]] = m[2];
                        }
                        const l1M = raw.match(/\[標\] := "([^"]{1,100})"/);
                        if (l1M) summary.front_nodes.push({ file: f, l1: l1M[1] });

                        if (f.includes("PROJECT_WISDOM")) {
                            const ruleM = raw.match(/【ACCUMULATED_RULES】([\s\S]*?)(?:【|$)/);
                            if (ruleM) summary.project_wisdom.accumulated_rules = ruleM[1].trim();
                        }
                        if (f.includes("SOUL")) {
                            const lvM = raw.match(/OP\.STATE\("character_level",\s*"([^"]+)"\)/);
                            if (lvM) summary.user_profile.character_level = lvM[1];
                        }
                    } catch { /* skip */ }
                }
            }

            const currentAliases = loadAliases();
            const output = [
                "# Verantyx Boot",
                `zone_counts: ${JSON.stringify(summary.zone_counts)}`,
                `character_level: ${summary.user_profile.character_level ?? "(not yet evolved)"}`,
                "",
                "## User Profile",
                Object.entries(summary.user_profile).slice(0, 10).map(([k, v]) => `  ${k}: ${v}`).join("\n"),
                "",
                "## Project Wisdom",
                summary.project_wisdom.accumulated_rules
                    ? summary.project_wisdom.accumulated_rules.slice(0, 500)
                    : "  (no accumulated rules yet — sessions will build this)",
                "",
                "## Active Front Nodes",
                summary.front_nodes.slice(0, 8).map((n: any) => `  ${n.file}: "${n.l1}"`).join("\n"),
                "",
                "## Tool Aliases",
                Object.keys(currentAliases).length > 0
                    ? Object.entries(currentAliases).map(([a, t]) => `  ${a} → ${t}`).join("\n")
                    : "  (none — use rename_tool() to add aliases)",
                "",
                "## Next steps",
                "  1. guide()        — generate re-immersion protocol",
                "  2. calibrate()    — run calibration packet",
                "  3. recall('key')  — look up specific facts",
            ].join("\n");
            return { content: [{ type: "text", text: output }] };
        } catch (e: any) {
            return { isError: true, content: [{ type: "text", text: `boot error: ${e.message}` }] };
        }
    }

    // ── recall ────────────────────────────────────────────────────────────────
    if (name === "recall") {
        const { key } = args as any;
        try {
            const frontDir = path.join(ENGINE_ROOT, "front");
            if (!fs.existsSync(frontDir)) return { content: [{ type: "text", text: `Not found: ${key}` }] };
            for (const f of fs.readdirSync(frontDir).filter(f => f.endsWith(".jcross"))) {
                const raw = fs.readFileSync(path.join(frontDir, f), "utf-8");
                const m   = raw.match(new RegExp(`OP\\.(?:ENTITY|FACT|STATE)\\("${key}",\\s*"([^"]+)"\\)`));
                if (m) return { content: [{ type: "text", text: m[1] }] };
            }
            return { content: [{ type: "text", text: `Not found: ${key}` }] };
        } catch (e: any) {
            return { isError: true, content: [{ type: "text", text: `recall error: ${e.message}` }] };
        }
    }

    // ── store ─────────────────────────────────────────────────────────────────
    if (name === "store") {
        const { key, value } = args as any;
        try {
            const profilePath = path.join(ENGINE_ROOT, "front", "user_profile.jcross");
            const op = `OP.FACT("${key}", "${value}")`;
            if (fs.existsSync(profilePath)) {
                let raw = fs.readFileSync(profilePath, "utf-8");
                const existing = new RegExp(`OP\\.(?:ENTITY|FACT|STATE)\\("${key}",\\s*"[^"]*"\\)`);
                if (existing.test(raw)) {
                    raw = raw.replace(existing, op);
                } else {
                    raw = raw.replace(/【META】/, `${op}\n【META】`);
                }
                fs.writeFileSync(profilePath, raw, "utf-8");
            } else {
                const content = `■ JCROSS_USER_PROFILE\n【空間座相】\n[人:1.0] [記:0.9]\n\n【位相対応表】\n[標] := "User profile"\n\n【操作対応表】\n${op}\n\n【META】\n`;
                fs.writeFileSync(profilePath, content, "utf-8");
            }
            return { content: [{ type: "text", text: `✅ Stored: ${key} = ${value}` }] };
        } catch (e: any) {
            return { isError: true, content: [{ type: "text", text: `store error: ${e.message}` }] };
        }
    }

    // ── gc ────────────────────────────────────────────────────────────────────
    if (name === "gc") {
        try {
            const { MemoryEngine } = await import("../memory/engine.js");
            const caps = {
                front: (args as any)?.front_cap ?? 100,
                near:  (args as any)?.near_cap  ?? 1000,
                mid:   (args as any)?.mid_cap   ?? 5000,
                deep:  Infinity,
            };
            const eng = new MemoryEngine(ENGINE_ROOT);
            const report = eng.runAutonomousGc(caps);
            const total  = report.classifier.length + report.coldEvictions.length +
                report.lruEvictions.reduce((s: number, r: { evicted: string[] }) => s + r.evicted.length, 0);
            return { content: [{ type: "text", text:
                `GC complete. Total moved: ${total}\n` +
                `  Classifier: ${report.classifier.length}\n` +
                `  Cold evictions: ${report.coldEvictions.length}\n` +
                `  LRU evictions: ${report.lruEvictions.map((r: any) => `${r.zone}:${r.evicted.length}`).join(", ") || "none"}`
            }] };
        } catch (e: any) {
            return { isError: true, content: [{ type: "text", text: `gc error: ${e.message}` }] };
        }
    }

    // ── guide ─────────────────────────────────────────────────────────────────
    if (name === "guide") {
        try {
            const os = await import("os");
            const HOME = os.homedir();
            const candidates = [
                (args as any)?.project_root,
                path.join(HOME, "verantyx-cli"),
                process.cwd(),
            ].filter(Boolean);
            const projectRoot = candidates.find(p => fs.existsSync(path.join(p, "_verantyx-cortex"))) ?? candidates[0];
            const { generateReimmersionGuide, writeReimmersionProtocol } = await import("../memory/reimmersion.js");
            writeReimmersionProtocol(projectRoot, ENGINE_ROOT);
            const guide = generateReimmersionGuide(projectRoot, ENGINE_ROOT);
            const output = [
                `# ${guide.title}`,
                `Total steps: ${guide.totalSteps} | Cost: ${guide.estimatedTokenCost}`,
                "",
                "## Steps",
                ...guide.steps.map(s => {
                    const icon = s.costEstimate === "low" ? "🟢" : s.costEstimate === "medium" ? "🟡" : "🔴";
                    return `### Step ${s.step} ${icon} [${s.type}]\n**Action**: ${s.action}\n**Why**: ${s.reason}\n**You gain**: ${s.expectedGain}\n`;
                }),
                "## Kanji Vectors",
                guide.kanjiSearchVectors.map((v, i) => `  vector_${i + 1}: ${JSON.stringify(v)}`).join("\n"),
                "",
                "## Synthesis",
                guide.synthesisPrompt,
                "",
                "✅ REIMMERSION_PROTOCOL.jcross written to front/",
            ].join("\n");
            return { content: [{ type: "text", text: output }] };
        } catch (e: any) {
            return { isError: true, content: [{ type: "text", text: `guide error: ${e.message}` }] };
        }
    }

    // ── evolve ────────────────────────────────────────────────────────────────
    if (name === "evolve") {
        try {
            const { CharacterEngine } = await import("../memory/soul.js");
            const eng  = new CharacterEngine(ENGINE_ROOT);
            const soul = eng.evolve();
            const bar  = "█".repeat(soul.level.level) + "░".repeat(5 - soul.level.level);
            return { content: [{ type: "text", text: [
                `# ⭐ Character Evolved`,
                `Level: Lv.${soul.level.level} ${soul.level.name}  [${bar}]`,
                `XP: ${soul.level.experience} | Wisdom: ${soul.memoryStats.wisdomScore}%`,
                "",
                "## Traits",
                soul.primaryTraits.map(t => `  • ${t.name}: ${t.description}`).join("\n"),
                "",
                "## Signature",
                soul.signaturePhrases.map(p => `  「${p}」`).join("\n"),
                "",
                soul.level.nextLevel ? `Next level in ${soul.level.nextLevel - soul.level.experience} XP` : "🏆 Legendary",
                "✅ SOUL.jcross → front/",
            ].join("\n") }] };
        } catch (e: any) {
            return { isError: true, content: [{ type: "text", text: `evolve error: ${e.message}` }] };
        }
    }

    // ── soul ──────────────────────────────────────────────────────────────────
    if (name === "soul") {
        try {
            const soulPath = path.join(ENGINE_ROOT, "front", "SOUL.jcross");
            if (!fs.existsSync(soulPath))
                return { content: [{ type: "text", text: "SOUL.jcross not found. Call evolve() first." }] };
            const raw  = fs.readFileSync(soulPath, "utf-8");
            const card = raw.match(/【CHARACTER_CARD】([\s\S]*?)(?:【|$)/)?.[1]?.trim() ?? raw;
            return { content: [{ type: "text", text: card }] };
        } catch (e: any) {
            return { isError: true, content: [{ type: "text", text: `soul error: ${e.message}` }] };
        }
    }

    // ── setup ─────────────────────────────────────────────────────────────────
    if (name === "setup") {
        try {
            const os   = await import("os");
            const HOME = os.homedir();
            const calRoot     = path.join(HOME, ".openclaw", "calibration");
            fs.mkdirSync(path.join(calRoot, "sessions"), { recursive: true });
            const commandName = (args as any)?.command_name ?? "cal";
            const projectRoot = (args as any)?.project_root ??
                [path.join(HOME, "verantyx-cli"), process.cwd()].find(p => fs.existsSync(path.join(p, "_verantyx-cortex"))) ??
                path.join(HOME, "verantyx-cli");
            const shellRc = (args as any)?.shell_rc ??
                (process.env.SHELL?.includes("zsh") ? path.join(HOME, ".zshrc") : path.join(HOME, ".bashrc"));
            const cortexDir = path.join(projectRoot, "_verantyx-cortex");
            const nodeBin   = process.execPath;
            const tsxPath   = path.join(cortexDir, "node_modules", "tsx", "dist", "esm", "index.cjs");
            const calibPath = path.join(cortexDir, "src", "cli", "calibrate.ts");
            const config = {
                version: 2, command_name: commandName, project_root: projectRoot,
                mcp_cortex_dir: cortexDir, shell_rc: shellRc,
                created_at: new Date().toISOString(),
            };
            fs.writeFileSync(path.join(calRoot, "config.json"), JSON.stringify(config, null, 2) + "\n", "utf-8");
            const tsxExists = fs.existsSync(tsxPath);
            const aliasLine = tsxExists
                ? `alias ${commandName}='${nodeBin} --import ${tsxPath} ${calibPath}'`
                : `alias ${commandName}='npx --prefix ${cortexDir} tsx ${calibPath}'`;
            try {
                const { writeReimmersionProtocol } = await import("../memory/reimmersion.js");
                writeReimmersionProtocol(projectRoot, ENGINE_ROOT);
            } catch { /* non-fatal */ }
            return { content: [{ type: "text", text: [
                "# ✅ Setup Complete",
                `command_name: ${commandName}`,
                `project_root: ${projectRoot}`,
                `shell_rc:     ${shellRc}`,
                "",
                "## Add to shell RC (one-time)",
                "```bash",
                `# verantyx-calibrate: ${commandName}`,
                aliasLine,
                "```",
                "",
                "## From now on",
                `  Terminal: \`${commandName}\``,
                `  MCP tool: \`calibrate()\`  ← no terminal needed`,
                "",
                "✅ REIMMERSION_PROTOCOL.jcross updated",
            ].join("\n") }] };
        } catch (e: any) {
            return { isError: true, content: [{ type: "text", text: `setup error: ${e.message}` }] };
        }
    }

    // ── calibrate ─────────────────────────────────────────────────────────────
    if (name === "calibrate") {
        try {
            const { CalibrationStore } = await import("../memory/calibration_store.js");
            const os = await import("os");
            const HOME = os.homedir();
            const outputFmt = (args as any)?.output_format ?? "text";
            let projectRoot = (args as any)?.project_root;
            if (!projectRoot) {
                const confPath = path.join(HOME, ".openclaw", "calibration", "config.json");
                try { projectRoot = JSON.parse(fs.readFileSync(confPath, "utf-8")).project_root; } catch { /* ignore */ }
                if (!projectRoot) {
                    projectRoot = [path.join(HOME, "verantyx-cli"), process.cwd()]
                        .find(p => fs.existsSync(path.join(p, "_verantyx-cortex"))) ?? path.join(HOME, "verantyx-cli");
                }
            }
            const store    = new CalibrationStore(ENGINE_ROOT);
            const snapshot = store.captureSnapshot();
            const tasks    = store.generateMemoryDerivedTasks(snapshot, projectRoot);
            const sessionId = new Date().toISOString().replace(/[:.]/g, "-").slice(0, 19);

            if (outputFmt === "json") {
                const json = JSON.stringify({ session_id: sessionId, snapshot, tasks }, null, 2);
                store.saveSession(json, { id: sessionId, created_at: new Date().toISOString(), zone_counts: snapshot.zone_counts, task_count: tasks.length });
                return { content: [{ type: "text", text: json }] };
            }

            const zStr    = Object.entries(snapshot.zone_counts).map(([z, c]) => `${z}/:${c}`).join(" | ");
            const aliases = loadAliases();
            const taskStr = tasks.map((t, i) => {
                const icon = t.priority === "high" ? "🔴" : t.priority === "medium" ? "🟡" : "🟢";
                return [
                    `### Task ${i + 1} ${icon} [${t.source}]`,
                    `**Task**: ${t.task}`,
                    t.file_hint ? `**Read**: \`${t.file_hint}\`` : "",
                    `**Context**: ${t.context}`,
                ].filter(Boolean).join("\n");
            }).join("\n\n");

            const text = [
                `# ⚡ CALIBRATION: ${sessionId}`,
                `Memory: ${zStr}`,
                "",
                "## 1. First call",
                "```",
                "boot()   ← always first",
                "```",
                "",
                "## 2. User Profile",
                Object.entries(snapshot.profile).slice(0, 6).map(([k, v]) => `  ${k}: ${v}`).join("\n") || "  (empty)",
                "",
                "## 3. Tool Aliases",
                Object.keys(aliases).length > 0
                    ? Object.entries(aliases).map(([a, t]) => `  ${a} → ${t}`).join("\n")
                    : "  (none — use rename_tool() to customize)",
                "",
                `## 4. Review Tasks (${tasks.length})`,
                taskStr || "  (sessions will generate meaningful tasks)",
                "",
                "## 5. Search Vectors",
                "```json",
                `{"核": 1.0, "技": 0.9, "標": 0.8}   // Architecture`,
                `{"人": 1.0, "職": 0.9, "標": 0.7}   // Profile`,
                "```",
                `Call: \`find({"核": 1.0, "技": 0.9})\``,
                "",
                "## 6. Synthesis",
                "  MCP > context window. Zone: front→near→mid→deep.",
                "  Bench data → deep/ always. Triple-track GC is autonomous.",
                "",
                `*Tasks: ${tasks.length} | ${new Date().toISOString()}*`,
            ].join("\n");

            store.saveSession(text, { id: sessionId, created_at: new Date().toISOString(), zone_counts: snapshot.zone_counts, task_count: tasks.length });
            return { content: [{ type: "text", text: text }] };
        } catch (e: any) {
            return { isError: true, content: [{ type: "text", text: `calibrate error: ${e.message}` }] };
        }
    }

    // ── rename_tool ───────────────────────────────────────────────────────────
    if (name === "rename_tool") {
        const { from, to } = args as any;
        try {
            if (!/^[a-zA-Z][a-zA-Z0-9_-]*$/.test(to)) {
                return { isError: true, content: [{ type: "text", text: `Invalid alias: "${to}". Use alphanumeric, hyphens, underscores only.` }] };
            }
            const aliases = loadAliases();
            // Resolve the source through existing aliases
            const resolved = resolveToolName(from, aliases);
            // Verify the resolved name is a real tool
            const isCoreToolOrAlias = CORE_TOOLS.some(t => t.name === resolved) || aliases[resolved];
            if (!isCoreToolOrAlias && !CORE_TOOLS.some(t => t.name === from)) {
                return { isError: true, content: [{ type: "text", text: `Tool "${from}" not found. Available: ${CORE_TOOLS.map(t => t.name).join(", ")}` }] };
            }
            aliases[to] = resolved;
            saveAliases(aliases);
            return { content: [{ type: "text", text: [
                `✅ Alias registered: ${to} → ${resolved}`,
                "",
                `You can now call \`${to}()\` instead of \`${resolved}()\`.`,
                "",
                "Current aliases:",
                Object.entries(aliases).map(([a, t]) => `  ${a} → ${t}`).join("\n") || "  (none)",
            ].join("\n") }] };
        } catch (e: any) {
            return { isError: true, content: [{ type: "text", text: `rename_tool error: ${e.message}` }] };
        }
    }

    // ── list_aliases ──────────────────────────────────────────────────────────
    if (name === "list_aliases") {
        try {
            const aliases = loadAliases();
            if (Object.keys(aliases).length === 0) {
                return { content: [{ type: "text", text: [
                    "No aliases registered.",
                    "",
                    "Example: rename_tool({from: 'calibrate', to: 'vera'})",
                    "",
                    "Available tools:",
                    CORE_TOOLS.map(t => `  ${t.name}`).join("\n"),
                ].join("\n") }] };
            }
            return { content: [{ type: "text", text: [
                "# Tool Aliases",
                Object.entries(aliases).map(([a, t]) => `  ${a} → ${t}`).join("\n"),
                "",
                "Core tools:",
                CORE_TOOLS.map(t => `  ${t.name}`).join("\n"),
            ].join("\n") }] };
        } catch (e: any) {
            return { isError: true, content: [{ type: "text", text: `list_aliases error: ${e.message}` }] };
        }
    }

    throw new Error(`Unknown tool: ${name}. Call list_aliases() to see available tools.`);
});

// ─── Start ────────────────────────────────────────────────────────────────────

async function run() {
    const transport = new StdioServerTransport();
    await server.connect(transport);
    console.error("Verantyx Cortex MCP Server v3.1 running");

    // Background GC every 10 minutes
    setInterval(async () => {
        try {
            const { MemoryEngine } = await import("../memory/engine.js");
            const eng    = new MemoryEngine(ENGINE_ROOT);
            const report = eng.runAutonomousGc();
            const total  = report.classifier.length + report.coldEvictions.length +
                report.lruEvictions.reduce((s: number, r: { evicted: string[] }) => s + r.evicted.length, 0);
            if (total > 0) console.error(`[GC] Moved ${total} node(s)`);
        } catch (err: any) {
            console.error(`[GC] Error: ${err.message}`);
        }
    }, 10 * 60 * 1000);
}

run().catch(console.error);
