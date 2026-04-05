import { MemoryEngine } from "./engine.js";

const GEMINI_API_KEY = "AIzaSyATPOY0fmk94_bOWvkj13tvXGIegyjsZKE"; 

export async function extractEternalContext(sessionTurns: {role: string, content: string}[], engineRoot: string): Promise<void> {
    console.log("\n🧠 [Auto Selector] Scanning Session for Eternal Context Extraction via Gemini 3 Flash...");
    
    if (sessionTurns.length === 0) return;

    const engine = new MemoryEngine(engineRoot);
    const fullText = sessionTurns.map(t => `${t.role}: ${t.content}`).join("\n");
    const timestamp = Date.now();

    const prompt = `
You are the Eternal Context Memory Extractor for Verantyx.
Analyze the following session log and extract key architectural decisions, hard constraints, proven facts, or failed lessons.
You must output ONLY valid JSON in the following format:
{
  "extractions": [
    {
      "category": "front" | "near" | "mid" | "deep",
      "filename": "string_without_extension",
      "content_markdown": "Detailed markdown explanation"
    }
  ]
}

Categories meaning:
- front: Active/Current design constraints or operational parameters explicitly defined.
- near: Lessons learned, failed paths (e.g. system crashed when doing X).
- mid: Officially validated facts or completed large-scale design shifts.
- deep: Foundational knowledge or core truths established.

Log:
${fullText}
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
            
            let extractedCount = 0;
            if (parsed.extractions && Array.isArray(parsed.extractions)) {
                for (const item of parsed.extractions) {
                    engine.write(item.category, `${item.filename}_${timestamp}`, item.content_markdown);
                    console.log(`  -> Saved extraction: ${item.category}/${item.filename}_${timestamp}.md`);
                    extractedCount++;
                }
            }
            if (extractedCount === 0) {
                console.log("  -> No context thresholds reached. Dismissing ephemeral session.");
            }
        } else {
            console.log("  -> Gemini failed to classify session context.");
        }
    } catch (e: any) {
        console.error("  -> Network error during Eternal Context Extraction:", e.message);
    }
}

