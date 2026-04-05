import * as p from "@clack/prompts";
import { writeFileSync, existsSync, mkdirSync } from "fs";
import { join } from "path";
import { loadConfig, RoninAgentConfig } from "../../config.js";

export async function runRoninSetup() {
  console.clear();
  
  p.intro(`◇  Welcome to Ronin Agent System`);

  // 1. Language Selection
  const lang = await p.select({
    message: "Select your language / 言語を選択してください:",
    options: [
      { value: "en", label: "English" },
      { value: "ja", label: "日本語" }
    ],
  });

  if (p.isCancel(lang)) {
    p.cancel("Setup cancelled.");
    process.exit(0);
  }

  const isEn = lang === "en";

  // 2. Fallback Mode Selection
  const mode = await p.select({
    message: isEn 
      ? "Choose your cloud inference fallback mode:" 
      : "クラウド推論を使用する際のモードを選択してください:",
    options: [
      { 
        value: "browser_hitl", 
        label: isEn 
          ? "Browser Relay (Free & Stealth) — Uses custom Web UI to extract data. No API cost." 
          : "ブラウザリレーモード (無料＆ステルス) — ブラウザを裏で開いて無料で推論を利用します。" 
      },
      { 
        value: "api", 
        label: isEn 
          ? "API Mode (Paid & Fast) — Standard headless Anthropic/Gemini API execution."
          : "APIモード (有料＆高速) — 従来のAPI接続を利用する標準的なモードです。" 
      }
    ],
  });

  if (p.isCancel(mode)) {
    p.cancel(isEn ? "Setup cancelled." : "セットアップをキャンセルしました。");
    process.exit(0);
  }

  // Load existing config or defaults
  const config = loadConfig();
  
  config.agents.systemLanguage = lang as "en" | "ja";
  config.agents.cloudFallbackMode = mode as "api" | "browser_hitl";

  // Save to ~/.ronin/config.json
  const roninDir = join(process.env.HOME || "", ".ronin");
  if (!existsSync(roninDir)) {
    mkdirSync(roninDir, { recursive: true });
  }

  writeFileSync(
    join(roninDir, "config.json"),
    JSON.stringify(config, null, 2),
    "utf-8"
  );

  p.outro(
    isEn 
      ? `◇  Setup complete! Configuration saved to ${join(roninDir, "config.json")}\n   You are ready to roam.`
      : `◇  セットアップ完了！設定は ${join(roninDir, "config.json")} に保存されました。\n   浪人、出陣の準備が整いました。`
  );
}
