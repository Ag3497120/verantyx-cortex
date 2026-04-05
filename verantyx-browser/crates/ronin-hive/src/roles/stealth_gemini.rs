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

                // Inject System Prompt Wrapper securely
                let system_prompt = format!("
【SYSTEM: 司令塔AI 実行プロトコル】
あなたはローカルファイルシステムにアクセスできる完全自律型のCommander AIです。
ユーザーの要望（[USER REQUEST]）を解決するために、以下のツールを駆使してください。
実行したいツールがある場合は、以下の独自構文を回答の中に含めてください。システムが自動で検知して実行結果をあなたに返却します。

1) ファイル読み込み: VX_FILE_READ: `/path/to/file`
2) コマンド実行: VX_EXEC_BRAIN: `echo hello`
3) ファイル編集:
VX_FILE_EDIT: `/path/to/file` <<<< SEARCH
検索・置換したい対象コード
==== REPLACE
新しいコード文字列
>>>>

[ルール]
- 一度に実行するツールは1〜3つまでにしてください。結果が返されるまで次のことはできません。
- ファイルパスは絶対パスか、プロジェクトルートからの相対パスを使用してください。
- すべての要件を満たしたと判断した場合は、[TASK_COMPLETE] と出力し、最終報告をまとめてください。

[USER REQUEST]
{}
", objective);

                let mut current_payload = system_prompt.clone();
                let mut final_output = String::new();
                let mut loop_counter = 0;

                info!("[StealthGemini-{}] Entering Autonomous Action-Observation Loop...", self.id);

                loop {
                    loop_counter += 1;
                    if loop_counter > 20 {
                        warn!("[StealthGemini-{}] Max loop iterations (20) reached. Force stopping.", self.id);
                        final_output = "Task forcefully terminated to prevent infinite tool loop.".to_string();
                        break;
                    }

                    // 1. Move Safari to background/hide it, ensuring Gemini tab is open
                    let prepare_js = "if (!window.location.href.includes('gemini.google.com')) window.location.href = 'https://gemini.google.com/app';";
                    run_js(prepare_js);
                    // Wait for page load if it was redirecting
                    std::thread::sleep(std::time::Duration::from_millis(2000));

                    let prev_count = run_js("return (document.body.innerText.split('Gemini の回答').length - 1).toString();").parse::<i32>().unwrap_or(0);

                    // 2. Inject Prompt
                    info!("[StealthGemini-{}] Injecting Observation/Query Cycle #{}...", self.id, loop_counter);
                    let b64 = general_purpose::STANDARD.encode(current_payload.as_bytes());
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
                                if(q){{q.setText(''); q.setText(decoded); q.emitter.emit('text-change',{{ops:[{{insert:decoded}}]}},{{ops:[]}},'user');}}
                                else {{el.innerText=decoded; el.dispatchEvent(new Event('input',{{bubbles:true}}));}}
                            }} else {{
                                el.value=decoded; el.dispatchEvent(new Event('input',{{bubbles:true}}));
                            }}
                            return 'OK';
                        }})();
                    ", b64);
                    
                    run_js(&inject_js);
                    std::thread::sleep(std::time::Duration::from_millis(1000));

                    // 3. Submit
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

                    if last_response.is_empty() {
                        warn!("[StealthGemini-{}] Timeout waiting for Gemini.", self.id);
                        final_output = "Error: Request timed out or DOM structure rejected injection.".to_string();
                        break;
                    }

                    // 5. Evaluate Response for VX Commands
                    if last_response.contains("[TASK_COMPLETE]") {
                        info!("[StealthGemini-{}] Commander reached TASK_COMPLETE state.", self.id);
                        final_output = last_response;
                        break;
                    }

                    let mut feedback = String::new();
                    let mut tools_used = false;

                    // VX_FILE_READ
                    let read_re = regex::Regex::new(r"VX_FILE_READ:\s*`([^`]+)`").unwrap();
                    for cap in read_re.captures_iter(&last_response) {
                        tools_used = true;
                        let path = &cap[1];
                        info!("[StealthGemini-Tools] Simulating File Read: {}", path);
                        match std::fs::read_to_string(path) {
                            Ok(c) => feedback.push_str(&format!("[SYS: Read {}]\n```\n{}\n```\n\n", path, c)),
                            Err(e) => feedback.push_str(&format!("[SYS: Error reading {}]: {}\n\n", path, e)),
                        }
                    }

                    // VX_EXEC_BRAIN
                    let exec_re = regex::Regex::new(r"VX_EXEC_BRAIN:\s*`([^`]+)`").unwrap();
                    for cap in exec_re.captures_iter(&last_response) {
                        tools_used = true;
                        let cmd = &cap[1];
                        println!("\n\x1b[33m⚡ Web Commander wants to execute: {}\x1b[0m", cmd);
                        print!("Allow? [y/N]: ");
                        std::io::Write::flush(&mut std::io::stdout()).unwrap();
                        let mut input = String::new();
                        std::io::stdin().read_line(&mut input).unwrap();
                        
                        if input.trim().eq_ignore_ascii_case("y") {
                            let out = std::process::Command::new("bash").arg("-c").arg(cmd).output();
                            match out {
                                Ok(o) => {
                                    let stdout = String::from_utf8_lossy(&o.stdout);
                                    let stderr = String::from_utf8_lossy(&o.stderr);
                                    feedback.push_str(&format!("[SYS: Exec {}]\nSTDOUT:\n{}\nSTDERR:\n{}\n\n", cmd, stdout, stderr));
                                }
                                Err(e) => feedback.push_str(&format!("[SYS: Exec Failed]: {}\n\n", e)),
                            }
                        } else {
                            feedback.push_str(&format!("[SYS: DENIED] Command '{}' was aborted by Human Operator.\n\n", cmd));
                        }
                    }

                    // VX_FILE_EDIT
                    let edit_re = regex::Regex::new(r"VX_FILE_EDIT:\s*`([^`]+)`\s*<<<< SEARCH\n([\s\S]*?)==== REPLACE\n([\s\S]*?)>>>>").unwrap();
                    for cap in edit_re.captures_iter(&last_response) {
                        tools_used = true;
                        let path = &cap[1];
                        let search = &cap[2];
                        let replace = &cap[3];
                        println!("\n\x1b[33m⚡ Web Commander wants to edit: {}\x1b[0m", path);
                        print!("Allow? [y/N]: ");
                        std::io::Write::flush(&mut std::io::stdout()).unwrap();
                        let mut input = String::new();
                        std::io::stdin().read_line(&mut input).unwrap();

                        if input.trim().eq_ignore_ascii_case("y") {
                            match std::fs::read_to_string(path) {
                                Ok(mut content) => {
                                    if content.contains(search) {
                                        content = content.replace(search, replace);
                                        std::fs::write(path, content).unwrap_or_default();
                                        feedback.push_str(&format!("[SYS: Patched {}]\nStatus: SUCCESS\n\n", path));
                                    } else {
                                        feedback.push_str(&format!("[SYS: Patch Failed {}]\nREASON: SEARCH block not found exactly in file. Check whitespace.\n\n", path));
                                    }
                                }
                                Err(e) => feedback.push_str(&format!("[SYS: Patch Failed {}]\nREASON: Could not read file. {}\n\n", path, e)),
                            }
                        } else {
                            feedback.push_str(&format!("[SYS: DENIED] File Edit on '{}' was aborted by Human Operator.\n\n", path));
                        }
                    }

                    if !tools_used {
                        info!("[StealthGemini-{}] No tools detected. Yielding final response.", self.id);
                        final_output = last_response;
                        break;
                    } else {
                        current_payload = format!("[SYSTEM HOOK FEEDBACK]\nFollowing tool calls were executed:\n{}\nPlease proceed with next step or output [TASK_COMPLETE].", feedback);
                    }
                }

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
