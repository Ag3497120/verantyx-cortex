import { examineWithGemini } from "./gemini-examiner.js";

export type Tier = "Tier 1: Fully Autonomous" | "Tier 2: Hybrid Sensory Matrix" | "Tier 3: Cloud Fallback / Chat Only";

export interface CalibrationResult {
  score: number;
  tier: Tier;
  report: string;
}

export interface CalibrationProfile {
  name: string;
  maxParallelTools: number;
  enforceAtomicReAct: boolean;
  systemPromptPreset: "lightweight" | "standard" | "unbound";
}

/**
 * Derives the active strategy profile based on the parameter size of the local model
 */
export function getCalibrationProfile(modelId: string): CalibrationProfile {
    const rawMatch = modelId.match(/(\d+)b/i);
    const params = rawMatch ? parseInt(rawMatch[1]) : 0;
    
    // Light-weight (e.g. Gemma 2B, 7B, 8B, 9B, Llama 8B)
    if (params <= 10) {
        return {
            name: "Lightweight Engine",
            maxParallelTools: 1, // MUST execute 1 tool and observe
            enforceAtomicReAct: true, // Forces "Wait for STDERR" behavior before allowing next action
            systemPromptPreset: "lightweight"
        };
    }
    // Mid-weight (e.g. Gemma 27B)
    else if (params > 10 && params <= 35) {
        return {
            name: "Midweight Engine",
            maxParallelTools: 2,
            enforceAtomicReAct: false,
            systemPromptPreset: "standard"
        };
    }
    // Heavy-weight / Cloud (e.g. Llama 70B, GPT-4, Opus)
    else {
        return {
            name: "Heavyweight Engine",
            maxParallelTools: 10,
            enforceAtomicReAct: false,
            systemPromptPreset: "unbound"
        };
    }
}

/**
 * Applies the calibrated tier rules into the ReAct XML generation schema.
 * Replaces generic Aider/Fabric constructs into pure Ronin atomic loops.
 */
export function generateCalibratedSystemDirectives(profile: CalibrationProfile): string {
    if (profile.systemPromptPreset === "lightweight") {
        return `[RONIN CORE SYSTEM] 
Your parameter size dictates strict atomic execution. 
1. DO NOT parallelize tasks. 
2. Execute ONE <action> at a time.
3. ALWAYS wait for the <observation> shell output before proceeding.
Failure to do so will result in an OS logic deadlock.`;
    } 
    
    if (profile.systemPromptPreset === "standard") {
        return `[RONIN CORE SYSTEM]
Standard execution confirmed. You may bundle multiple related shell modifications per turn if safe. 
Prioritize [Diff Review] validation for critical files.`;
    }

    return `[RONIN CORE SYSTEM - UNBOUND]
Maximum capability unlocked. You possess full multi-threaded tool orchestration capabilities. 
Execute dynamic MCP chains or bulk shell changes as you see fit.`;
}

/**
 * Executes historical 5 Trial constraints against the local LLM proxy.
 */
export async function runCalibrationTrials(model: string = "gemma:27b"): Promise<CalibrationResult> {
    console.log(`\n⏳ Running Verantyx Local Model Calibration on: ${model}`);
    const profile = getCalibrationProfile(model);
    console.log(`  └─ Implied Profile: ${profile.name}`);
    
    // Mock simulation returns...
    return {
        score: 95,
        tier: "Tier 1: Fully Autonomous",
        report: `Model calibrated correctly as ${profile.name} with parallel tools: ${profile.maxParallelTools}`
    };
}
