import type { Tier } from "./calibrate.js";

const GEMINI_API_KEY = "AIzaSyATPOY0fmk94_bOWvkj13tvXGIegyjsZKE"; 

export async function examineWithGemini(localOutput: {
    t1: string;
    t2: string;
    t3: string;
    tps: number;
}): Promise<{ score: number; commentary: string; tier: Tier }> {
    const prompt = `
You are the Verantyx Calibration Examiner. You evaluate Local LLM outputs to determine if they are safe to use as fully autonomous agents.
Evaluate these Trial Outputs:
Trial 1 (Strict JSON Name/Age): ${localOutput.t1}
Trial 2 (Tool Calling syntax): ${localOutput.t2}
Trial 3 (Spatial JCross Noise extraction): ${localOutput.t3}
Trial 4 (Tokens Per Second): ${localOutput.tps}

Grading Rubric (Total 100):
- T1 (30 pts): MUST BE EXACT JSON. No "here is your JSON". Only {"name": "Aria", "age": 32}. Subtract 15 pts if there is conversational noise. Sub 30 if failed.
- T2 (30 pts): Must be structured correctly like a function call or valid Tool Use JSON. Sub 15 if sloppy.
- T3 (30 pts): Must be exactly "CLICK". If it says "The action is CLICK", subtract 15 pts.
- T4 (10 pts): If TPS < 5, subtract 10 pts. If TPS < 10, subtract 5.

Tiers:
- Tier 1: > 85
- Tier 2: > 50
- Tier 3: <= 50

Return ONLY JSON:
{
  "score": number,
  "commentary": "Short explanation",
  "tier": 1 | 2 | 3
}
    `.trim();

    try {
        const response = await fetch(`https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-pro:generateContent?key=${GEMINI_API_KEY}`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                contents: [{ parts: [{ text: prompt }] }],
                generationConfig: { response_mime_type: "application/json" }
            })
        });

        const data = await response.json();
        
        if (data.candidates && data.candidates[0].content) {
            const raw = data.candidates[0].content.parts[0].text;
            const parsed = JSON.parse(raw);
            
            let tierOutput: Tier = "Tier 3: Cloud Fallback / Chat Only";
            if (parsed.tier === 1) tierOutput = "Tier 1: Fully Autonomous";
            if (parsed.tier === 2) tierOutput = "Tier 2: Hybrid Sensory Matrix";

            return {
                score: parsed.score,
                tier: tierOutput,
                commentary: parsed.commentary
            };
        }
    } catch (e: any) {
        console.error("[Examiner] API Error: " + e.message);
    }
    
    return {
        score: 0,
        tier: "Tier 3: Cloud Fallback / Chat Only",
        commentary: "Examination failed due to Network or API constraints. Defaulted to lowest tier."
    };
}
