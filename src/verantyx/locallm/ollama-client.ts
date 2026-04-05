/**
 * Ollama Client
 * Direct HTTP binding to a local 11434 Ollama deployment
 */

export interface OllamaMessage {
    role: "user" | "assistant" | "system";
    content: string;
}

export class OllamaClient {
    private endpoint: string;

    constructor(host: string = "http://127.0.0.1:11434") {
        this.endpoint = host;
    }

    async chat(model: string, messages: OllamaMessage[]): Promise<string> {
        try {
            const resp = await fetch(`${this.endpoint}/api/chat`, {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({
                    model: model,
                    messages: messages,
                    stream: false,
                    options: {
                        num_ctx: 16000,
                    }
                })
            });

            if (!resp.ok) {
                throw new Error(`Ollama HTTP Error: ${resp.status}`);
            }

            const data = await resp.json();
            return data.message.content;
        } catch (err: any) {
            console.warn(`[OllamaClient] Connection failed. Is Ollama running on ${this.endpoint}?`);
            throw err; // Escalate so Orchestrator can fallback
        }
    }
}
