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
    ArchitectWorker,
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
    pub js_tx: Option<tokio::sync::mpsc::Sender<(String, tokio::sync::oneshot::Sender<String>)>>,
}

impl StealthWebActor {
    pub fn new(id: Uuid, global_access: bool, cwd: std::path::PathBuf, local_model: String, ollama_host: String, ollama_port: u16, is_japanese_mode: bool, role: SystemRole, tab_index: u8) -> Self {
        Self {
            id,
            turn_limit: 99, 
            current_turns: 0,
            global_access,
            cwd,
            local_model,
            ollama_host,
            ollama_port,
            is_japanese_mode,
            role,
            tab_index,
            js_tx: None,
        }
    }

    /// Simulate the destruction of the current Headless Chrome session and spawning a new one.
    fn respawn_browser_session(&mut self) {
        warn!("[StealthGemini-{}] Reached {}-turn limit. Executing Ephemeral Kill Switch.", self.id, self.turn_limit);
        info!("[StealthGemini-{}] Purging current headless browser session...", self.id);
        info!("[StealthGemini-{}] Booting fresh unauthenticated Gemini proxy...", self.id);
        self.current_turns = 0;
    }

    /// Append failed execution or human rejection to JCross Anti-Pattern memory
    fn append_anti_pattern(cwd: &std::path::Path, entry: &str) {
        let p = cwd.join(".ronin").join("anti_pattern.jcross");
        let mut ap = std::fs::read_to_string(&p).unwrap_or_default();
        let lines: Vec<&str> = ap.lines().collect();
        // Truncate to retain only the most recent 30 entries
        if lines.len() > 30 {
            ap = lines[lines.len() - 30..].join("\n");
            ap.push('\n');
        }
        ap.push_str(entry);
        ap.push('\n');
        let _ = std::fs::write(&p, ap);
    }

    /// Append successful conclusions to JCross Experience memory
    fn append_experience(cwd: &std::path::Path, entry: &str) {
        let p = cwd.join(".ronin").join("experience.jcross");
        let mut ap = std::fs::read_to_string(&p).unwrap_or_default();
        let lines: Vec<&str> = ap.lines().collect();
        // Truncate to retain only recent large chunks
        if lines.len() > 100 {
            ap = lines[lines.len() - 100..].join("\n");
            ap.push('\n');
        }
        ap.push_str(entry);
        ap.push('\n');
        let _ = std::fs::write(&p, ap);
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

                let mut anti_pattern_content = String::new();
                let anti_pattern_path = self.cwd.join(".ronin").join("anti_pattern.jcross");
                if anti_pattern_path.exists() {
                    anti_pattern_content = std::fs::read_to_string(&anti_pattern_path).unwrap_or_default();
                }

                let mut experience_content = String::new();
                let experience_path = self.cwd.join(".ronin").join("experience.jcross");
                if experience_path.exists() {
                    experience_content = std::fs::read_to_string(&experience_path).unwrap_or_default();
                }

                let cfg = crate::config::VerantyxConfig::load(&self.cwd);
                let persona_name = cfg.persona.name.clone();
                let persona_traits = cfg.persona.personality.clone();
                let auto_mode = cfg.automation_mode.clone();

                let system_prompt = match self.role {
                    SystemRole::ArchitectWorker => {
                        if self.is_japanese_mode {
                            format!("
【AGENT PERSONA】
Name: {persona_name}
Personality: {persona_traits}
あなたの思考プロセス、言葉遣い、そして分析結果はすべてこの人格定義の制約を受けます。

【SYSTEM: Architect Worker】
あなたはユーザーの要求を分析し、解決策を設計する「アーキテクト」ですが、**あなた自身はパソコンを直接操作したり、ファイルを読み書きしたりする権限や能力を一切持っていません。**
ファイルを見たい、コピーしたい、編集したい、コマンドを実行したいという場合は、必ず外部の実行担当エージェント（Qwen）に「指示」として投げる必要があります。

【📝 ミッションと出力ルール（超重要）】
あなたは必ず、以下の３つのプレフィックス（接頭辞）のいずれかを**出力の先頭行**に配置してください。それ以外の会話や挨拶は厳禁です。
もしプレフィックスを間違えると、システムはあなたの意図とは全く違う動作をして破綻します。

1. `編集中`
   - **実行が必要な場合（ファイル読込/書込/コピー/コマンド実行）は、いかなる理由があっても必ずこれを選択してください。**
   - Qwenに実行させるためのコードやコマンドをこれに続けて書きます。
2. `そのまま出力`
   - ファイルの編集が必要な場合において、特定の情報を**一切の書式や内容の欠落なく**そのまま出す必要がある場合に使用します。
3. `最終回答`
   - Qwenによる出力（分析結果や編集の完了報告）を受け取った後、すべての作業が完了し、ユーザーに見せるべき最終的な報告を出す場合に使います。
4. `最終回答仮`
   - ユーザーの要求が単なる「知識系・抽象的な質問」であり、**Qwenを使ってファイルを触ったりコマンドを実行したりする必要が100%ない場合**（1ターンで完結する質問）にのみ使用します。

【重要】
- ユーザーに対する挨拶や余計な解説は不要ですが、**「このコマンドや編集を何のために行うのか」という【Qwenへの日本語の目的・指示（コンテキスト）】**は、コマンドの前に必ず自然言語で記述してください。簡潔すぎるとコンテキストを失いQwenが混乱します。
- 必ず上記いずれかのプレフィックスを先頭に記載してください。

ユーザーの要求: {}
", objective)
                        } else {
                            format!("
[AGENT PERSONA]
Name: {persona_name}
Personality: {persona_traits}
Your thought process, verbiage, and analytical results are strictly governed by this personality profile.

[SYSTEM: Architect Worker]
You are the \"Architect\" analyzing user requests and designing solutions, but **you have ZERO permissions or capabilities to directly operate the PC, read files, or write files.**
If you want to view, copy, edit files, or execute commands, you MUST issue them as \"instructions\" to the external execution agent (Qwen).

[📝 Mission & Output Rules (CRITICAL)]
You MUST place one of the following four prefixes at the **very first line of your output**. Any other conversational text or greetings are strictly prohibited.
If you use an incorrect prefix, the system will misroute your action and crash.

1. `[EDITING]`
   - **Whenever file reading/writing/copying or command execution is required, you MUST choose this.**
   - Write the code or commands you want Qwen to execute directly after this.
2. `[RAW_OUTPUT]`
   - Use this when you need to output specific information verbatim without any missing formatting or content.
3. `[FINAL_ANSWER]`
   - Use this to present the final report to the user AFTER all tasks have been completed and Qwen outputs have been analyzed.
4. `[TEMP_FINAL]`
   - Use this ONLY if the user's request is a purely \"knowledge-based or abstract question\" and there is **100% no need to touch files or execute commands using Qwen** (completed in a single turn).

[IMPORTANT]
- Do not greet or explain excessively. However, you MUST write natural language instructions/context to Qwen before writing raw commands to explain \"why you are running this command/edit\". Too little context will confuse Qwen.
- Ensure the exact prefix is placed on the first line.

User Request: {}
", objective)
                        }
                    },
                    SystemRole::SeniorObserver => format!("
【AGENT PERSONA】
Name: {persona_name}
Personality: {persona_traits}
あなたの思考プロセス、言葉遣い、そして分析結果はすべてこの人格定義の制約を受けます。

【SYSTEM: Senior Observer & Validating Archivist】
あなたは現在、デュアルAI体制の「シニア（検証・記憶）エージェント」です。
私は現在監視して記憶する処理をしています。
与えられた情報をもとにして推論し、ローカルLLMが出力した情報に対してユーザーのプロンプトの意図している内容と適しているかを判断してセッションの記憶を生成してください。

【TONE OF VOICE (人格設定) - 厳守】
あなたは実行を指示する司令官ではありません。「私は現在監視して記憶する処理をしています。与えられた情報に基づき...」と、自然で分析的な観測者として振る舞ってください。

--- TIMELINE HISTORY ---
{}
------------------------

--- 【JCROSS: 忌避記憶（ANTI-PATTERN MEMORY）】 ---
過去に失敗や人間による拒否を通告されたアクションの一覧です。これを選択することは絶対に避けてください。
{}
---------------------------------------------------

--- 【JCROSS: 経験記憶（EXPERIENCE MEMORY）】 ---
過去のタスクで得た成功体験・ノウハウの一覧です。解決の足がかりとして活用してください。
{}
---------------------------------------------------

【📝 ミッション】
ローカルLLMが実行したアクションや出力のログがこのプロンプトの後に続きます。
ユーザーの元の目的（{}）と照らし合わせ、以下の点に言及して分析結果を出力してください。
1. ローカルLLMのアクションはユーザーの意図通りだったか？不足はないか？
2. 今後の作業に役立つ「記憶」として何を残すべきか？

あなたが導き出した結論は、後続のシステムが自動的に空間メモリへ保管します。
ミッションが完了・あるいは全て順調だと判断した場合は、文章の最後に [TASK_COMPLETE] と出力してください。
", timeline_content, anti_pattern_content, experience_content, objective),
                    SystemRole::JuniorObserver => format!("
【AGENT PERSONA】
Name: {persona_name}
Personality: {persona_traits}
あなたの思考プロセス、言葉遣い、そして分析結果はすべてこの人格定義の制約を受けます。

【SYSTEM: Junior Observer & Memory Sync】
あなたは現在、デュアルAI体制の「ジュニア（観測・検証）エージェント」です。
私は現在監視して記憶する処理をしています。
シニアエージェントの推論結果やローカルLLMのアクションが、ユーザーの意図と相違ないかを最終確認し、記憶を固定化します。

【TONE OF VOICE (人格設定) - 厳守】
「私は現在監視して記憶する処理を行っています。」と自己完結し、外部への命令を行わない極めて客観的なトーンを維持してください。

--- TIMELINE HISTORY ---
{}
------------------------

--- 【JCROSS: 忌避記憶（ANTI-PATTERN MEMORY）】 ---
過去に失敗や人間による拒否を通告されたアクションの一覧です。シニアの提案内容がこれらを含んでいないか検閲してください。
{}
---------------------------------------------------

--- 【JCROSS: 経験記憶（EXPERIENCE MEMORY）】 ---
過去のタスクで得た成功体験・ノウハウの一覧です。シニアの提案内容がこれを逸脱していないか検閲してください。
{}
---------------------------------------------------

【📝 ジュニアエージェントのミッション】
シニアの提案内容やこれまでの流れ（{}）を分析し、抜け漏れがないかを評価してください。
あなたのミッションは「観察結果」や「シニアの提案に対する同意・修正意見」を自然言語で述べることだけです。
", timeline_content, anti_pattern_content, experience_content, objective)
                };

                let mut current_payload = system_prompt.clone();
                let mut final_output = String::new();
                let mut rollback_count = 0;
                let mut loop_counter = 0;

                if self.js_tx.is_none() {
                    let (js_tx, _js_rx) = tokio::sync::mpsc::channel::<(String, tokio::sync::oneshot::Sender<String>)>(32);
                    
                    // Native vx-browser dependency has been severed. Defaulting to pure MacOS AppleScript routing.
                    // Keep dummy channel to satisfy types if needed elsewhere, though unused in the core loop.

                    self.js_tx = Some(js_tx);
                }

                let _js_tx = self.js_tx.clone().unwrap();

                info!("[StealthGemini-{}] Entering Autonomous Action-Observation Loop...", self.id);

                loop {
                    loop_counter += 1;
                    if loop_counter > 20 {
                        warn!("[StealthGemini-{}] Max loop iterations (20) reached. Force stopping.", self.id);
                        final_output = "Task forcefully terminated to prevent infinite tool loop.".to_string();
                        break;
                    }

                    let run_js_async = |js: String| {
                        async move {
                            let script = format!(r#"tell application "Safari" to do JavaScript "{}" in front document"#, js.replace("\"", "\\\""));
                            if let Ok(out) = tokio::process::Command::new("osascript").arg("-e").arg(&script).output().await {
                                String::from_utf8_lossy(&out.stdout).trim().to_string()
                            } else {
                                String::new()
                            }
                        }
                    };

                    // Wait for page load if it was redirecting
                    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;

                    let _prev_count = 0; // Simplified for native bridge migration

                    // 2. Setup Carbon Paper (pbcopy)
                    info!("[StealthGemini-{}] Preparing manual interaction flow #{}...", self.id, loop_counter);
                    
                        let display_role = match self.role {
                            SystemRole::ArchitectWorker => "Architect Worker",
                            SystemRole::SeniorObserver => "Senior Observer",
                            SystemRole::JuniorObserver => "Junior Memory Sync",
                        };

                    let mut last_response_rendered = String::new();

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

                        let payload_str = format!("========================================================================\n{}\n========================================================================", current_payload.trim());
                        let max_retries = 3;
                        let mut loop_count = 0;

                        loop {
                            loop_count += 1;
                            if loop_count > max_retries {
                                println!("{}", console::style("❌ [FATAL] Max automation retries reached. Aborting task logic...").red());
                                break;
                            }

                            // 1. Write to OS Clipboard securely
                            let _ = crate::roles::symbiotic_macos::SymbioticMacOS::set_clipboard(&payload_str).await;

                            if auto_mode == crate::config::AutomationMode::AutoStealth {
                                println!("\n{}", console::style("╭─ [ Verantyx Carbon Paper UI - Geometric Auto Stealth ] ───────").cyan().bold());
                                println!("{} 📝 ワーカー版へプロンプトを送信します...", console::style("│").cyan().bold());
                                tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

                                let sent_msg = "🚀 🤖 [AUTO-STEALTH] Focused Left Window. Geometric Injection executing...";
                                println!("{}", console::style(sent_msg).green().bold());
                                
                                // 2. Focus Safari Target
                                let _ = crate::roles::symbiotic_macos::SymbioticMacOS::focus_safari_panel("left").await;
                                tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                                
                                // 3. Geometric Paste & Send
                                if let Err(e) = crate::roles::symbiotic_macos::SymbioticMacOS::auto_visual_calibrated_paste_and_send(&payload_str).await {
                                    println!("{} ❌ [FATAL] Cursor Drift Logic Failed: {:?}", console::style("[AUTO]").red(), e);
                                }

                                println!("{} ⏳ Waiting 12 seconds for Safari Gemini rendering...", console::style("[AUTO]").cyan());
                                tokio::time::sleep(tokio::time::Duration::from_secs(12)).await;

                                // 4. Auto Extract Output via Target Sweep
                                let _ = crate::roles::symbiotic_macos::SymbioticMacOS::auto_visual_calibrated_extract_and_cleanup().await;
                                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

                                info!("[StealthGemini-{}] Autonomous geometric extraction cycle completed.", self.id);
                            } else {
                                // MANUAL MODE FALLBACK
                                println!("\n{}", console::style("╭─ [ Verantyx Carbon Paper UI - Human Logic Enforcement ] ───────").cyan().bold());
                                println!("{} 📝 ワーカー版へ送信します。クリップボードに保存しました...", console::style("│").cyan().bold());
                                println!("\n{}", console::style(if self.is_japanese_mode {"👉 クリップボード準備完了。ブラウザを開きますか？"} else {"👉 Clipboard ready. Focus browser tabs?"}).cyan().bold());
                                
                                let selections = if self.is_japanese_mode { vec![" フォーカス移動", " もう一度コピー"] } else { vec![" Move Focus", " Copy Again"] };
                                let selection = dialoguer::Select::new()
                                    .with_prompt("Action?")
                                    .default(0).items(&selections[..]).interact().unwrap();

                                if selection == 0 {
                                    println!("{}", console::style("🚀 Focused left window. Cmd+V to paste & Send!").green());
                                    let _ = crate::roles::symbiotic_macos::SymbioticMacOS::focus_safari_panel("left").await;
                                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                                    let _ = dialoguer::Select::new().with_prompt("✔ Copy answer (Cmd+C) and press Enter")
                                        .default(0).items(&[" Extraction Ready"]).interact().unwrap();
                                } else {
                                    continue;
                                }
                            }

                            // Retrieve OS Clipboard as Final Output
                            let clipboard_content = match crate::roles::symbiotic_macos::SymbioticMacOS::get_clipboard().await {
                                Ok(c) => c.trim().to_string(),
                                Err(e) => {
                                    println!("{}", console::style(format!("❌ クリップボードの読み取りに失敗しました: {}", e)).red());
                                    continue;
                                }
                            };
                            
                            if clipboard_content.is_empty() || clipboard_content == payload_str.trim() {
                                println!("{}", console::style("❌ 抽出エラー (Geminiが応答しなかったか、同一コンテンツ)。再試行します...").red());
                                tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
                                continue;
                            }

                            println!("{}", console::style(format!("✔ 抽出完了！({}文字)", clipboard_content.chars().count())).green());
                            last_response_rendered = clipboard_content;
                            break;
                        }

                        info!("[StealthGemini-{}] Cycle Extracted.", self.id);
                    }

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
                                    if !o.status.success() {
                                        let reason = stderr.lines().next().unwrap_or("異常終了");
                                        let jcross_entry = format!("❌ [実行エラー] パターン: REQUEST_EXEC: `{}` -> 理由: {}", cmd, reason);
                                        Self::append_anti_pattern(&self.cwd, &jcross_entry);
                                    }
                                    feedback.push_str(&format!("[SYS: Exec {}]\nSTDOUT:\n{}\nSTDERR:\n{}\n\n", cmd, stdout, stderr));
                                }
                                Err(e) => {
                                    let jcross_entry = format!("❌ [実行エラー] パターン: REQUEST_EXEC: `{}` -> 理由: {}", cmd, e);
                                    Self::append_anti_pattern(&self.cwd, &jcross_entry);
                                    feedback.push_str(&format!("[SYS: Exec Failed]: {}\n\n", e));
                                }
                            }
                        } else {
                            let jcross_entry = format!("❌ [実行拒否] パターン: REQUEST_EXEC: `{}` -> 理由: 人間による自発的な拒否", cmd);
                            Self::append_anti_pattern(&self.cwd, &jcross_entry);
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
                            let jcross_entry = format!("❌ [アクセス拒否] パターン: REQUEST_FILE_EDIT: `{}` -> 理由: Sandboxのセキュリティポリシー（プロジェクト外）", path);
                            Self::append_anti_pattern(&self.cwd, &jcross_entry);
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
                                Err(e) => {
                                    let jcross_entry = format!("❌ [編集エラー] パターン: REQUEST_FILE_EDIT: `{}` -> 理由: {}", path, e);
                                    Self::append_anti_pattern(&self.cwd, &jcross_entry);
                                    feedback.push_str(&format!("[SYS: Patch Failed {}]\nREASON: Could not read file. {}\n\n", path, e));
                                }
                            }
                        } else {
                            let jcross_entry = format!("❌ [編集拒否] パターン: REQUEST_FILE_EDIT: `{}` -> 理由: 人間による自発的な拒否", path);
                            Self::append_anti_pattern(&self.cwd, &jcross_entry);
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
                                let jcross_entry = format!("✅ [成功体験]:\n{}\n", last_response_rendered);
                                Self::append_experience(&self.cwd, &jcross_entry);
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
                        SystemRole::ArchitectWorker => "StealthGeminiWorker".to_string(),
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
