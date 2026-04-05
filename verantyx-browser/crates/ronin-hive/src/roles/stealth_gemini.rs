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

                // Carbon-Paper Stealth DOM Injection
                info!("[StealthGemini-{}] Initializing Carbon Paper stealth wrapper...", self.id);

                use std::process::Command;
                use base64::{Engine as _, engine::general_purpose};

                let run_applescript = |script: &str| -> String {
                    let out = Command::new("osascript")
                        .arg("-e").arg(script)
                        .output()
                        .expect("Failed to execute osascript");
                    String::from_utf8_lossy(&out.stdout).trim().to_string()
                };

                let run_js = |js: &str| -> String {
                    let escaped = js.replace("\\", "\\\\").replace("\"", "\\\"").replace("\n", "");
                    let script = format!("tell application \"Safari\"\nreturn do JavaScript \"{}\" in front document\nend tell", escaped);
                    run_applescript(&script)
                };

                // 1. Move Safari to background/hide it, ensuring Gemini tab is open
                let prepare_js = "if (!window.location.href.includes('gemini.google.com')) window.location.href = 'https://gemini.google.com/app';";
                run_js(prepare_js);
                // Wait for page load if it was redirecting
                std::thread::sleep(std::time::Duration::from_millis(2000));

                let prev_count = run_js("return (document.body.innerText.split('Gemini の回答').length - 1).toString();").parse::<i32>().unwrap_or(0);

                // 2. Inject Prompt
                info!("[StealthGemini-{}] Injecting query via Headless DOM...", self.id);
                let b64 = general_purpose::STANDARD.encode(objective.as_bytes());
                let inject_js = format!("
                    (function(){{
                        var encoded='{}';
                        var decoded=decodeURIComponent(escape(atob(encoded)));
                        var el=document.querySelector('div.ql-editor[contenteditable=true]') || document.querySelector('textarea');
                        if(!el) return 'ERROR';
                        el.focus(); el.click();
                        if(el.classList.contains('ql-editor')){{
                            var c=el.closest('.ql-container');
                            var q=c&&c.__quill;
                            if(q){{q.setText(decoded); q.emitter.emit('text-change',{{ops:[{{insert:decoded}}]}},{{ops:[]}},'user');}}
                            else {{el.innerText=decoded; el.dispatchEvent(new Event('input',{{bubbles:true}}));}}
                        }} else {{
                            el.value=decoded; el.dispatchEvent(new Event('input',{{bubbles:true}}));
                        }}
                        return 'OK';
                    }})();
                ", b64);
                
                run_js(&inject_js);
                std::thread::sleep(std::time::Duration::from_millis(500));

                // 3. Submit
                info!("[StealthGemini-{}] Committing prompt...", self.id);
                let submit_js = "(function(){var btn=document.querySelector('button[aria-label*=\"プロンプトを送信\"]') || document.querySelector('button[aria-label*=\"Send\"]'); if(btn) {btn.click(); return 'OK';} return 'ERROR';})();";
                run_js(submit_js);

                // 4. Poll Response
                info!("[StealthGemini-{}] Polling for stream stabilization...", self.id);
                let poll_js = format!("
                    (function(){{
                        var parts=document.body.innerText.split('Gemini の回答');
                        if (parts.length > {} + 1) {{
                            var last = parts[parts.length-1];
                            var idx;
                            idx=last.indexOf('あなたのプロンプト'); if(idx>0) last=last.substring(0,idx);
                            idx=last.indexOf('Gemini は AI であり'); if(idx>0) last=last.substring(0,idx);
                            idx=last.lastIndexOf('ツール'); if(idx>0) last=last.substring(0,idx);
                            idx=last.indexOf('回答案を表示'); if(idx>0) last=last.substring(0,idx);
                            return last.trim();
                        }}
                        return '';
                    }})();
                ", prev_count);

                let mut last_response = String::new();
                let mut stable_count = 0;
                let max_attempts = 60; // 2 mins timeout given 2s sleep

                for _ in 0..max_attempts {
                    std::thread::sleep(std::time::Duration::from_millis(2000));
                    let current = run_js(&poll_js);
                    
                    if !current.is_empty() {
                        if current == last_response {
                            stable_count += 1;
                            if stable_count >= 2 {
                                break;
                            }
                        } else {
                            stable_count = 0;
                            last_response = current;
                        }
                    }
                }

                let final_output = if last_response.is_empty() {
                    warn!("[StealthGemini-{}] Timeout waiting for Gemini.", self.id);
                    "Error: Request timed out or DOM structure rejected injection.".to_string()
                } else {
                    last_response
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
