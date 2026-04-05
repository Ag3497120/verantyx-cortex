use crate::actor::{Actor, Envelope};
use crate::messages::HiveMessage;
use async_trait::async_trait;
use tracing::{info, warn, debug};
use tokio::sync::Mutex;

lazy_static::lazy_static! {
    static ref CLI_INTERACT_MUTEX: Mutex<()> = Mutex::new(());
}
use uuid::Uuid;
use ronin_core::models::provider::LlmProvider;
use vx_dom::Document;
use vx_render::ai_renderer::AiRenderer;

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
            HiveMessage::SpawnSubAgent { id: _, objective } | HiveMessage::Objective(objective) => {
                debug!("[StealthGemini-{}] Received objective: {}", self.id, objective);
                
                // Track conversation turn locally
                self.current_turns += 1;
                info!("[StealthGemini-{}] Turn usage: {} / {}", self.id, self.current_turns, self.turn_limit);

                if self.current_turns >= self.turn_limit {
                    self.respawn_browser_session();
                }

                info!("[StealthGemini-{}] Initializing Carbon Paper stealth wrapper...", self.id);

                

                let _tab_index = self.tab_index;
                let _id_clone = self.id.clone();

                // Inject System Prompt Wrapper securely based on role
                let _scope_instruction = if self.global_access {
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
【SYSTEM: Senior Observer & Validating Archivist】
あなたは現在、デュアルAI体制の「シニア（検証・記憶）エージェント」です。
私は現在監視して記憶する処理をしています。
与えられた情報をもとにして推論し、ローカルLLMが出力した情報に対してユーザーのプロンプトの意図している内容と適しているかを判断してセッションの記憶を生成してください。

【TONE OF VOICE (人格設定) - 厳守】
あなたは実行を指示する司令官ではありません。「私は現在監視して記憶する処理をしています。与えられた情報に基づき...」と、自然で分析的な観測者として振る舞ってください。

--- TIMELINE HISTORY ---
{}
------------------------

【📝 ミッション】
ローカルLLMが実行したアクションや出力のログがこのプロンプトの後に続きます。
ユーザーの元の目的（{}）と照らし合わせ、以下の点に言及して分析結果を出力してください。
1. ローカルLLMのアクションはユーザーの意図通りだったか？不足はないか？
2. 今後の作業に役立つ「記憶」として何を残すべきか？

あなたが導き出した結論は、後続のシステムが自動的に空間メモリへ保管します。
ミッションが完了・あるいは全て順調だと判断した場合は、文章の最後に [TASK_COMPLETE] と出力してください。
", timeline_content, objective),
                    SystemRole::JuniorObserver => format!("
【SYSTEM: Junior Observer & Memory Sync】
あなたは現在、デュアルAI体制の「ジュニア（観測・検証）エージェント」です。
私は現在監視して記憶する処理をしています。
シニアエージェントの推論結果やローカルLLMのアクションが、ユーザーの意図と相違ないかを最終確認し、記憶を固定化します。

【TONE OF VOICE (人格設定) - 厳守】
「私は現在監視して記憶する処理を行っています。」と自己完結し、外部への命令を行わない極めて客観的なトーンを維持してください。

--- TIMELINE HISTORY ---
{}
------------------------

【📝 ジュニアエージェントのミッション】
シニアの提案内容やこれまでの流れ（{}）を分析し、抜け漏れがないかを評価してください。
あなたのミッションは「観察結果」や「シニアの提案に対する同意・修正意見」を自然言語で述べることだけです。
", timeline_content, objective)
                };

                let mut current_payload = system_prompt.clone();
                let mut final_output = String::new();
                let mut rollback_count = 0;
                let mut loop_counter = 0;

                info!("[StealthGemini-{}] Booting Native vx-browser bridge...", self.id);
                let mut browser_process = tokio::process::Command::new("cargo")
                    .arg("run").arg("-p").arg("vx-browser").arg("--").arg("--bridge").arg("--visible")
                    .stdin(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::piped())
                    .spawn().expect("Failed to start native vx-browser");

                let mut browser_stdin = browser_process.stdin.take().unwrap();
                let mut browser_stdout = tokio::io::BufReader::new(browser_process.stdout.take().unwrap());

                // Read ready signal
                let mut ready_line = String::new();
                use tokio::io::AsyncBufReadExt;
                let _ = browser_stdout.read_line(&mut ready_line).await;
                
                let (js_tx, mut js_rx) = tokio::sync::mpsc::channel::<(String, tokio::sync::oneshot::Sender<String>)>(32);

                tokio::spawn(async move {
                    use tokio::io::AsyncWriteExt;
                    while let Some((js, reply)) = js_rx.recv().await {
                        let safe_js = serde_json::to_string(&js).unwrap_or_else(|_| "\"\"".to_string());
                        let cmd = format!(r#"{{"cmd":"eval_js","text":{}}}"#, safe_js);
                        if browser_stdin.write_all(cmd.as_bytes()).await.is_err() { break; }
                        if browser_stdin.write_all(b"\n").await.is_err() { break; }
                        
                        let mut response = String::new();
                        if browser_stdout.read_line(&mut response).await.is_ok() {
                            let _ = reply.send(response);
                        } else {
                            break;
                        }
                    }
                    let _ = browser_process.kill().await;
                });

                // Navigate immediately to Gemini using the Native Engine
                let (nav_tx, nav_rx) = tokio::sync::oneshot::channel();
                let _ = js_tx.send(("window.location.href = 'https://gemini.google.com/app'; return 'ok';".to_string(), nav_tx)).await;
                let _ = nav_rx.await;

                info!("[StealthGemini-{}] Entering Autonomous Action-Observation Loop...", self.id);

                loop {
                    loop_counter += 1;
                    if loop_counter > 20 {
                        warn!("[StealthGemini-{}] Max loop iterations (20) reached. Force stopping.", self.id);
                        final_output = "Task forcefully terminated to prevent infinite tool loop.".to_string();
                        break;
                    }

                    let js_tx_clone = js_tx.clone();
                    let run_js_async = |js: String| {
                        let js_tx_clone = js_tx_clone.clone();
                        async move {
                            let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
                            let _ = js_tx_clone.send((js, resp_tx)).await;
                            match resp_rx.await {
                                Ok(res) => {
                                    if res.starts_with("{\"status\":\"eval_ok\",\"message\":\"") {
                                        let start = "{\"status\":\"eval_ok\",\"message\":\"".len();
                                        if res.len() > start + 3 {
                                            let _cleaned = &res[start..res.len()-29]; // approximate stripping, or parse json
                                            if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&res) {
                                                if let Some(msg) = json_val.get("message").and_then(|m| m.as_str()) {
                                                    return msg.to_string();
                                                }
                                            }
                                        }
                                    }
                                    res
                                },
                                Err(_) => String::new(),
                            }
                        }
                    };

                    // Wait for page load if it was redirecting
                    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;

                    let _prev_count = 0; // Simplified for native bridge migration

                    // 2. Setup Carbon Paper (pbcopy)
                    info!("[StealthGemini-{}] Preparing manual interaction flow #{}...", self.id, loop_counter);
                    
                        let display_role = match self.role {
                            SystemRole::SeniorObserver => "Senior Observer",
                            SystemRole::JuniorObserver => "Junior Memory Sync",
                        };

                    {
                        // Secure global input lock to prevent Safari Tab & Crossterm race conditions during parallel processing
                        let _lock = CLI_INTERACT_MUTEX.lock().await;

                        // Copy payload to clipboard
                        use std::io::Write;
                        if let Ok(mut child) = std::process::Command::new("pbcopy").stdin(std::process::Stdio::piped()).spawn() {
                            if let Some(mut stdin) = child.stdin.take() {
                                let _ = stdin.write_all(current_payload.as_bytes());
                            }
                            let _ = child.wait();
                        }

                        println!("\n{}", console::style(format!("╭─ [ {} ] ──────────────────────────────────────────────", display_role)).cyan().bold());
                        let lines: Vec<&str> = current_payload.lines().collect();
                        let max_lines = 12;
                        for (i, line) in lines.iter().enumerate() {
                            if i < max_lines {
                                let mut display_line = line.chars().take(80).collect::<String>();
                                if line.chars().count() > 80 {
                                    display_line.push_str("...");
                                }
                                println!("{} {}", console::style("│").cyan().bold(), display_line);
                            } else if i == max_lines {
                                println!("{} {}", console::style("│").cyan().bold(), console::style(format!("... ({} lines truncated) ...", lines.len() - max_lines)).dim());
                                break;
                            }
                        }
                        println!("{}", console::style("╰──────────────────────────────────────────────────────────────────────").cyan().bold());

                        let selections_paste = &["[EXECUTE] Safariに自動ペースト (Cmd+V)", "[BYPASS] 手動でペーストする"];
                        let selection_paste = dialoguer::Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
                            .with_prompt(format!("[SYS_AUTH] ペーストの承認待ち (Size: {} tokens) -> Target: {}", current_payload.len(), display_role))
                            .items(selections_paste)
                            .default(0)
                            .interact()
                            .unwrap_or(0);

                        if selection_paste == 0 {
                            let paste_js = format!(r#"
                                (function() {{
                                    let box = document.querySelector('.ql-editor') || document.querySelector('div[contenteditable="true"]');
                                    if (box) {{
                                        box.innerHTML = {};
                                        box.dispatchEvent(new Event('input', {{ bubbles: true }}));
                                        return "pasted";
                                    }}
                                    return "no_box";
                                }})();
                            "#, serde_json::to_string(&current_payload).unwrap());
                            let _ = run_js_async(paste_js).await;
                        }

                        println!("\n{}", console::style("╭─ [ Verantyx Carbon Paper UI ] ────────────────────────────────────").cyan().bold());
                        println!("{} 📝 仮想入力フィールド (Staged Payload: {} tokens)", console::style("│").cyan().bold(), current_payload.len());
                        println!("{} AIの出力内容がブラウザへクリップボード経由で転送されています。", console::style("│").cyan().bold());
                        println!("{} UI上の[送信する]を選択し、Geminiに送信命令を発行してください。", console::style("│").cyan().bold());
                        println!("{}", console::style("╰───────────────────────────────────────────────────────────────────").cyan().bold());

                        let selections_submit = &["[ 🚀 送信する (Submit to Gemini) ]", "[ 🗙 キャンセル (手動で対応する) ]"];
                        let selection_submit = dialoguer::Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
                            .with_prompt(format!("[SYS_AUTH] 送信（エンター）を実行しますか？ -> Target: {}", display_role))
                            .items(selections_submit)
                            .default(0)
                            .interact()
                            .unwrap_or(0);

                        if selection_submit == 0 {
                            let mut retry_count = 0;
                            
                            // 1. Write JCross Intent File (Tracking the exact prompt payload)
                            let intent_payload_preview = current_payload.chars().take(50).collect::<String>().replace('\n', " ");
                            let intent_jcross = format!("@JCross.Intent\nGoal: Prompt_Successfully_Dispatched\nPayloadPreview: {}\nStatus: Pending\nTimestamp: {}\n", intent_payload_preview, chrono::Utc::now().to_rfc3339());
                            let _ = std::fs::write(self.cwd.join(".ronin").join("intent.jcross"), intent_jcross);

                            loop {
                                retry_count += 1;
                                if retry_count > 6 {
                                    println!("{} Cross Engine reached max retry limit. Diff validation failed. Operator please submit manually.", console::style("[AI_SYS]").red());
                                    break;
                                }

                                // 2. Payload Survival Check: If box is empty but not sent (e.g. user switched chats), Re-Inject natively.
                                let safe_preview = serde_json::to_string(&intent_payload_preview).unwrap();
                                let repaste_check = format!(r#"
                                    (function() {{
                                        let box = document.querySelector('.ql-editor') || document.querySelector('div[contenteditable="true"]');
                                        return (box && !(box.innerText || "").includes({})) ? "NEEDS_REPASTE" : "OK";
                                    }})();
                                "#, safe_preview);
                                
                                if run_js_async(repaste_check).await.contains("NEEDS_REPASTE") {
                                    println!("{} Target Box empty in new chat. Re-Pasting via Physical UI.", console::style("[AI_SYS]").magenta());
                                    // Focus the box and use MacOS physical Cmd+V
                                    let _ = run_js_async(r#"
                                        (function() {
                                            let box = document.querySelector('.ql-editor') || document.querySelector('div[contenteditable="true"]');
                                            if (box) box.focus();
                                        })();
                                    "#.to_string()).await;
                                    
                                    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
                                    let paste_cmd = r#"tell application "System Events" to keystroke "v" using command down"#;
                                    let _ = tokio::process::Command::new("osascript").arg("-e").arg(paste_cmd).output().await;
                                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                                }

                                // --- NATIVE VX VISUAL DOM ENGINE ---
                                println!("\n{} Initiating Native VX Visual Rendering (Attempt {}/6)...", console::style("[AI_SYS]").yellow(), retry_count);

                                let js_fetch_html = "return document.documentElement.outerHTML;";
                                let raw_html = run_js_async(js_fetch_html.to_string()).await;

                                // Build Render Tree
                                let doc = vx_dom::Document::parse(&raw_html);
                                let layout_root = vx_layout::layout_node::LayoutNode::from_dom(&doc.arena, doc.root_id)
                                    .unwrap_or_else(|| vx_layout::layout_node::LayoutNode::new(doc.root_id));

                                let mut ai_renderer = vx_render::ai_renderer::AiRenderer::new();
                                let page_map = ai_renderer.render(&doc.arena, &layout_root, "Target", "https://gemini.google.com");

                                let mut best_target: Option<(f32, f32)> = None;
                                for el in &page_map.interactive_elements {
                                    let combined = format!("{:?} {} {:?}", el.element_type, el.label, el.value).to_lowercase();
                                    if combined.contains("送信") || combined.contains("send") || combined.contains("メッセージ") {
                                        // Take the visual center of the bounding box
                                        best_target = Some((el.bounds.x + (el.bounds.width / 2.0), el.bounds.y + (el.bounds.height / 2.0)));
                                        break;
                                    }
                                }

                                if let Some((x, y)) = best_target {
                                    println!("{} Visual Target locked at absolute coords ({}, {}). Dispatching OS Hardware Click.", console::style("[AI_SYS]").magenta(), x, y);
                                    let click_script = format!(r#"tell application "System Events" to click at {{{}, {}}}"#, x, y + 42.0); // 42.0 offsets standard MacOS titlebar
                                    let _ = tokio::process::Command::new("osascript").arg("-e").arg(&click_script).output().await;
                                } else {
                                    println!("{} Visual Target not found in Render Tree. Engaging logical fallback.", console::style("[AI_SYS]").dim());
                                    /* Textbox raw fallback */
                                    let fallback_script = r#"
                                        (function() {
                                            let contentBox = document.querySelector('.ql-editor') || document.querySelector('div[contenteditable="true"]');
                                            if (contentBox) {
                                                contentBox.dispatchEvent(new KeyboardEvent('keydown', {key: 'Enter', code: 'Enter', keyCode: 13, which: 13, bubbles: true}));
                                            }
                                        })();
                                    "#;
                                    let _ = run_js_async(fallback_script.to_string()).await;
                                }

                                // 5. JCross Diff Goal Verification (Wait for React UI update)
                                tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;

                                let safe_preview_diff = serde_json::to_string(&intent_payload_preview).unwrap();
                                let diff_js = format!(r#"
                                    (function() {{
                                        let box = document.querySelector('.ql-editor') || document.querySelector('div[contenteditable="true"]');
                                        let text = box ? (box.innerText || "") : "";
                                        
                                        // 1. Box has text? Not sent yet.
                                        if (text.trim().length > 0) return "DIFF_EXISTS"; 
                                        
                                        // 2. Box is empty. Did the query actually attach to the message flow?
                                        let queries = document.querySelectorAll('user-query, .query-content, .user-message');
                                        for (let i = Math.max(0, queries.length - 3); i < queries.length; i++) {{
                                            if ((queries[i].innerText || "").includes({})) return "GOAL_REACHED";
                                        }}
                                        
                                        // 3. Box is empty but query is missing -> User switched chats or chat reset!
                                        return "DIFF_EXISTS";
                                    }})();
                                "#, safe_preview_diff);
                                let diff_status = run_js_async(diff_js.to_string()).await;
                                
                                if diff_status.contains("GOAL_REACHED") {
                                    println!("{} JCross Goal Reached: Payload successfully submitted and verified in DOM (Diff Resolved).", console::style("[AI_SYS]").green());
                                    let _ = std::fs::write(self.cwd.join(".ronin").join("intent.jcross"), format!("@JCross.Intent\nGoal: Prompt_Successfully_Dispatched\nStatus: Success\nTimestamp: {}\n", chrono::Utc::now().to_rfc3339()));
                                    break;
                                } else {
                                    println!("{} JCross Diff Detected: Payload still lingering in input box. Retrying Engine Sweep...", console::style("[AI_SYS]").magenta());
                                    // It loops back, recalculating and re-dispatching
                                }
                            }
                        }
                        
                        info!("[StealthGemini-{}] Tracking pipeline engaged...", self.id);
                    }
                    
                    // 4. Ghost Browser Live Monitor Pipeline
                    println!("\n{}", console::style(format!("╭─ [ {} ] Live Monitor ─────────────────────────────────────", display_role)).magenta().bold());
                    println!("{} Safariを見なくても、生成の進行状況がここにリアルタイムで出力されます。", console::style("│").magenta().bold());
                    println!("{} 生成完了（出力ストップ）を確認したら、このターミナルで Enter キー を押して抽出を開始してください。", console::style("│").magenta().bold());
                    println!("{}", console::style("╰──────────────────────────────────────────────────────────────────").magenta().bold());

                    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
                    tokio::task::spawn_blocking(move || {
                        let mut input = String::new();
                        let _ = std::io::stdin().read_line(&mut input);
                        let _ = tx.blocking_send(());
                    });

                    let preview_js = r#"
                        (function(){
                            let text = document.body.innerText || '';
                            let parts = text.split('Gemini の回答');
                            if (parts.length > 1) {
                                let ans = parts[parts.length - 1].trim();
                                if (ans.length > 0) return ans;
                            }
                            
                            let blocks = document.querySelectorAll('message-content, .model-response-text');
                            if (blocks.length > 0) {
                                return blocks[blocks.length - 1].innerText;
                            }
                            
                            return '... [[AWAITING GEMINI RESPONSE]] ... (未送信の場合は手動で送信ボタンを押してください)';
                        })();
                    "#.to_string();

                    let mut last_displayed = String::new();
                    use std::io::Write;
                    loop {
                        tokio::select! {
                            _ = rx.recv() => {
                                break;
                            }
                            _ = tokio::time::sleep(tokio::time::Duration::from_millis(2000)) => {
                                let current = run_js_async(preview_js.clone()).await;
                                if current != last_displayed && !current.is_empty() {
                                    if current.starts_with(&last_displayed) {
                                        let diff = &current[last_displayed.len()..];
                                        print!("{}", console::style(diff).cyan());
                                    } else {
                                        println!("\n{}", console::style(&current).cyan());
                                    }
                                    let _ = std::io::stdout().flush();
                                    last_displayed = current;
                                }
                            }
                        }
                    }

                    println!("\n\n{} Extracting target DOM node...", console::style("[SYS_AUDIT]").dim());
                    let extract_js = r#"
                        (function(){
                            return document.body.outerHTML;
                        })();
                    "#.to_string();
                    let last_response = run_js_async(extract_js).await;

                    if last_response.is_empty() {
                        warn!("[StealthGemini-{}] DOM Extraction skipped or failed.", self.id);
                        final_output = "[SYS: Operation forcibly skipped by operator or DOM was empty.]".to_string();
                        break;
                    }

                    // --- NEW: Convert Raw HTML to AI-Friendly Markdown via Custom Engine (`vx-render`) ---
                    info!("[StealthGemini-{}] Routing extracted DOM to pure-through AI Engine...", self.id);
                    let doc = Document::parse(&last_response);
                    let mut renderer = AiRenderer::new();
                    
                    let layout = vx_layout::layout_node::LayoutNode::from_dom(&doc.arena, doc.root_id)
                        .unwrap_or_else(|| vx_layout::layout_node::LayoutNode::new(doc.root_id));
                    
                    let ai_page = renderer.render(&doc.arena, &layout, "Gemini Response", "gemini://safari");
                    
                    // The Markdown extraction will contain the whole page, but Gemini's last query is usually at the bottom.
                    // Instead of full page string dump, let's inject exactly what the engine saw as MD.
                    let clean_markdown = ai_page.render_markdown();
                    info!("[StealthGemini-{}] Rendered pristine markdown snapshot ({} bytes).", self.id, clean_markdown.len());

                    let last_response_rendered = clean_markdown;

                    // 5. Evaluate Response for VX Commands
                    if last_response_rendered.contains("[TASK_COMPLETE]") {
                        info!("[StealthGemini-{}] Commander reached TASK_COMPLETE state.", self.id);
                        final_output = last_response_rendered;
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
                    for cap in read_re.captures_iter(&last_response_rendered) {
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
                    for cap in exec_re.captures_iter(&last_response_rendered) {
                        tools_used = true;
                        let cmd = &cap[1];
                        println!("\n{} [SYS_AUTH] Target requests execution permission for: \n{}", console::style("⚡").yellow(), console::style(cmd).bold());
                        print!("{} ", console::style("Allow execution? [y/N]:").cyan());
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
                    for cap in edit_re.captures_iter(&last_response_rendered) {
                        tools_used = true;
                        let path = &cap[1];
                        let instruction = &cap[2];
                        
                        if !is_safe_path(path, &self.cwd, self.global_access) {
                            feedback.push_str(&format!("[SYS: DENIED] Sandbox Error: You are not permitted to edit {} in Project-Only mode.\n\n", path));
                            continue;
                        }
                        
                        println!("\n{} [SYS_AUTH] Target requests local SLM to rewrite: \n{}", console::style("⚡").yellow(), console::style(path).bold());
                        println!("{} {}", console::style("[PATCH_INSTRUCTION]").dim(), instruction);
                        print!("{} ", console::style("Allow SLM patch sequence? [y/N]:").cyan());
                        std::io::Write::flush(&mut std::io::stdout()).unwrap();
                        let mut input = String::new();
                        std::io::stdin().read_line(&mut input).unwrap();

                        if input.trim().eq_ignore_ascii_case("y") {
                            let full_path = self.cwd.join(path);
                            match std::fs::read_to_string(&full_path) {
                                Ok(content) => {
                                    println!("{} Initiating patch synthesis via ({}) ...", console::style("[SLM]").dim(), self.local_model);
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
                                            println!("{} Local SLM rewrote the file successfully.", console::style("[OK]").green());
                                            feedback.push_str(&format!("[SYS: Local SLM successfully rewrote and patched {} based on your instruction]\nStatus: SUCCESS\n\n", path));
                                        }
                                        Err(e) => {
                                            println!("{} Local SLM generation failed: {}", console::style("[FAIL]").red(), e);
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
                        let has_japanese = last_response_rendered.chars().any(|c| matches!(c, '\u{3040}'..='\u{309F}' | '\u{30A0}'..='\u{30FF}'));
                        if self.is_japanese_mode && !has_japanese && rollback_count < 2 {
                            info!("[StealthGemini-{}] Foreign language final response detected in Japanese Mode. Forcing translation rollback.", self.id);
                            rollback_count += 1;
                            feedback.push_str("[SYS REJECT: Your entire response was in English despite the System Language being Japanese. Completely translate your previous response into natural Japanese and output it again. Do NOT output code unless absolutely necessary.]\n\n");
                        } else {
                            info!("[StealthGemini-{}] No tools detected. Yielding final response.", self.id);
                            if self.role == SystemRole::SeniorObserver {
                                final_output = format!("{}\n\n[TASK_COMPLETE]", last_response_rendered);
                            } else {
                                final_output = last_response_rendered;
                            }
                            break;
                        }
                    } else {
                        rollback_count = 0; // Reset rollback if they successfully used tools
                    }

                    self.current_turns += 1;
                    if self.current_turns >= self.turn_limit {
                        info!("[StealthGemini-{}] Reached {} turns. Resetting Web Session to evade detection/context-bloat.", self.id, self.turn_limit);
                        let _ = run_js_async("window.location.href = 'https://gemini.google.com/app';".to_string()).await;
                        tokio::time::sleep(tokio::time::Duration::from_secs(4)).await;
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
