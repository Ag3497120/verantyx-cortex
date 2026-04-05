import { BrowserClient } from "../vfs/browser_client.js";
import { OllamaClient } from "../locallm/ollama-client.js";
import { HitlProxy } from "./hitl-proxy.js";

export class SubAgentDispatcher {
    private commander: OllamaClient;
    private browser: BrowserClient;
    private commanderModel: string;

    constructor(commander: OllamaClient, browser: BrowserClient, commanderModel: string) {
        this.commander = commander;
        this.browser = browser;
        this.commanderModel = commanderModel;
    }

    /**
     * Phase 1: MAP
     * Prompts the Local LLM Commander to dissect a large-scale architecture task
     * into discrete, stateless sub-tasks that can be executed independently.
     */
    async mapTask(globalTask: string, numChunks: number = 3): Promise<string[]> {
        console.log(`\n🧩 [SubAgentDispatcher] Map Phase: Asking Commander to split task into ${numChunks} chunks...`);

        const system = `You are a strict task orchestrator. 
The user has provided a massively large coding architecture task.
Split this task into independent AST micro-tasks (aim for ${numChunks} if logical).
DO NOT instruct the sub-agents to "write the whole file". Instead, output micro-tasks defining the exact Abstract Syntax Tree (AST) skeleton, such as the exact function signatures, directory paths, and dependencies they need to implement.
Do NOT output JSON arrays. Output your tasks using STRICT XML formatting enclosed in <micro_tasks> and <task> tags.
Example:
<micro_tasks>
  <task>Implement function verifyToken(token: string): boolean in src/auth.ts using jwt</task>
  <task>Create interface UserData in src/types.ts</task>
  <task>Draft the Express POST /login route utilizing verifyToken in src/api.ts</task>
</micro_tasks>`;

        const resp = await this.commander.chat(this.commanderModel, [
            { role: "system", content: system },
            { role: "user", content: `Global Task:\n${globalTask}` }
        ]);

        try {
            const chunks: string[] = [];
            const regex = /<task>([\s\S]*?)<\/task>/g;
            let match;
            while ((match = regex.exec(resp)) !== null) {
                chunks.push(match[1].trim());
            }

            if (chunks.length === 0) {
                console.warn(`  └─ Warning: No <task> tags matched. Falling back to simple split.`);
                return [globalTask];
            }

            console.log(`  └─ Success: Dissected via XML into ${chunks.length} modules.`);
            return chunks;
        } catch (err: any) {
            console.warn(`  └─ Extraction failed for Map Phase. Falling back to simple split.`);
            return [globalTask]; // Fallback to single chunk
        }
    }

    /**
     * Phase 2: REDUCE (Execution)
     * Takes the discrete sub-tasks, dispatches them sequentially to the Web browser sub-agents.
     */
    async executeSubTasks(chunks: string[], systemLanguage: "en" | "ja" = "en"): Promise<string[]> {
        const results: string[] = [];

        for (let i = 0; i < chunks.length; i++) {
            const chunk = chunks[i];
            console.log(`\n🚀 [SubAgentDispatcher] Executing Chunk ${i + 1}/${chunks.length}: ${chunk.substring(0, 40)}...`);

            // Using HitlProxy to pipe to Gemini/ChatGPT via the Private Web UI
            const systemPrompt = `[SUB-TASK ${i + 1}/${chunks.length}]\nYou are a sub-agent worker. Write the exact code for this discrete chunk. DO NOT output conversational filler. Provide only the implementation.`;
            
            // In a fully autonomous future, this would be an automated WebDriver,
            // but for now, we use the Free Browser HITL relay.
            const result = await HitlProxy.waitForHumanExecution(
                systemPrompt,
                chunk,
                this.browser,
                systemLanguage
            );

            results.push(result);
            console.log(`  └─ Chunk ${i + 1} completed (${result.length} characters).`);
            
            // Rebirth / Reset the browser automatically after each large chunk
            await this.browser.restart(true);
            await this.browser.navigate("https://gemini.google.com/app");
        }

        return results;
    }
}
