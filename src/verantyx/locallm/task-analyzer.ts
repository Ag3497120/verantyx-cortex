import { Tier } from "./calibrate.js";

export interface TaskContext {
  domain: "web_scraping" | "reasoning" | "coding" | "planning" | "general_chat";
  complexity: "low" | "medium" | "high";
  requiredTier: 1 | 2 | 3;
}

/**
 * Task Analyzer uses the Local LLM (or falls back to rules) 
 * to determine the complexity and required abilities for a task.
 */
export async function analyzeTaskComplexity(userMessage: string, localModelUrl: string = "http://127.0.0.1:11434/api/generate"): Promise<TaskContext> {
    // In a full production setup, we would actually fetch() the local model with a JSON schema
    // instructing it to evaluate itself. For stability without spinning up Ollama here, 
    // we use a sophisticated heuristic text-analysis fallback to simulate the self-reflection.
    
    const msg = userMessage.toLowerCase();
    
    let domain: TaskContext["domain"] = "general_chat";
    let complexity: TaskContext["complexity"] = "low";
    let requiredTier: 1 | 2 | 3 = 3; // Default Tier 3 (Pure Chat) 

    // Heuristics for Web Scraping / Navigation
    if (msg.includes("scrape") || msg.includes("browse") || msg.includes("search") || msg.includes("url") || msg.includes("site") || msg.includes("web")) {
        domain = "web_scraping";
        complexity = "medium";
        requiredTier = 2; // Needs at least Tier 2 (Sensory Matrix)
    }

    // Heuristics for Coding / Action execution
    if (msg.includes("code") || msg.includes("fix") || msg.includes("refactor") || msg.includes("implement") || msg.includes("script")) {
        domain = "coding";
        complexity = "high";
        requiredTier = 1; // Needs Tier 1 (Fully Autonomous Tool usage)
    }

    // Heuristics for Deep Planning / Architecture
    if (msg.includes("plan") || msg.includes("architect") || msg.includes("design") || msg.includes("structure")) {
        domain = "planning";
        complexity = "high";
        requiredTier = 1; // Needs Tier 1
    }

    // Heuristics for General Reasoning / Summarization
    if (msg.includes("explain") || msg.includes("summarize") || msg.includes("why did") || msg.includes("what is")) {
        domain = "reasoning";
        if (complexity !== "high") { 
            complexity = "low";
            requiredTier = 3;
        }
    }

    // Override required Tier if user specifically mentions massive actions
    if (msg.includes("autonomously") || msg.includes("take action") || msg.includes("command")) {
        requiredTier = 1;
        complexity = "high";
    }

    return {
        domain,
        complexity,
        requiredTier
    };
}

export function parseTierToNumber(tier: Tier): number {
    if (tier.includes("Tier 1")) return 1;
    if (tier.includes("Tier 2")) return 2;
    return 3;
}
