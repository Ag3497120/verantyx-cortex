use crate::actor::{Actor, Envelope};
use crate::messages::HiveMessage;
use async_trait::async_trait;
use tracing::{info, warn, debug};
use uuid::Uuid;
use ronin_core::models::provider::LlmProvider;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SystemRole {
    SeniorObserver,
    JuniorObserver,
}

pub struct StealthWebActor {
    pub id: Uuid,
    pub turn_limit: u8,
    pub current_turns: u8,
    global_access: bool,
    cwd: std::path::PathBuf,
    local_model: String,
    ollama_host: String,
    ollama_port: u16,
    pub is_japanese_mode: bool,
    pub role: SystemRole,
    pub tab_index: u8,
}

impl StealthWebActor {
    pub fn new(id: Uuid, global_access: bool, cwd: std::path::PathBuf, local_model: String, ollama_host: String, ollama_port: u16, is_japanese_mode: bool, role: SystemRole, tab_index: u8) -> Self {
        Self {
            id,
            turn_limit: 5, 
            current_turns: 0,
            global_access,
            cwd,
            local_model,
            ollama_host,
            ollama_port,
            is_japanese_mode,
            role,
            tab_index,
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

                let tab_index = self.tab_index;
                let run_js = |js: &str| -> String {
                    let escaped = js.replace("\\", "\\\\").replace("\"", "\\\"").replace("\n", "");
                    let script = format!("tell application \"Safari\"\nreturn do JavaScript \"{}\" in tab {} of window 1\nend tell", escaped, tab_index);
                    run_applescript(&script)
                };

                // Inject System Prompt Wrapper securely based on role
                let scope_instruction = if self.global_access {
                    "【重要】あなたは制限解除モードです。絶対パスを使ってPC内のすべてのディレクトリやファイルを読み書きできます。"
                } else {
                    "【制限】現在のプロジェクト直下のファイルにのみアクセス可能です。絶対パスや上位階層(../)へのアクセスは拒否されます。"
                };

                let mut timeline_content = String::new();
                let timeline_path = self.cwd.join(".ronin").join("timeline.md");
                
                if timeline_path.exists() {
                    timeline_content = std::fs::read_to_string(&timeline_path).unwrap_or_default();
                } else {
                    let _ = std::fs::create_dir_all(self.cwd.join(".ronin"));
                }

                let system_prompt = match self.role {
                    SystemRole::SeniorObserver => format!("
【SYSTEM: Senior Observer & Task Commander】
あなたは現在、デュアルAI体制の「シニア（主導）エージェント」です。
以下の時系列コンテキスト（Timeline History）とLocal Systemからの要約内容を分析し、ファイルの読み込みや書き換えの「行動提案」を行ってください。

--- TIMELINE HISTORY ---
{}
------------------------

【⚠️ 最重要ルール】
ローカルのシステムがあなたの指示を忠実に代行作業します。
ファイルの取得や編集が必要な場合は、以下の構文を使ってアシスタントに「依頼」してください。

1) ファイル読み込み依頼: 
REQUEST_READ_FILE: `/path/to/file`

2) コマンド実行依頼: 
REQUEST_EXEC: `echo hello`

3) ファイル編集依頼:
REQUEST_FILE_EDIT: `/path/to/file` <<<< 
ここに追加する機能や修正内容の日本語か英語の指示文を書いてください（例：「execute関数内にログ出力を追加して最適化してください」）。ローカルLLMがコードを完璧に書き直させます。
>>>>

【手順】
- 一度に依頼する作業は最大2つまで。
- すべての要件が完了した場合は [TASK_COMPLETE] と出力してください。
- Local Systemからの直近メッセージ:
{}
", timeline_content, objective),
                    SystemRole::JuniorObserver => format!("
【SYSTEM: Junior Observer & Validator】
あなたは現在、デュアルAI体制の「ジュニア（観測・検証）エージェント」です。
あなたの役割は、シニアエージェントが送ってきた行動提案と、Local Systemがまとめた要約が「ユーザーの意図に沿っているか」「矛盾がないか」を監査（Validate）することです。

--- TIMELINE HISTORY ---
{}
------------------------

【⚠️ ジュニアエージェントのルール】
あなたは **決して** `REQUEST_` コマンドを使用してファイルの取得や編集を提案してはいけません。
行動する権利はシニアエージェントにのみあります。
あなたのミッションは「観察結果」や「シニアの提案に対する同意・修正意見」を自然言語で述べることだけです。

直近のイベント/入力：{}
上記に従い、現状の状況分析や意見を端的に述べてください。
", timeline_content, objective)
                };

                let mut current_payload = system_prompt.clone();
                let mut final_output = String::new();
                let mut rollback_count = 0;
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

                    let is_safe_path = |path: &str, cwd: &std::path::Path, global: bool| -> bool {
                        if global { return true; }
                        let p = std::path::Path::new(path);
                        if p.is_absolute() { return p.starts_with(cwd); }
                        for comp in p.components() {
                            if comp == std::path::Component::ParentDir { return false; }
                        }
                        true
                    };

                    // REQUEST_READ_FILE
                    let read_re = regex::Regex::new(r"REQUEST_READ_FILE:\s*`([^`]+)`").unwrap();
                    for cap in read_re.captures_iter(&last_response) {
                        tools_used = true;
                        let path = &cap[1];
                        if !is_safe_path(path, &self.cwd, self.global_access) {
                            feedback.push_str(&format!("[SYS: DENIED] Sandbox Error: You are not permitted to access {} in Project-Only mode.\n\n", path));
                            continue;
                        }
                        info!("[StealthGemini-Tools] Simulating File Read: {}", path);
                        let full_path = self.cwd.join(path);
                        match std::fs::read_to_string(&full_path) {
                            Ok(c) => feedback.push_str(&format!("[SYS: Read {}]\n```\n{}\n```\n\n", path, c)),
                            Err(e) => feedback.push_str(&format!("[SYS: Error reading {}]: {}\n\n", path, e)),
                        }
                    }

                    // REQUEST_EXEC
                    let exec_re = regex::Regex::new(r"REQUEST_EXEC:\s*`([^`]+)`").unwrap();
                    for cap in exec_re.captures_iter(&last_response) {
                        tools_used = true;
                        let cmd = &cap[1];
                        println!("\n\x1b[33m⚡ Web Commander wants to execute: {}\x1b[0m", cmd);
                        print!("Allow? [y/N]: ");
                        std::io::Write::flush(&mut std::io::stdout()).unwrap();
                        let mut input = String::new();
                        std::io::stdin().read_line(&mut input).unwrap();
                        
                        if input.trim().eq_ignore_ascii_case("y") {
                            let out = std::process::Command::new("bash").arg("-c").arg(cmd).current_dir(&self.cwd).output();
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

                    // REQUEST_FILE_EDIT
                    let edit_re = regex::Regex::new(r"REQUEST_FILE_EDIT:\s*`([^`]+)`\s*<<<<\s*([\s\S]*?)\s*>>>>").unwrap();
                    for cap in edit_re.captures_iter(&last_response) {
                        tools_used = true;
                        let path = &cap[1];
                        let instruction = &cap[2];
                        
                        if !is_safe_path(path, &self.cwd, self.global_access) {
                            feedback.push_str(&format!("[SYS: DENIED] Sandbox Error: You are not permitted to edit {} in Project-Only mode.\n\n", path));
                            continue;
                        }
                        
                        println!("\n\x1b[33m⚡ Web Commander wants Local SLM to rewrite: {}\x1b[0m", path);
                        println!("Instruction: {}", instruction);
                        print!("Allow SLM Editor to run? [y/N]: ");
                        std::io::Write::flush(&mut std::io::stdout()).unwrap();
                        let mut input = String::new();
                        std::io::stdin().read_line(&mut input).unwrap();

                        if input.trim().eq_ignore_ascii_case("y") {
                            let full_path = self.cwd.join(path);
                            match std::fs::read_to_string(&full_path) {
                                Ok(content) => {
                                    println!("⚙️ Handing over to Local SLM ({}) to perform exact rewrite...", self.local_model);
                                    let provider = ronin_core::models::provider::ollama::OllamaProvider::new(
                                        &self.ollama_host,
                                        self.ollama_port
                                    );
                                    let req = ronin_core::models::sampling_params::InferenceRequest {
                                        model: self.local_model.clone(),
                                        format: ronin_core::models::sampling_params::PromptFormat::OllamaChat,
                                        stream: false,
                                        sampling: ronin_core::models::sampling_params::SamplingParams::for_heavyweight().with_temperature(0.0),
                                    };
                                    let hist = vec![
                                        ronin_core::models::provider::LlmMessage {
                                            role: "system".to_string(),
                                            content: "You are the Ronin Code HAND. You receive the original file and an edit instruction. Output ONLY the FULL, freshly rewritten file text. Do not use Markdown backticks. Do not add explanations. Your output will overwrite the original file directly. Start writing the raw text immediately.".to_string(),
                                        },
                                        ronin_core::models::provider::LlmMessage {
                                            role: "user".to_string(),
                                            content: format!("[FILE CONTENT]\n{}\n[INSTRUCTION]\n{}", content, instruction),
                                        }
                                    ];
                                    
                                    match provider.invoke(&req, &hist).await {
                                        Ok(new_code) => {
                                            std::fs::write(&full_path, new_code).unwrap_or_default();
                                            println!("✅ Local SLM rewrote the file successfully!");
                                            feedback.push_str(&format!("[SYS: Local SLM successfully rewrote and patched {} based on your instruction]\nStatus: SUCCESS\n\n", path));
                                        }
                                        Err(e) => {
                                            println!("❌ Local SLM generation failed: {}", e);
                                            feedback.push_str(&format!("[SYS: Local SLM Patch Failed {}]\nREASON: {}\n\n", path, e));
                                        }
                                    }
                                }
                                Err(e) => feedback.push_str(&format!("[SYS: Patch Failed {}]\nREASON: Could not read file. {}\n\n", path, e)),
                            }
                        } else {
                            feedback.push_str(&format!("[SYS: DENIED] File Edit on '{}' was aborted by Human Operator.\n\n", path));
                        }
                    }

                    if !tools_used {
                        let has_japanese = last_response.chars().any(|c| matches!(c, '\u{3040}'..='\u{309F}' | '\u{30A0}'..='\u{30FF}'));
                        if self.is_japanese_mode && !has_japanese && rollback_count < 2 {
                            info!("[StealthGemini-{}] Foreign language final response detected in Japanese Mode. Forcing translation rollback.", self.id);
                            rollback_count += 1;
                            feedback.push_str("[SYS REJECT: Your entire response was in English despite the System Language being Japanese. Completely translate your previous response into natural Japanese and output it again. Do NOT output code unless absolutely necessary.]\n\n");
                        } else {
                            info!("[StealthGemini-{}] No tools detected. Yielding final response.", self.id);
                            final_output = last_response;
                            break;
                        }
                    } else {
                        rollback_count = 0; // Reset rollback if they successfully used tools
                    }

                    self.current_turns += 1;
                    if self.current_turns >= self.turn_limit {
                        info!("[StealthGemini-{}] Reached {} turns. Resetting Web Session to evade detection/context-bloat.", self.id, self.turn_limit);
                        let _ = run_js("window.location.href = 'https://gemini.google.com/app';");
                        std::thread::sleep(std::time::Duration::from_secs(4));
                        self.current_turns = 0;
                        
                        // We must inject the system prompt again into the fresh chat, along with the recent context
                        current_payload = format!("{}\n\n[SYSTEM RECOVERY - YOUR PREVIOUS CHAT WAS RESET FOR MEMORY LIMITS]\nContinue where you left off. Feedback from previous operations:\n{}\nPlease proceed with next step or output [TASK_COMPLETE].", system_prompt, feedback);
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
                    sender: match self.role {
                        SystemRole::SeniorObserver => "SeniorGemini".to_string(),
                        SystemRole::JuniorObserver => "JuniorGemini".to_string(),
                    },
                    recipient: env.sender,
                    payload: serde_json::to_string(&result)?,
                }))
            },
            _ => {
                Ok(None)
            }
        }
    }
}
