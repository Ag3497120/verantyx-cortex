import { BrowserClient } from "../vfs/browser_client.js";

export class HitlProxy {
  /**
   * Prompts the user to manually proxy a payload via their Browser/Gemini UI,
   * completely bypassing API keys and saving costs.
   * Uses AutoHarvest to automatically extract the DOM when typing pauses for 2 seconds.
   */
  static async waitForHumanExecution(systemPrompt: string, userMessage: string, browserClient: BrowserClient, lang: "en" | "ja" = "en"): Promise<string> {
    const fullPayload = `${systemPrompt}\n\n---\n\nUser Request: ${userMessage}`;

    const isJa = lang === "ja";
    
    console.log("\n==================================================");
    console.log(isJa ? "⚠️  [Browser HITL モード] 自動抽出（Auto-Harvest）起動" : "⚠️  [Browser HITL Mode] Auto-Harvest Enabled");
    console.log("==================================================");
    console.log(isJa ? "1. ブラウザがGemini Web UIを開きます。" : "1. The browser is opening to Gemini Web UI.");
    console.log(isJa ? "2. 【自動コピー完了】プロンプトは既にクリップボードに入っています。" : "2. 【COPIED TO CLIPBOARD】 Automatically injected into your clipboard.");
    console.log(isJa ? "3. ブラウザの入力欄で Cmd+V / Ctrl+V を押してペーストしてください。" : "3. Just press Cmd+V / Ctrl+V in the browser input box.");
    console.log(isJa ? "4. 自由に微修正を加えた後、送信ボタンを押してください。" : "4. You may edit the prompt before pressing Submit in Gemini.");
    console.log(isJa ? "\n(Auto-Harvestは、Geminiがタイピングを停止して2秒後に自動でデータを抽出します...)" : "\n(Auto-Harvest is listening for 2 seconds of silence after Gemini types...)");
    console.log("------------------- PAYLOAD PREVIEW -------------------");
    console.log(fullPayload.slice(0, 300) + (fullPayload.length > 300 ? "\n... (truncated for display) ..." : ""));
    console.log("-------------------------------------------------------\n");

    try {
      // Execute OS-level clipboard copy (macOS pbcopy)
      const { spawnSync } = await import("child_process");
      spawnSync("pbcopy", { input: fullPayload, encoding: "utf-8" });
    } catch (e) {
      console.log(isJa ? "クリップボードの自動コピーに失敗しました。手動でコピーしてください。" : "Auto-clipboard failed. Please copy manually.");
    }

    console.log(isJa ? "⏳ 自動DOM抽出を待機しています..." : "⏳ Waiting for automated DOM extraction...");
    const resp = await browserClient.waitForAutoHarvest();
    console.log(isJa ? "\n✅ 完了！DOMを正常に抽出しました。" : "\n✅ Typing paused. Extracted DOM successfully!");
    
    return resp.markdown || (isJa ? "テキストが抽出されませんでした" : "No text extracted");
  }
}
