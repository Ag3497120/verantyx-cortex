use ronin_hive::actor::{Actor, Envelope};
use ronin_hive::messages::HiveMessage;
use ronin_hive::roles::stealth_gemini::StealthWebActor;
use ronin_core::models::provider::{LlmProvider, LlmMessage};
use ronin_core::models::provider::ollama::OllamaProvider;
use ronin_core::models::sampling_params::{InferenceRequest, SamplingParams, PromptFormat};
use ronin_core::memory_bridge::spatial_index::SpatialIndex;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;
use uuid::Uuid;
use chrono::Utc;
use std::io::{self, Write};
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

enum ActiveAgent {
    Stealth(StealthWebActor),
    Hybrid(ronin_hive::roles::hybrid_api::HybridApiActor),
}

impl ActiveAgent {
    async fn receive(&mut self, env: Envelope) -> anyhow::Result<Option<Envelope>> {
        match self {
            Self::Stealth(a) => a.receive(env).await,
            Self::Hybrid(a) => a.receive(env).await,
        }
    }
}

fn focus_terminal() {
    let term = std::env::var("TERM_PROGRAM").unwrap_or_default();
    let app_name = if term.contains("iTerm") {
        "iTerm"
    } else if term.contains("Apple_Terminal") {
        "Terminal"
    } else if term.contains("vscode") {
        if std::path::Path::new("/Applications/Cursor.app").exists() {
            "Cursor"
        } else {
            "Visual Studio Code"
        }
    } else if term.contains("ghostty") {
        "Ghostty"
    } else if term.contains("WezTerm") {
        "WezTerm"
    } else if term.contains("Alacritty") {
        "Alacritty"
    } else {
        "Terminal" // fallback
    };

    let script = format!("tell application \"{}\" to activate", app_name);
    let _ = std::process::Command::new("osascript").arg("-e").arg(&script).spawn();
    let _ = std::process::Command::new("afplay").arg("/System/Library/Sounds/Glass.aiff").spawn();
}

fn create_spinner(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner()
        .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", "✔"])
        .template("\x1b[38;2;240;148;100m{spinner}\x1b[0m {msg}").unwrap()
    );
    pb.enable_steady_tick(Duration::from_millis(80));
    pb.set_message(msg.to_string());
    pb
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Initialize minimalistic UI logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();
    
    // 1.5 Load Config (Wizard if first time)
    let config = ronin_hive::config::VerantyxConfig::load_or_wizard(&std::env::current_dir().unwrap());
    
    // Save to persist their latest explicit choice
    let _ = config.save(&std::env::current_dir().unwrap());

    let is_api_mode = config.automation_mode == ronin_hive::config::AutomationMode::HybridApi;
    let is_ja = config.language == "ja";

    // 2. Dual-Window Visualization Orchestration (AppleScript) - Skip in API modes
    if !is_api_mode {
        let init_spinner = create_spinner(if is_ja { "ステルスブラウザを起動・調整中..." } else { "Spawning Custom Stealth Browser..." });
        let split_screen_js = r#"
        do shell script "open -a Safari"
        delay 1.5
        
        tell application "Finder"
            set bnd to bounds of window of desktop
            set screenWidth to item 3 of bnd
            set screenHeight to item 4 of bnd
        end tell
        
        set winHeight to (screenHeight * 0.85) as integer
        set topMargin to 50
        set winWidth to (screenWidth * 0.65) as integer
        
        tell application "Safari"
            activate
            delay 0.5
            
            make new document with properties {URL:"https://gemini.google.com/app"}
            set _w1 to front window
            set bounds of _w1 to {10, topMargin, 10 + winWidth, topMargin + winHeight}
            
            make new document with properties {URL:"https://gemini.google.com/app"}
            set _w2 to front window
            set bounds of _w2 to {100, topMargin, 100 + winWidth, topMargin + winHeight}
            
            make new document with properties {URL:"https://gemini.google.com/app"}
            set _w3 to front window
            set bounds of _w3 to {200, topMargin, 200 + winWidth, topMargin + winHeight}
        end tell
        "#;
        let _ = tokio::process::Command::new("osascript")
            .arg("-e")
            .arg(split_screen_js)
            .output()
            .await;
        init_spinner.finish_with_message(format!("{}", console::style(if is_ja { "✔ ブラウザをリンク完了" } else { "✔ Browser Coordinated" }).green()));
    }

    // 2.5 Print the OpenClaude-style Banner
    ronin_hive::openclaude_ui::print_startup_screen(&config.cloud_provider, true);

    if config.nightwatch.enabled {
        let watch_dir = if config.nightwatch.watch_dir == "." {
            std::env::current_dir().unwrap()
        } else {
            std::path::PathBuf::from(&config.nightwatch.watch_dir)
        };
        ronin_hive::nightwatch::observer::FileObserver::new(watch_dir.clone()).start_detached();

        // Phase 1: Nightwatch Background Distillation Daemon
        let daemon_cfg = config.clone();
        let daemon_dir = watch_dir.clone();
        tokio::spawn(async move {
            // Check the queue every 60 seconds
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            let daemon = ronin_hive::nightwatch::daemon::NightwatchDaemon::new(daemon_cfg, daemon_dir);
            loop {
                interval.tick().await;
                daemon.run_knowledge_distillation().await;
            }
        });
    }

    let env_key_name = match config.cloud_provider {
        ronin_hive::config::CloudProvider::Gemini => "GEMINI_API_KEY",
        ronin_hive::config::CloudProvider::OpenAi => "OPENAI_API_KEY",
        ronin_hive::config::CloudProvider::Anthropic => "ANTHROPIC_API_KEY",
        ronin_hive::config::CloudProvider::DeepSeek => "DEEPSEEK_API_KEY",
        ronin_hive::config::CloudProvider::OpenRouter => "OPENROUTER_API_KEY",
        ronin_hive::config::CloudProvider::Groq => "GROQ_API_KEY",
        ronin_hive::config::CloudProvider::Together => "TOGETHER_API_KEY",
    };
    
    let cloud_api_key = std::env::var(env_key_name).unwrap_or_else(|_| {
        let warning = format!("⚠ Warning: Missing {} in environment. Cloud Brain may fail.", env_key_name);
        println!("{}", console::style(warning).yellow());
        String::new()
    });

    // 3. Spawn StealthGemini Actor
    let subagent_id = Uuid::new_v4();
    let mut ephemeral_worker = if is_api_mode {
        ActiveAgent::Hybrid(ronin_hive::roles::hybrid_api::HybridApiActor::new(
            subagent_id,
            true, // global access
            std::env::current_dir().unwrap(), 
            "gemma-2-test".to_string(), 
            "Dual Browser Reactive REPL".to_string(), 
            999, // Infinite loop essentially
            is_ja, 
            ronin_hive::roles::stealth_gemini::SystemRole::ArchitectWorker, 
            1,
            cloud_api_key.clone(),
        ))
    } else {
        ActiveAgent::Stealth(StealthWebActor::new(
            subagent_id,
            true, // global access
            std::env::current_dir().unwrap(), 
            "gemma-2-test".to_string(), 
            "Dual Browser Reactive REPL".to_string(), 
            999, // Infinite loop essentially
            is_ja, 
            ronin_hive::roles::stealth_gemini::SystemRole::ArchitectWorker, 
            1
        ))
    };

    let senior_id = Uuid::new_v4();
    let apprentice_id = Uuid::new_v4();
    let mut senior_agent = ronin_hive::roles::supervisor_gemini::SupervisorGeminiActor::new(senior_id, ronin_hive::roles::supervisor_gemini::SupervisorRank::Senior, is_ja);
    let mut apprentice_agent = ronin_hive::roles::supervisor_gemini::SupervisorGeminiActor::new(apprentice_id, ronin_hive::roles::supervisor_gemini::SupervisorRank::Apprentice, is_ja);

    // --- Boot Spatial Index Memory Engine ---
    let root_path = std::env::current_dir().unwrap().join(".ronin").join("experience.jcross");
    let mut spatial_index = SpatialIndex::new(root_path);
    if let Ok(count) = spatial_index.hydrate().await {
        let text = if is_ja { format!("🧠 空間記憶エンジン起動... {}件の過去のノードをロードしました。", count) } else { format!("🧠 Spatial Memory Engine Booted... Loaded {} past nodes.", count) };
        println!("{}", console::style(text).color256(147)); // Soft purple/blue
    }
    
    // --- Boot UI Bridge Server ---
    let bridge = std::sync::Arc::new(ronin_hive::nightwatch::server::VeraUiBridge::new());
    let bridge_clone = bridge.clone();
    tokio::spawn(async move {
        ronin_hive::nightwatch::server::VeraUiBridge::start(bridge_clone).await;
    });

    println!();

    let mut conversation_turns = 0;
    let mut previous_worker_payload = String::new();
    let mut auto_forward_payload = String::new();

    // Define Prefixes
    let pfx_editing = if is_ja { "編集中" } else { "[EDITING]" };
    let pfx_raw = if is_ja { "そのまま出力" } else { "[RAW_OUTPUT]" };
    let pfx_final = if is_ja { "最終回答" } else { "[FINAL_ANSWER]" };
    let pfx_temp = if is_ja { "最終回答仮" } else { "[TEMP_FINAL]" };
    let pfx_final_out = if is_ja { "最終出力" } else { "[FINAL_OUTPUT]" };

    // --- Stdin Background Reader ---
    let (stdin_tx, stdin_rx) = std::sync::mpsc::channel::<String>();
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        loop {
            let mut buf = String::new();
            if stdin.read_line(&mut buf).is_ok() {
                let _ = stdin_tx.send(buf.trim().to_string());
            }
        }
    });

    let mut last_auto_thirst_id: Option<String> = None;

    // 4. Interactive Chat Loop
    loop {
        // --- PRE-PHASE: STRUCTURAL TENSION (FREE ENERGY) EVALUATION ---
        let (tension_score, critical_void) = spatial_index.calculate_structural_tension();
        if tension_score > 5.0 {
            println!("{}", console::style(format!("⚠️ [SYSTEM NERVOUS ALERT]: High Structural Entropy Detected (Tension: {:.2})", tension_score)).red().bold());
            println!("{}", console::style(format!("   Target Void: {}", critical_void.as_deref().unwrap_or("UNKNOWN"))).red());
            
            // --- WEANING PHASE: AUTONOMOUS EPISTEMIC DRIVE INTERCEPT ---
            if let Some(void_id) = &critical_void {
                if last_auto_thirst_id.as_deref() != Some(void_id.as_str()) {
                    if let Some(void_node) = spatial_index.nodes.get(void_id) {
                        let content = &void_node.content;
                        let auto_prompt = format!(
                            "【SYSTEM AUTONOMOUS EPISTEMIC DRIVE】\nJCrossノード [{}] 周辺に致命的な知識の欠落(Void)が存在し、システムのエントロピーが最大化しています。\n以下の自発的アクションキューを元に、直ちにブラウザ検索・実装戦略を立案し、欠落しているアーキテクチャの知識を補完してください。\n\n{}", 
                            void_id, content
                        );
                        auto_forward_payload = auto_prompt;
                        last_auto_thirst_id = Some(void_id.clone());
                        println!("{}", console::style("🌀 [AUTONOMOUS BYPASS] System is seizing STDIN to execute self-directed knowledge acquisition...").yellow().bold());
                    }
                }
            }
        }

        let is_new_user_turn = auto_forward_payload.is_empty();
        
        let query = if !is_new_user_turn {
            let val = auto_forward_payload.clone();
            auto_forward_payload.clear();
            println!("{}", console::style("  [System Auto Forwarding Payload to Worker]").dim());
            val
        } else {
            print!("\x1b[38;2;240;148;100m❯\x1b[0m ");
            io::stdout().flush().unwrap();
            
            let mut input_res = String::new();
            loop {
                // Check Crucible UI Queue
                if let Ok(mut q) = bridge.crucible_command_queue.lock() {
                    if let Some(cmd) = q.take() {
                        println!("{}", console::style(&cmd).yellow()); // visually print it so user sees
                        input_res = cmd;
                        break;
                    }
                }
                
                // Check Stdin Input
                if let Ok(line) = stdin_rx.try_recv() {
                    input_res = line;
                    break;
                }
                
                // Yield to prevent CPU 100%
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            input_res
        };

        if query == "exit" || query == "quit" {
            println!("Exiting Interactive Mode.");
            break;
        }

        if query == "vera" || query == "show memory" || query == "memory" || query == "veramemory" {
            let target_dir = std::env::current_dir().unwrap();
            
            // Render on demand to ensure latest state
            ronin_hive::nightwatch::visualizer::VeraMemoryVisualizer::generate_html(&spatial_index, &target_dir);
            
            let html_path = target_dir.join(".ronin").join("vera_memory.html");
            if html_path.exists() {
                if is_ja { println!("{}", console::style("✨ Vera Memoryのダッシュボードを展開します...").cyan()); }
                else { println!("{}", console::style("✨ Opening Vera Memory Dashboard...").cyan()); }
                
                #[cfg(target_os = "macos")]
                let _ = std::process::Command::new("open").arg(&html_path).spawn();
                #[cfg(target_os = "windows")]
                let _ = std::process::Command::new("cmd").args(&["/C", "start", html_path.to_str().unwrap()]).spawn();
                #[cfg(target_os = "linux")]
                let _ = std::process::Command::new("xdg-open").arg(&html_path).spawn();
            } else {
                if is_ja { println!("{}", console::style("⚠ Vera Memoryはまだ生成されていません。Nightwatchデーモンがファイル解析を終えるのをお待ちください。").yellow()); }
                else { println!("{}", console::style("⚠ Vera Memory is not generated yet. Please wait for Nightwatch to complete its first distillation loop.").yellow()); }
            }
            continue;
        }

        if query.starts_with("time-machine") {
            let parts: Vec<&str> = query.split_whitespace().collect();
            if parts.len() >= 2 {
                let path = parts[1].to_string();
                if is_ja { println!("{}", console::style(format!("🚀 タイムマシン・プロトコル発動: {} を全探索し、空間記憶を構築します...", path)).magenta().bold()); }
                else { println!("{}", console::style(format!("🚀 TIME MACHINE INITIATED: Full scanning {} to build spatial memory...", path)).magenta().bold()); }
                
                let mut indexer = ronin_hive::nightwatch::time_machine::TimeMachineIndexer::new(&path);
                // We run it synchronously or block on it because it's a massive batch process initiated manually
                indexer.run_scan(&mut spatial_index).await;
                
                if is_ja { println!("{}", console::style("✅ タイムマシン処理が完了し、JCrossが大規模更新されました。").green().bold()); }
                else { println!("{}", console::style("✅ Time Machine scan complete. JCross spatial index massively updated.").green().bold()); }
                continue;
            } else {
                println!("{}", console::style("Usage: time-machine <path_to_scan>").yellow());
                continue;
            }
        }

        if query.starts_with("crucible ") {
            let parts: Vec<&str> = query.split_whitespace().collect();
            let files: Vec<&str> = parts[1..].to_vec();
            if files.len() >= 2 {
                if files.len() > 10 {
                    println!("{}", console::style("⚠ Crucible supports a maximum of 10 nodes simultaneously.").red());
                    continue;
                }
                
                let mut contents_block = String::new();
                for (i, file) in files.iter().enumerate() {
                    let jcat = spatial_index.nodes.values().find(|n| n.env_hash.as_deref() == Some(*file)).map(|n| n.to_jcross());
                    let content = jcat.unwrap_or_else(|| {
                        if is_ja { format!("【未解析の外部ファイル】\nパス: {}", file) }
                        else { format!("[UNPARSED EXTERNAL FILE]\nPath: {}", file) }
                    });
                    contents_block.push_str(&format!("\nファイル{} ({})\n```jcross\n{}\n```\n", i+1, file, content));
                }
                
                let mut prompt = if is_ja {
                    format!("【🚨 VERA LAB CRUCIBLE SYNTHESIS 🚨】\n\n以下の{}個のファイルの中間表現（JCross仕様のLossless Semantic Compression）を与えます。これらの構造（タグ、抽象度、関係性）を融合させ、それらの機能の『中間』や『架け橋』となる全く新しいロジック（またはアーキテクチャ）のIR（中間表現）と具体的なコード案をシミュレーションして出力せよ。\n{}", files.len(), contents_block)
                } else {
                    format!("[🚨 VERA LAB CRUCIBLE SYNTHESIS 🚨]\n\nI am giving you the JCross Lossless Semantic Compressions for {} files. Synthesize their core concepts (Kanji tags, abstraction, topology) and propose a brand new hybrid architectural logic or bridge code that combines their functions. Provide a high-level IR and code simulation.\n{}", files.len(), contents_block)
                };

                prompt.push_str("\n\nOUTPUT FORMAT REQUIREMENT:\nYou MUST output ONLY a valid JSON object matching this schema. Do not output anything outside the JSON block.\n```json\n{\n  \"synthesized_jcross\": {\n    \"kanji_tags\": [\"[創:0.9]\", \"[結:0.8]\"],\n    \"concept\": \"Description of the fused architecture\",\n    \"abstract_level\": 0.8\n  },\n  \"explanation\": \"Brief explanation of how the architectures were fused.\",\n  \"vision_prompt\": \"A highly detailed, cinematic concept art of a glowing cyberpunk architecture representing [concept], neural networks fusing...\"\n}\n```");

                let env = Envelope {
                    message_id: Uuid::new_v4(),
                    sender: "UserREPL".to_string(),
                    recipient: "SeniorSupervisor".to_string(), // Send to highest reasoning unit
                    payload: serde_json::to_string(&HiveMessage::Objective(prompt))?,
                };
                
                println!("{}", console::style(format!("🌀 CRUCIBLE INITIATED: Fusing Semantics of {} files in memory...", files.len())).bold().yellow());
                
                let sp_synth = create_spinner("Synthesizing JCross Logic Crucible...");
                if let Some(reply) = senior_agent.receive(env).await? {
                    if let Ok(HiveMessage::Objective(synth_res)) = serde_json::from_str(&reply.payload) {
                        sp_synth.finish_and_clear();
                        
                        let mut clean_json = synth_res.trim();
                        if let Some(start) = clean_json.find('{') {
                            if let Some(end) = clean_json.rfind('}') {
                                clean_json = &clean_json[start..=end];
                            }
                        }
                            
                        if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(clean_json) {
                            if let Some(vision_prompt) = json_val.get("vision_prompt").and_then(|v| v.as_str()) {
                                println!("\n{}", console::style("✨ [VISION AI PROMPT GENERATED] ✨").magenta().bold());
                                println!("{}", console::style(vision_prompt).blue());
                                println!("{}", console::style("────────────────────────────────────────────").dim());
                            }
                            
                            println!("{}", console::style("🚀 Launching 3D Crucible Visualizer...").green());
                            ronin_hive::nightwatch::visualizer_3d::generate_3d_html(&json_val);

                            let sp_judge = create_spinner("Executing Cold Judge & Void Extraction...");
                            let judge_payload = format!(
                                "以下の合成された新アーキテクチャ案に対して、徹底的に冷徹な専門家として『それがいかに机上の空論であり、現在の物理的制約や既存システムの構造に照らし合わせて破綻しているか』を厳しく批判し、そのアーキテクチャを実現するために【物理的に決定的に足りていないパーツ（Void: 欠落）】を1つ抽出せよ。\n\n出力は以下のJSONのみとすること：\n```json\n{{\n  \"harsh_criticism\": \"批判内容\",\n  \"missing_piece\": \"欠落している具体的な概念やパーツ\"\n}}\n```\n\n対象案：\n{}", 
                                clean_json
                            );

                            let judge_env = Envelope {
                                message_id: Uuid::new_v4(),
                                sender: "UserREPL".to_string(),
                                recipient: "EPISTEMIC_DRIVE".to_string(),
                                payload: serde_json::to_string(&HiveMessage::Objective(judge_payload)).unwrap(),
                            };
                            
                            let evaluator = ronin_core::models::task_evaluator::TaskEvaluator::new();
                            let judge_res = match evaluator.evaluate_task(&judge_env.payload, ronin_core::models::task_evaluator::EvaluationMode::Strict).await {
                                Ok(res) => res.logs,
                                Err(_) => String::new(),
                            };
                            sp_judge.finish_and_clear();

                            let mut clean_epi = judge_res.trim();
                            if let Some(start) = clean_epi.find('{') {
                                if let Some(end) = clean_epi.rfind('}') {
                                    clean_epi = &clean_epi[start..=end];
                                }
                            }
                            match serde_json::from_str::<serde_json::Value>(clean_epi) {
                                Ok(epi_json) => {
                                    println!("{}", console::style("\n👨‍⚖️ [THE COLD JUDGE: HARSH CRITICISM]").red().bold());
                                    if let Some(criticism) = epi_json.get("harsh_criticism").and_then(|v| v.as_str()) {
                                        println!("{}", console::style(criticism).red());
                                    }
                                    
                                    println!("{}", console::style("\n🕳️ [VOID EXTRACTION]").cyan().bold());
                                    if let Some(missing) = epi_json.get("missing_piece").and_then(|v| v.as_str()) {
                                        println!("Missing Piece: {}", console::style(missing).cyan());
                                        
                                        println!("{}", console::style("\n🧭 [EPISTEMIC DRIVE: NEW THIRST NODE]").yellow().bold());
                                        let void_id = format!("void_{}", Uuid::new_v4().to_string().replace("-", "")[0..12].to_string());
                                        let thirst_tags = vec![
                                            ronin_core::memory_bridge::kanji_ontology::KanjiTag::resolve("探"),
                                            ronin_core::memory_bridge::kanji_ontology::KanjiTag::resolve("基"),
                                            ronin_core::memory_bridge::kanji_ontology::KanjiTag::resolve("縛"),
                                        ];
                                        let mut void_node = ronin_core::memory_bridge::spatial_index::MemoryNode {
                                            id: Uuid::new_v4(),
                                            key: void_id.clone(),
                                            kanji_tags: thirst_tags,
                                            abstract_level: 0.95,
                                            utility: 1.0,
                                            content: format!("{}\n\n【自発的アクションキュー】\n{}", 
                                                json_val.get("concept").and_then(|v| v.as_str()).unwrap_or("Unknown Architecture"),
                                                missing),
                                            relations: std::collections::HashMap::new(),
                                            env_hash: None,
                                            reflex_action: None,
                                            physical_filepath: None,
                                        };
                                        let target_dir = std::env::current_dir().unwrap().join(".ronin").join("jcross_v4");
                                        std::fs::create_dir_all(&target_dir).unwrap();
                                        let file_path = target_dir.join(format!("{}.jcross", void_id));
                                        std::fs::write(&file_path, void_node.to_jcross()).unwrap();
                                        spatial_index.nodes.insert(void_id.clone(), void_node);
                                        
                                        println!("\n{}", console::style(format!("💾 [MIND UPLOAD]: Epistemic Thirst Node '{}' physically injected into Cyberspace.", void_id)).dim());
                                    }
                                }
                                Err(e) => {
                                    println!("{}", console::style(format!("⚠ Failed to parse Epistemic JSON. Error: {}", e)).red());
                                    println!("Raw extracted string:\n{}", console::style(clean_epi).dim());
                                }
                            }

                        } else {
                            println!("{}", console::style("⚠ Failed to parse pure JSON. AI response was:").red());
                            println!("\n{}\n", synth_res);
                        }
                    }
                } else {
                    sp_synth.finish_and_clear();
                }
                
                continue;
            } else {
                println!("{}", console::style("Usage: crucible <file1_path> <file2_path>").yellow());
                continue;
            }
        }

        if query.is_empty() { continue; }

        let mut apprentice_feedback = String::new();

        if is_new_user_turn {
            conversation_turns += 1;
            
            if conversation_turns > 5 {
                if is_ja {
                    println!("{}", console::style("\n🔄 [Turn Limit Reached] 5ターンを経過しました。記憶の抽出とリレー大移動を開始します...").magenta().bold());
                } else {
                    println!("{}", console::style("\n🔄 [Turn Limit Reached] Surpassed 5 turns. Triggering memory extraction and relay...").magenta().bold());
                }
                let relay_prompt = if is_ja {
                    r#"
【強制コマンド：次世代Geminiへの記憶リレー抽出】
現在の時系列の内容とこれまでの会話の流れをすべて出力せよ。
あなたは間もなくシャットダウンされ、次にあなたと全く同じ初期プロンプトを持った新しいGemini（シニア・弟子・ワーカー）が立ち上がります。
そのため、あなたの現在の内部記憶状態の詳細をまとめて、次のGeminiに渡すための引き継ぎテキストを生成してください。
もし文字数が1万文字を超えそうな場合は、直近10件分の情報のみ詳細に記述し、それより前の時系列については要約して、全体が確実に1万文字以内に収まるように圧縮してください。
"#
                } else {
                    r#"
[FORCED COMMAND: Memory Relay Extraction to Next-Gen Gemini]
Output the content of the current timeline and the flow of conversation up to this point.
You are about to be shut down, and a new Gemini (Senior/Apprentice/Worker) with the exact same initial prompt will spin up.
Therefore, summarize the details of your current internal memory state and generate a handover text for the next Gemini.
If it exceeds 10,000 characters, detail only the 10 most recent events and summarize older ones to ensure it stays within 10,000 characters.
"#
                };
                let relay_dispatch = HiveMessage::Objective(relay_prompt.to_string());
                let relay_env = Envelope {
                    message_id: Uuid::new_v4(),
                    sender: "UserREPL".to_string(),
                    recipient: "SeniorSupervisor".to_string(),
                    payload: serde_json::to_string(&relay_dispatch)?,
                };

                let mut extracted_memory = String::new();
                if let Some(relay_reply) = senior_agent.receive(relay_env).await? {
                    if let Ok(HiveMessage::Objective(mem)) = serde_json::from_str(&relay_reply.payload) {
                        extracted_memory = mem;
                    }
                }

                println!("{}", console::style("\n✅ 記憶の抽出が完了しました。古いエージェントを破棄して新しいエージェントに継承します。").green().bold());
                let mut timeline_path = std::env::current_dir().unwrap();
                timeline_path.push(".ronin");
                std::fs::create_dir_all(&timeline_path).unwrap();
                timeline_path.push("timeline.md");
                std::fs::write(&timeline_path, extracted_memory).unwrap();

                #[cfg(target_os = "macos")]
                if !is_api_mode {
                    // Physical 5-Turn Reset: Closing the Safari Tab to clear conversational memory leaks
                    println!("{}", console::style("🧹 Closing Safari tab to flush free Gemini memory...").dim());
                    let script = r#"
                        tell application "Safari"
                            activate
                            tell application "System Events"
                                keystroke "w" using command down
                            end tell
                        end tell
                    "#;
                    let _ = std::process::Command::new("osascript").arg("-e").arg(script).output();
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }

                let new_worker_id = Uuid::new_v4();
                let new_senior_id = Uuid::new_v4();
                let new_apprentice_id = Uuid::new_v4();

                ephemeral_worker = if is_api_mode {
                    ActiveAgent::Hybrid(ronin_hive::roles::hybrid_api::HybridApiActor::new(
                        new_worker_id,
                        true,
                        std::env::current_dir().unwrap(), 
                        "gemma-2-test".to_string(), 
                        "Dual Browser Reactive REPL".to_string(), 
                        999,
                        is_ja, 
                        ronin_hive::roles::stealth_gemini::SystemRole::ArchitectWorker, 
                        1,
                        cloud_api_key.clone(),
                    ))
                } else {
                    ActiveAgent::Stealth(StealthWebActor::new(
                        new_worker_id,
                        true,
                        std::env::current_dir().unwrap(), 
                        "gemma-2-test".to_string(), 
                        "Dual Browser Reactive REPL".to_string(), 
                        999,
                        is_ja, 
                        ronin_hive::roles::stealth_gemini::SystemRole::ArchitectWorker, 
                        1
                    ))
                };
                senior_agent = ronin_hive::roles::supervisor_gemini::SupervisorGeminiActor::new(
                    new_senior_id,
                    ronin_hive::roles::supervisor_gemini::SupervisorRank::Senior,
                    is_ja
                );
                apprentice_agent = ronin_hive::roles::supervisor_gemini::SupervisorGeminiActor::new(
                    new_apprentice_id,
                    ronin_hive::roles::supervisor_gemini::SupervisorRank::Apprentice,
                    is_ja
                );

                conversation_turns = 1;
                println!("{}", console::style("✅ 次世代への時系列引き継ぎが完了しました。\n").green().bold());
            }

            // --- PHASE 0: Supervisor Apprentice Hook (Offset T+1) ---
            if conversation_turns > 1 {
                let app_payload = format!("【前回の実行結果（振り返り用）】\n{}\n\nこれに基づく空間記憶の反映漏れや異常を指摘してください。", previous_worker_payload);
                let app_dispatch = HiveMessage::Objective(app_payload);
                let app_env = Envelope {
                    message_id: Uuid::new_v4(),
                    sender: "UserREPL".to_string(),
                    recipient: "ApprenticeSupervisor".to_string(),
                    payload: serde_json::to_string(&app_dispatch)?,
                };
                
                if let Some(app_reply) = apprentice_agent.receive(app_env).await? {
                    if let Ok(HiveMessage::Objective(app_mod)) = serde_json::from_str(&app_reply.payload) {
                        apprentice_feedback = app_mod;
                    }
                }
            }
        }

        // --- PHASE 1: Gemini Architect (Worker) Dispatch ---
        let mut worker_prompt = query.to_string();
        
        if tension_score > 5.0 && !worker_prompt.contains("AUTONOMOUS EPISTEMIC DRIVE") {
            let void_msg = if is_ja {
                format!("\n[システム緊急アラート (構造的エントロピー張力: {:.2})]\nJCrossノード [{}] 周辺に深刻な知識の欠落(Void)が存在し、空間グラフのエントロピーが最大化しています。\n絶対指令: このVoidを埋めるための外部知識（検索、API探索、実装戦略）の取得を「最優先事項」として行動し、具体的な解決アクションを生成せよ。", tension_score, critical_void.as_deref().unwrap_or_default())
            } else {
                format!("\n[SYSTEM ALERT (Structural Dissonance Level: {:.2})]\nA severe structural void exists around JCross Node [{}]. The graph entropy is maximizing.\nABSOLUTE DIRECTIVE: You MUST prioritize searching for external knowledge (APIs, documentation, implementation strategies) to resolve this void. Generate an action plan to fulfill this missing semantic edge.", tension_score, critical_void.as_deref().unwrap_or_default())
            };
            worker_prompt = format!("{}\n\n{}", void_msg, worker_prompt);
        }

        if !apprentice_feedback.is_empty() {
            worker_prompt.push_str("\n\n【前ターンの弟子からの空間観測フィードバック】\n");
            worker_prompt.push_str(&apprentice_feedback);
        }

        let dispatch_msg = HiveMessage::SpawnSubAgent {
            id: subagent_id,
            objective: worker_prompt.clone(),
        };

        let turn_env = Envelope {
            message_id: Uuid::new_v4(),
            sender: "UserREPL".to_string(),
            recipient: "StealthGeminiWorker".to_string(),
            payload: serde_json::to_string(&dispatch_msg)?,
        };

        let pt = create_spinner("Thinking (Gemini Architect)...");
        let gemini_response_payload = match ephemeral_worker.receive(turn_env).await? {
            Some(reply) => {
                if let Ok(HiveMessage::Objective(res)) = serde_json::from_str(&reply.payload) {
                    res
                } else {
                    reply.payload
                }
            }
            None => {
                pt.finish_with_message(format!("{}", console::style("✖ Worker failed or returned empty!").red()));
                continue;
            }
        };
        pt.finish_and_clear();

        // --- State Machine Routing: `最終回答`, `編集中`, `最終回答仮`, `そのまま出力` ---
        let mut seen_final_answer = 0;
        let mut seen_editing = 0;
        let mut display_to_user = String::new();
        
        let trimmed_gemini_output = gemini_response_payload.trim_start();
        
        // Priority checks using `.contains` to be highly fault-tolerant against Gemini's conversational preamble
        // --- JCross MCP Tool Interception ---
        if trimmed_gemini_output.starts_with("REQUEST_FETCH_CODE:") {
            let path = trimmed_gemini_output.replace("REQUEST_FETCH_CODE:", "").trim().to_string();
            let clean_path = path.trim_matches('`').trim_matches('\'').trim_matches('"');
            println!("{}", console::style(format!("🛠️ [MCP Action] FETCH_RAW_CODE: {}", clean_path)).cyan());
            
            match std::fs::read_to_string(clean_path) {
                Ok(content) => {
                    auto_forward_payload = format!("[OBSERVATION (FETCH_CODE)]: \n```\n{}\n```", content);
                }
                Err(e) => {
                    auto_forward_payload = format!("[OBSERVATION ERROR]: Failed to read file: {}", e);
                }
            }
            continue;
        } else if trimmed_gemini_output.starts_with("REQUEST_JCROSS_MAP:") {
            let tag = trimmed_gemini_output.replace("REQUEST_JCROSS_MAP:", "").trim().to_string();
            let clean_tag = tag.trim_matches('`').trim_matches('\'').trim_matches('"');
            println!("{}", console::style(format!("🛠️ [MCP Action] READ_JCROSS_MAP: {}", clean_tag)).cyan());
            
            let nodes = spatial_index.query_nearest(clean_tag, 15);
            let mut summary = String::new();
            for n in nodes {
                summary.push_str(&format!("- Node [{}]: Concept: {}, Tags: {:?}\n", n.key, n.concept, n.kanji_tags));
            }
            
            if summary.is_empty() {
                auto_forward_payload = format!("[OBSERVATION (JCROSS_MAP)]: No matches found for tag '{}'.", clean_tag);
            } else {
                auto_forward_payload = format!("[OBSERVATION (JCROSS_MAP)] Nearest Concept Nodes:\n{}", summary);
            }
            continue;
        } else if trimmed_gemini_output.starts_with("REQUEST_TRACE_LOGIC:") {
            let node = trimmed_gemini_output.replace("REQUEST_TRACE_LOGIC:", "").trim().to_string();
            let clean_node = node.trim_matches('`').trim_matches('\'').trim_matches('"');
            println!("{}", console::style(format!("🛠️ [MCP Action] TRACE_LOGIC: {}", clean_node)).cyan());
            
            if let Some(target) = spatial_index.read_node(clean_node) {
                auto_forward_payload = format!("[OBSERVATION (TRACE_LOGIC)] Node Logic:\n{}\nRelations: {:?}", target.content, target.relations);
            } else {
                auto_forward_payload = format!("[OBSERVATION ERROR]: Node {} not found.", clean_node);
            }
            continue;
        } else if trimmed_gemini_output.contains(pfx_temp) {
            seen_final_answer += 1;
            
            let sp_senior = create_spinner("Auditing with Senior and generating Final Answer...");
            let request_title1 = if is_ja { "【ユーザーの元の要件】" } else { "[Original User Req]" };
            let request_title2 = if is_ja { "【出力結果】" } else { "[Result Output]" };
            let senior_dispatch = HiveMessage::Objective(format!(
                "{}\n{}\n\n{}\n{}",
                request_title1, query, request_title2, gemini_response_payload
            ));
            let senior_env = Envelope {
                message_id: Uuid::new_v4(),
                sender: "UserREPL".to_string(),
                recipient: "SeniorSupervisor".to_string(),
                payload: serde_json::to_string(&senior_dispatch)?,
            };

            if let Some(senior_reply) = senior_agent.receive(senior_env).await? {
                if let Ok(HiveMessage::Objective(senior_mod)) = serde_json::from_str(&senior_reply.payload) {
                    if senior_mod.contains(pfx_final) {
                        seen_final_answer += 1;
                    }
                    if seen_final_answer >= 2 {
                        display_to_user = senior_mod;
                    }
                }
            }
            sp_senior.finish_and_clear();

            if seen_final_answer >= 2 {
                println!("\n{}\n", display_to_user.trim());
                previous_worker_payload = display_to_user;
                focus_terminal();
                continue;
            }

        } else if trimmed_gemini_output.contains(pfx_raw) {
            let mut seen_raw = 1;
            
            let sp_senior = create_spinner("Auditing with Senior and generating Final Output...");
            let req_title_ja = format!("【ユーザーの元の要件】\n{}\n\n【出力結果】\n{}", query, gemini_response_payload);
            let req_title_en = format!("[Original User Req]\n{}\n\n[Result Output]\n{}", query, gemini_response_payload);
            let senior_dispatch = HiveMessage::Objective(if is_ja { req_title_ja } else { req_title_en });
            let senior_env = Envelope {
                message_id: Uuid::new_v4(),
                sender: "UserREPL".to_string(),
                recipient: "SeniorSupervisor".to_string(),
                payload: serde_json::to_string(&senior_dispatch)?,
            };

            if let Some(senior_reply) = senior_agent.receive(senior_env).await? {
                if let Ok(HiveMessage::Objective(senior_mod)) = serde_json::from_str(&senior_reply.payload) {
                    if senior_mod.contains(pfx_final_out) {
                        seen_raw += 1;
                    }
                    if seen_raw >= 2 {
                        display_to_user = senior_mod.clone();
                        
                        // Save Raw Output to Spatial Memory Front Zone
                        let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
                        let memory_key = format!("mem_{}_raw_output", timestamp);
                        let _ = spatial_index.write_front(&memory_key, &senior_mod).await;
                    }
                }
            }
            sp_senior.finish_and_clear();

            if seen_raw >= 2 {
                println!("\n{}\n", display_to_user.trim());
                previous_worker_payload = display_to_user;
                focus_terminal();
                continue;
            }

        } else if trimmed_gemini_output.contains(pfx_editing) {
            seen_editing += 1;
            
            let sp_senior = create_spinner("Parsing intent into Time-Series Memory...");
            
            let payload_title = if is_ja { "【出力結果】" } else { "[Result Output]" };
            let senior_dispatch = HiveMessage::Objective(format!(
                "{}\n{}",
                payload_title, gemini_response_payload
            ));
            let senior_env = Envelope {
                message_id: Uuid::new_v4(),
                sender: "UserREPL".to_string(),
                recipient: "SeniorSupervisor".to_string(),
                payload: serde_json::to_string(&senior_dispatch)?,
            };

            let mut pass1_output = String::new();
            if let Some(senior_reply) = senior_agent.receive(senior_env).await? {
                if let Ok(HiveMessage::Objective(senior_mod)) = serde_json::from_str(&senior_reply.payload) {
                    if senior_mod.contains(pfx_editing) {
                        seen_editing += 1;
                        pass1_output = senior_mod;
                    }
                }
            }
            sp_senior.finish_and_clear();

            if seen_editing >= 2 {
                // Save to Spatial Memory Front Zone (Task Intent)
                let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
                let memory_key = format!("mem_{}_edit_intent", timestamp);
                let _ = spatial_index.write_front(&memory_key, &pass1_output).await;

                // Safely extract instruction by finding everything AFTER prefix
                let parts: Vec<&str> = pass1_output.splitn(2, pfx_editing).collect();
                let qwen_clean_prompt = if parts.len() > 1 {
                    parts[1].trim_start()
                } else {
                    pass1_output.as_str()
                };
                
                // --- PHASE 2-B: Qwen Execution (Local SLM) ---
                let sp_exec = create_spinner("Executing tasks with Local Qwen Executor...");
                let local_slm = OllamaProvider::new("127.0.0.1", 11434);
                let req = InferenceRequest {
                    model: "qwen2.5:1.5b".to_string(),
                    sampling: SamplingParams::for_midweight(),
                    format: PromptFormat::OllamaChat,
                    stream: false,
                };
                
                let exec_sys_en = "You are the Executioner. Based on the instructions from the Architect (Gemini), strictly perform the code edits or terminal commands, and output the result report. No dialogue or emotion is needed.";
                let exec_sys_ja = "あなたはファイル編集やプロジェクトを操作する外部の協力者（Executer）です。Gemini（Architect）から与えられた指示に基づき、ファイルの編集案や必要な操作を厳密に行い、結果のレポートのみを出力してください。感情的な表現や会話は不要です。";
                
                let req_title_en = "[Tasks to execute]";
                let req_title_ja = "【実行するべきタスク】";

                let history = vec![
                    LlmMessage {
                        role: "system".to_string(),
                        content: if is_ja { exec_sys_ja.to_string() } else { exec_sys_en.to_string() },
                    },
                    LlmMessage {
                        role: "user".to_string(),
                        content: format!("{}\n{}", if is_ja { req_title_ja } else { req_title_en }, qwen_clean_prompt),
                    }
                ];

                let qwen_output = match local_slm.invoke(&req, &history).await {
                    Ok(out) => out,
                    Err(e) => {
                        format!("Local SLM unreachable: {}", e)
                    }
                };
                sp_exec.finish_and_clear();

                let report_header = console::style(if is_ja { "▶ 実行レポート" } else { "▶ Execution Report" }).color256(208).bold();
                println!("\n{}\n{}\n", report_header, qwen_output.trim());

                // --- PHASE 3: Supervisor Senior Hook (Memory Record ONLY for Execution Result) ---
                let exec_report = if is_ja { format!("【Qwenによる実行完了報告】\n実行結果:\n{}", qwen_output) } else { format!("[Qwen Execution Report]\nResult:\n{}", qwen_output) };
                let senior_dispatch_exec = HiveMessage::Objective(exec_report);
                let senior_env_exec = Envelope {
                    message_id: Uuid::new_v4(),
                    sender: "UserREPL".to_string(),
                    recipient: "SeniorSupervisor".to_string(),
                    payload: serde_json::to_string(&senior_dispatch_exec)?,
                };
                
                let sp_sync = create_spinner("Syncing execution results to spatial memory...");
                if let Some(senior_reply) = senior_agent.receive(senior_env_exec).await? {
                    if let Ok(HiveMessage::Objective(senior_mod)) = serde_json::from_str(&senior_reply.payload) {
                        let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
                        let memory_key = format!("mem_{}_qwen_exec", timestamp);
                        let _ = spatial_index.write_front(&memory_key, &senior_mod).await;
                    }
                }
                sp_sync.finish_and_clear();

                let sys_note = if is_ja { format!("【システム通知: コマンド実行結果】\n{}", qwen_output) } else { format!("[System Notification: CLI Execution Result]\n{}", qwen_output) };
                auto_forward_payload = sys_note;
                previous_worker_payload = qwen_output;
                focus_terminal();
                continue;
            }
        
        } else if trimmed_gemini_output.contains(pfx_final) {
            seen_final_answer += 1;
            
            let sp_senior = create_spinner("Parsing intent into Time-Series Memory...");
            let req_res = if is_ja { format!("【出力結果】\n{}", gemini_response_payload) } else { format!("[Result Output]\n{}", gemini_response_payload) };
            let senior_dispatch = HiveMessage::Objective(req_res);
            let senior_env = Envelope {
                message_id: Uuid::new_v4(),
                sender: "UserREPL".to_string(),
                recipient: "SeniorSupervisor".to_string(),
                payload: serde_json::to_string(&senior_dispatch)?,
            };

            if let Some(senior_reply) = senior_agent.receive(senior_env).await? {
                if let Ok(HiveMessage::Objective(senior_mod)) = serde_json::from_str(&senior_reply.payload) {
                    if senior_mod.contains(pfx_final) {
                        seen_final_answer += 1;
                    }
                    if seen_final_answer >= 2 {
                        display_to_user = senior_mod.clone();
                        
                        let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
                        let memory_key = format!("mem_{}_final_answer", timestamp);
                        let _ = spatial_index.write_front(&memory_key, &senior_mod).await;
                    }
                }
            }
            sp_senior.finish_and_clear();

            if seen_final_answer >= 2 {
                println!("\n{}\n", display_to_user.trim());
                previous_worker_payload = display_to_user;
                focus_terminal();
                continue;
            }

        } else {
            // Fallback for no prefix
            println!("\n{}", console::style("⚠ プレフィックスが検出されませんでした (Fallback/Silent Timeline)").yellow().dim());
            // Print up to 1000 chars of the end of the payload to see what Gemini actually generated
            let snippet = if gemini_response_payload.len() > 1000 {
                &gemini_response_payload[gemini_response_payload.len() - 1000..]
            } else {
                &gemini_response_payload
            };
            println!("\n{}\n", console::style(snippet.trim()).dim());
            previous_worker_payload = gemini_response_payload;
            continue;
        }

    } // End of REPL loop

    Ok(())
}
