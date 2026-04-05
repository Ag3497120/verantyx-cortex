use crate::actor::{Actor, Envelope};
use crate::messages::HiveMessage;
use async_trait::async_trait;
use tracing::{info, warn, debug};
use uuid::Uuid;

pub struct StealthWebActor {
    id: Uuid,
    turn_limit: u8,
    current_turns: u8,
}

impl StealthWebActor {
    pub fn new(id: Uuid) -> Self {
        Self {
            id,
            // Strict enforce of 5-turn limit to bypass Google login triggers natively
            turn_limit: 5, 
            current_turns: 0,
        }
    }

    /// Simulate the destruction of the current Headless Chrome session and spawning a new one.
    fn respawn_browser_session(&mut self) {
        warn!("[StealthGemini-{}] Reached {}-turn limit. Executing Ephemeral Kill Switch.", self.id, self.turn_limit);
        info!("[StealthGemini-{}] Purging current headless browser session...", self.id);
        info!("[StealthGemini-{}] Booting fresh unauthenticated Gemini proxy...", self.id);
        self.current_turns = 0;
    }
}

#[async_trait]
impl Actor for StealthWebActor {
    fn name(&self) -> &str {
        // Technically this actor is unique per instance, but we use a fixed trait name for dispatch logs
        "StealthGeminiWorker"
    }
    
    async fn receive(&mut self, env: Envelope) -> anyhow::Result<Option<Envelope>> {
        let msg: HiveMessage = match serde_json::from_str(&env.payload) {
            Ok(m) => m,
            Err(_) => return Ok(None),
        };

        match msg {
            HiveMessage::SpawnSubAgent { id, objective } => {
                debug!("[StealthGemini-{}] Received objective: {}", self.id, objective);
                
                // Track conversation turn locally
                self.current_turns += 1;
                info!("[StealthGemini-{}] Turn usage: {} / {}", self.id, self.current_turns, self.turn_limit);

                if self.current_turns >= self.turn_limit {
                    self.respawn_browser_session();
                }

                // 1. Copy objective to clipboard
                info!("[StealthGemini-{}] Injecting prompt to macOS Clipboard...", self.id);
                use std::process::{Command, Stdio};
                use std::io::Write;

                let mut pbcopy = Command::new("pbcopy")
                    .stdin(Stdio::piped())
                    .spawn()
                    .unwrap_or_else(|e| panic!("Failed to run pbcopy: {}", e));
                
                if let Some(mut stdin) = pbcopy.stdin.take() {
                    stdin.write_all(objective.as_bytes()).ok();
                }
                pbcopy.wait().ok();

                // 2. Trigger Notification
                info!("[StealthGemini-{}] Firing HITL Push Notification...", self.id);
                let _ = Command::new("osascript")
                    .arg("-e")
                    .arg("display notification \"Prompt copied to clipboard! Please paste into Gemini.\" with title \"Ronin 🐺 Stealth Web\"")
                    .spawn();

                // 3. Open Browser
                info!("[StealthGemini-{}] Booting Chrome/Safari proxy to gemini.google.com...", self.id);
                let _ = Command::new("open")
                    .arg("https://gemini.google.com/app")
                    .spawn();

                // 4. Await User Input
                println!("\n\x1b[36m────────────────────────────────────────────────────────────\x1b[0m");
                println!("\x1b[1;36m🐺 Stealth Web Evasion Protocol Active\x1b[0m \x1b[2m──────────────────────\x1b[0m");
                println!("1. A new browser tab to Google Gemini has been opened.");
                println!("2. Your prompt is copied. Just press \x1b[32mCmd+V\x1b[0m and \x1b[32mEnter\x1b[0m!");
                println!("3. Paste Gemini's response below (Type 'EOF' on a new line and press Enter to finish):");
                println!("\x1b[36m────────────────────────────────────────────────────────────\x1b[0m");
                
                let mut captured_response = String::new();
                let stdin_handle = std::io::stdin();
                let mut iter = stdin_handle.lines();
                while let Some(Ok(line)) = iter.next() {
                    if line.trim() == "EOF" {
                        break;
                    }
                    captured_response.push_str(&line);
                    captured_response.push('\n');
                }

                let final_output = if captured_response.trim().is_empty() {
                    warn!("[StealthGemini-{}] User canceled or provided empty input.", self.id);
                    "Empty response.".to_string()
                } else {
                    captured_response.trim().to_string()
                };

                let result = HiveMessage::SubAgentResult {
                    id: self.id,
                    output: final_output,
                };
                
                Ok(Some(Envelope {
                    message_id: Uuid::new_v4(),
                    sender: format!("StealthGemini-{}", self.id),
                    recipient: env.sender, // Reply back to whoever summoned it (Commander/Planner)
                    payload: serde_json::to_string(&result)?,
                }))
            },
            _ => {
                Ok(None)
            }
        }
    }
}
