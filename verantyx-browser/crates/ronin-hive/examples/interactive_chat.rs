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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Initialize minimalistic UI logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    println!("\n{}", console::style("=== VERANTYX DUAL-BROWSER INTERACTIVE SHELL ===").cyan().bold());
    println!("{}", console::style("Spawning Custom Stealth Browser and coordinating side-by-side with Safari...").dim());

    // 2. Dual-Window Visualization Orchestration (AppleScript)
    let split_screen_js = r#"
    do shell script "open -a Safari"
    delay 1.5
    
    tell application "Finder"
        set bnd to bounds of window of desktop
        set screenWidth to item 3 of bnd
        set screenHeight to item 4 of bnd
    end tell
    
    -- Biraz küçült, we want the width to be about 65% to show the desktop UI
    set winHeight to (screenHeight * 0.85) as integer
    set topMargin to 50
    set winWidth to (screenWidth * 0.65) as integer
    
    -- Boot Safari with 3 cascading overlapping windows that preserve Desktop Layout
    tell application "Safari"
        activate
        delay 0.5
        
        -- Left Window (Worker)
        make new document with properties {URL:"https://gemini.google.com/app"}
        set _w1 to front window
        set bounds of _w1 to {10, topMargin, 10 + winWidth, topMargin + winHeight}
        
        -- Middle Window (Senior)
        make new document with properties {URL:"https://gemini.google.com/app"}
        set _w2 to front window
        set bounds of _w2 to {100, topMargin, 100 + winWidth, topMargin + winHeight}
        
        -- Right Window (Apprentice)
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

    // Load Config (Wizard if first time)
    let mut config = ronin_hive::config::VerantyxConfig::load_or_wizard(&std::env::current_dir().unwrap());
    
    // Always prompt for Automation Mode on Interactive Chat launch
    let auto_selections = if config.language == "ja" { 
        vec!["🤖 完全自動モード (AutoStealth) - 物理キャリブレーションで自律稼働", "👤 手動モード (Manual) - 人間がCmd+Vで仲介する安全網"]
    } else { 
        vec!["🤖 AutoStealth Mode - Physical calibration autonomous operation", "👤 Manual Mode - Human intervenes via Cmd+V"]
    };
    
    let auto_selection = dialoguer::Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt(if config.language == "ja" { "通信・ペーストの自動化モードを選択してください" } else { "Select Automation Mode" })
        .default(if config.automation_mode == ronin_hive::config::AutomationMode::AutoStealth { 0 } else { 1 })
        .items(&auto_selections)
        .interact()
        .unwrap();
        
    config.automation_mode = if auto_selection == 0 {
        if config.language == "ja" {
            println!("\n{}", console::style("⚠️ 【System Requirement for AutoStealth Mode】 ⚠️").red().bold());
            println!("{}", console::style("完全自動モード（物理マウス操作）を利用するには、以下の環境設定が必須となります：").yellow().bold());
            println!("{}", console::style("  ・ MacBook 14インチ であること").cyan());
            println!("{}", console::style("  ・ ディスプレイ解像度が「デフォルト」に設定されていること").cyan());
            println!("{}", console::style("  ・ Safariのフォントおよびブラウザのズームが「標準（100%）」であること").cyan());
            println!("{}", console::style("設定が一致しない場合、マウスのクリック位置がズレて誤操作が発生する可能性があります。").yellow());
            println!("{}", console::style("設定が正しい人はそのまま利用できますが、設定が違う人はシステム設定を上記に合わせてから再度お試しください。\n").yellow());
        } else {
            println!("\n{}", console::style("⚠️ [System Requirement for AutoStealth Mode] ⚠️").red().bold());
            println!("{}", console::style("To use AutoStealth Mode, the following environment settings are REQUIRED:").yellow().bold());
            println!("{}", console::style("  - MacBook 14-inch").cyan());
            println!("{}", console::style("  - Display Resolution set to 'Default'").cyan());
            println!("{}", console::style("  - Safari font and zoom set to 'Standard (100%)'").cyan());
            println!("{}", console::style("If these settings do not match, the mouse click position will drift and cause misoperations.").yellow());
        }
        ronin_hive::config::AutomationMode::AutoStealth
    } else {
        ronin_hive::config::AutomationMode::Manual
    };
    
    // Save to persist their latest explicit choice
    let _ = config.save(&std::env::current_dir().unwrap());

    let is_ja = config.language == "ja";

    // 3. Spawn StealthGemini Actor
    let subagent_id = Uuid::new_v4();
    let mut ephemeral_worker = StealthWebActor::new(
        subagent_id,
        true, // global access
        std::env::current_dir().unwrap(), 
        "gemma-2-test".to_string(), 
        "Dual Browser Reactive REPL".to_string(), 
        999, // Infinite loop essentially
        is_ja, 
        ronin_hive::roles::stealth_gemini::SystemRole::ArchitectWorker, 
        1
    );

    let senior_id = Uuid::new_v4();
    let apprentice_id = Uuid::new_v4();
    let mut senior_agent = ronin_hive::roles::supervisor_gemini::SupervisorGeminiActor::new(senior_id, ronin_hive::roles::supervisor_gemini::SupervisorRank::Senior, is_ja);
    let mut apprentice_agent = ronin_hive::roles::supervisor_gemini::SupervisorGeminiActor::new(apprentice_id, ronin_hive::roles::supervisor_gemini::SupervisorRank::Apprentice, is_ja);

    // --- Boot Spatial Index Memory Engine ---
    let root_path = std::env::current_dir().unwrap().join(".ronin").join("experience.jcross");
    let mut spatial_index = SpatialIndex::new(root_path);
    if let Ok(count) = spatial_index.hydrate().await {
        if is_ja {
            println!("{}", console::style(format!("🧠 空間記憶エンジン起動... {}件の過去のノードをロードしました。", count)).magenta());
        } else {
            println!("{}", console::style(format!("🧠 Spatial Memory Engine Booted... Loaded {} past nodes.", count)).magenta());
        }
    }
    
    println!("{}", console::style("Booting Stealth Gemini Actor... Please wait for Internal WKWebView GUI to render.\n").dim());

    let mut conversation_turns = 0;
    let mut previous_worker_payload = String::new();
    let mut auto_forward_payload = String::new();

    // Define Prefixes
    let pfx_editing = if is_ja { "編集中" } else { "[EDITING]" };
    let pfx_raw = if is_ja { "そのまま出力" } else { "[RAW_OUTPUT]" };
    let pfx_final = if is_ja { "最終回答" } else { "[FINAL_ANSWER]" };
    let pfx_temp = if is_ja { "最終回答仮" } else { "[TEMP_FINAL]" };
    let pfx_final_out = if is_ja { "最終出力" } else { "[FINAL_OUTPUT]" };

    // 4. Interactive Chat Loop
    loop {
        let is_new_user_turn = auto_forward_payload.is_empty();
        
        let query = if !is_new_user_turn {
            let val = auto_forward_payload.clone();
            auto_forward_payload.clear();
            println!("{}", console::style("\n(Verantyx)> [System Auto Forwarding Execution Result to Worker]").dim());
            val
        } else {
            print!("{}", console::style("\n(Verantyx)> ").green().bold());
            io::stdout().flush().unwrap();
            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            input.trim().to_string()
        };

        if query == "exit" || query == "quit" {
            println!("Exiting Interactive Mode.");
            break;
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

                let new_worker_id = Uuid::new_v4();
                let new_senior_id = Uuid::new_v4();
                let new_apprentice_id = Uuid::new_v4();

                ephemeral_worker = StealthWebActor::new(
                    new_worker_id,
                    true,
                    std::env::current_dir().unwrap(), 
                    "gemma-2-test".to_string(), 
                    "Dual Browser Reactive REPL".to_string(), 
                    999,
                    is_ja, 
                    ronin_hive::roles::stealth_gemini::SystemRole::ArchitectWorker, 
                    1
                );
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

        println!("\n{}", console::style("[Phase 1] Launching Gemini Architect...").magenta().bold());
        let gemini_response_payload = match ephemeral_worker.receive(turn_env).await? {
            Some(reply) => {
                if let Ok(HiveMessage::Objective(res)) = serde_json::from_str(&reply.payload) {
                    res
                } else {
                    reply.payload
                }
            }
            None => {
                println!("{}", console::style("[Error] Worker failed or returned empty!").red());
                continue;
            }
        };

        // --- State Machine Routing: `最終回答`, `編集中`, `最終回答仮`, `そのまま出力` ---
        let mut seen_final_answer = 0;
        let mut seen_editing = 0;
        let mut display_to_user = String::new();
        
        let trimmed_gemini_output = gemini_response_payload.trim_start();
        
        // Priority checks using `.contains` to be highly fault-tolerant against Gemini's conversational preamble
        if trimmed_gemini_output.contains(pfx_temp) {
            println!("\n{}", console::style(format!("=== 💡 GEMINI STATE: {} (Temporary Final Answer) ===", pfx_temp)).green().bold());
            seen_final_answer += 1;
            
            println!("\n{}", console::style("[Phase 2] Prompting Senior for hallucination audit and prefix conversion...").magenta().bold());
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
                        let success_msg = if is_ja { "✅ シニアが「最終回答仮」を監査し、「最終回答」へ書き換えました。" } else { "✅ Senior audited Temp-Final and rewrote as Final Answer." };
                        println!("{}", console::style(success_msg).green().bold());
                    }
                }
            }

            if seen_final_answer >= 2 {
                println!("\n{}", console::style("=== USER OUTPUT (最終回答) ===").cyan().bold());
                println!("{}", display_to_user);
                println!("{}", console::style("=============================").cyan().bold());
                previous_worker_payload = display_to_user;
                focus_terminal();
                continue;
            }

        } else if trimmed_gemini_output.contains(pfx_raw) {
            let log_state = if is_ja { format!("=== 💡 GEMINI STATE: {} (Raw Output / Audit Needed) ===", pfx_raw) } else { format!("=== 💡 GEMINI STATE: {} (Audit Needed) ===", pfx_raw) };
            println!("\n{}", console::style(log_state).green().bold());
            let mut seen_raw = 1;
            
            let log_phase2 = if is_ja { "[Phase 2] Seniorへハルシネーション監査およびプレフィックス変換を要求します..." } else { "[Phase 2] Prompting Senior for hallucination audit and prefix conversion..." };
            println!("\n{}", console::style(log_phase2).magenta().bold());
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
                        let success_msg = if is_ja { format!("✅ シニアが「{}」を監査し、「{}」へ書き換えました。", pfx_raw, pfx_final_out) } else { format!("✅ Senior audited {} and rewrote as {}.", pfx_raw, pfx_final_out) };
                        println!("{}", console::style(success_msg).green().bold());
                        
                        // Save Raw Output to Spatial Memory Front Zone
                        let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
                        let memory_key = format!("mem_{}_raw_output", timestamp);
                        let _ = spatial_index.write_front(&memory_key, &senior_mod).await;
                        let save_msg = if is_ja { format!("💾 [記憶保存] 'Front' ゾーンに出力を物理保存しました ({}.md)", memory_key) } else { format!("💾 [Memory] Saved output to 'Front' zone ({}.md)", memory_key) };
                        println!("{}", console::style(save_msg).magenta());
                    }
                }
            }

            if seen_raw >= 2 {
                let log_final = if is_ja { format!("=== USER OUTPUT ({}) ===", pfx_final_out) } else { format!("=== USER OUTPUT ({}) ===", pfx_final_out) };
                println!("\n{}", console::style(log_final).cyan().bold());
                println!("{}", display_to_user);
                println!("{}", console::style("=============================").cyan().bold());
                previous_worker_payload = display_to_user;
                focus_terminal();
                continue;
            }

        } else if trimmed_gemini_output.contains(pfx_editing) {
            let log_state = if is_ja { format!("=== 📝 GEMINI STATE: {} (Editing Mode) ===", pfx_editing) } else { format!("=== 📝 GEMINI STATE: {} (Editing Mode) ===", pfx_editing) };
            println!("\n{}", console::style(log_state).yellow().bold());
            seen_editing += 1;
            
            let log_phase2a = if is_ja { "[Phase 2-A] Seniorへ純粋な時系列記憶として記録します (監査なし)..." } else { "[Phase 2-A] Saving to Senior time-series memory (no audit)..." };
            println!("\n{}", console::style(log_phase2a).magenta().bold());
            
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
                        let success_save = if is_ja { format!("✅ シニアが「{}」をそのまま記憶に保存しました。", pfx_editing) } else { format!("✅ Senior saved '{}' into memory.", pfx_editing) };
                        println!("{}", console::style(success_save).green().bold());
                    }
                }
            }

            if seen_editing >= 2 {
                // Save to Spatial Memory Front Zone (Task Intent)
                let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
                let memory_key = format!("mem_{}_edit_intent", timestamp);
                let _ = spatial_index.write_front(&memory_key, &pass1_output).await;
                
                let save_intent_msg = if is_ja { format!("💾 [記憶保存] 'Front' ゾーンにタスク意図を物理保存しました ({}.md)", memory_key) } else { format!("💾 [Memory] Saved task intent to 'Front' zone ({}.md)", memory_key) };
                println!("{}", console::style(save_intent_msg).magenta());

                // Safely extract instruction by finding everything AFTER prefix
                let parts: Vec<&str> = pass1_output.splitn(2, pfx_editing).collect();
                let qwen_clean_prompt = if parts.len() > 1 {
                    parts[1].trim_start()
                } else {
                    pass1_output.as_str()
                };
                
                // --- PHASE 2-B: Qwen Execution (Local SLM) ---
                println!("\n{}", console::style("[Phase 2-B] Handing over to Local Qwen (Executor) for File Edits...").yellow().bold());
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
                        println!("{}", console::style(format!("Local SLM unreachable: {}", e)).red());
                        "SLM Error".to_string()
                    }
                };

                println!("\n{}", console::style("=== QWEN EXECUTION RESULT ===").cyan().bold());
                println!("{}", qwen_output);
                println!("{}", console::style("=============================").cyan().bold());

                // --- PHASE 3: Supervisor Senior Hook (Memory Record ONLY for Execution Result) ---
                let log_phase3 = if is_ja { "[Phase 3] Senior Gemini Memory Sync of Execution Result (Non-destructive)..." } else { "[Phase 3] Senior Gemini Memory Sync of Execution Result (Non-destructive)..." };
                println!("\n{}", console::style(log_phase3).magenta().bold());
                
                let exec_report = if is_ja { format!("【Qwenによる実行完了報告】\n実行結果:\n{}", qwen_output) } else { format!("[Qwen Execution Report]\nResult:\n{}", qwen_output) };
                let senior_dispatch_exec = HiveMessage::Objective(exec_report);
                let senior_env_exec = Envelope {
                    message_id: Uuid::new_v4(),
                    sender: "UserREPL".to_string(),
                    recipient: "SeniorSupervisor".to_string(),
                    payload: serde_json::to_string(&senior_dispatch_exec)?,
                };
                
                if let Some(senior_reply) = senior_agent.receive(senior_env_exec).await? {
                    if let Ok(HiveMessage::Objective(senior_mod)) = serde_json::from_str(&senior_reply.payload) {
                        let ok_save_exec = if is_ja { "シニア側の時系列記憶にプレフィックスなしで安全に実行結果を保存完了しました。" } else { "Safely saved execution result to Senior's memory stream without prefix." };
                        println!("\n✅ {}", console::style(ok_save_exec).green().bold());
                        
                        // Save Qwen exec results to Spatial Memory Front Zone
                        let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
                        let memory_key = format!("mem_{}_qwen_exec", timestamp);
                        let _ = spatial_index.write_front(&memory_key, &senior_mod).await;
                        
                        let save_done = if is_ja { format!("💾 [記憶保存] 'Front' ゾーンに結果を物理保存しました ({}.md)", memory_key) } else { format!("💾 [Memory] Saved exec result to 'Front' zone ({}.md)", memory_key) };
                        println!("{}", console::style(save_done).magenta());
                    }
                }

                let sys_note = if is_ja { format!("【システム通知: コマンド実行結果】\n{}", qwen_output) } else { format!("[System Notification: CLI Execution Result]\n{}", qwen_output) };
                auto_forward_payload = sys_note;
                previous_worker_payload = qwen_output;
                focus_terminal();
                continue;
            }
        
        } else if trimmed_gemini_output.contains(pfx_final) {
            let state_final_log = if is_ja { format!("=== 💡 GEMINI STATE: {} (Final Answer) ===", pfx_final) } else { format!("=== 💡 GEMINI STATE: {} (Final Answer) ===", pfx_final) };
            println!("\n{}", console::style(state_final_log).green().bold());
            seen_final_answer += 1;
            
            let phase2_log = if is_ja { "[Phase 2] Seniorへ純粋な時系列記憶として記録します (監査なし)..." } else { "[Phase 2] Recording to Senior time-series memory (no audit)..." };
            println!("\n{}", console::style(phase2_log).magenta().bold());
            
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
                        println!("{}", console::style("✅ シニアが「最終回答」をそのまま記憶に保存し、再リレーしました。").green().bold());
                        
                        // Save Final Answer to Spatial Memory Front Zone
                        let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
                        let memory_key = format!("mem_{}_final_answer", timestamp);
                        let _ = spatial_index.write_front(&memory_key, &senior_mod).await;
                        println!("{}", console::style(format!("💾 [記憶保存] 'Front' ゾーンに最終回答を物理保存しました ({}.md)", memory_key)).magenta());
                    }
                }
            }

            if seen_final_answer >= 2 {
                println!("\n{}", console::style("=== USER OUTPUT (最終回答) ===").cyan().bold());
                println!("{}", display_to_user);
                println!("{}", console::style("=============================").cyan().bold());
                previous_worker_payload = display_to_user;
                focus_terminal();
                continue;
            }

        } else {
            // Fallback for no prefix
            println!("\n{}", console::style("⚠️ [Warning] Worker returned no valid prefix! Treating as silent timeline record.").yellow().bold());
            println!("{}", console::style("--- [EXTRACTED RAW PAYLOAD SNIPPET START] ---").dim());
            // Print up to 1000 chars of the end of the payload to see what Gemini actually generated
            let snippet = if gemini_response_payload.len() > 1000 {
                &gemini_response_payload[gemini_response_payload.len() - 1000..]
            } else {
                &gemini_response_payload
            };
            println!("{}", snippet.trim());
            println!("{}", console::style("--- [EXTRACTED RAW PAYLOAD SNIPPET END] ---").dim());
            previous_worker_payload = gemini_response_payload;
            continue;
        }

    } // End of REPL loop

    Ok(())
}
