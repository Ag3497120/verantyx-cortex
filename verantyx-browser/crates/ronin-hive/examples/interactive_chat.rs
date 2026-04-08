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
    tell application "Finder"
        set bnd to bounds of window of desktop
        set screenWidth to item 3 of bnd
        set screenHeight to item 4 of bnd
    end tell
    
    -- 少し小さくするため、高さを85%に、幅も少し余裕を持たせる
    set winHeight to (screenHeight * 0.85) as integer
    set topMargin to 50
    set colWidth to (screenWidth / 3) as integer
    set marginX to 10
    
    -- Boot Safari with 3 side-by-side windows
    tell application "Safari"
        activate
        delay 0.5
        
        -- Left Window (Worker)
        make new document with properties {URL:"https://gemini.google.com/app"}
        set _w1 to front window
        set bounds of _w1 to {marginX, topMargin, colWidth - marginX, topMargin + winHeight}
        
        -- Middle Window (Senior)
        make new document with properties {URL:"https://gemini.google.com/app"}
        set _w2 to front window
        set bounds of _w2 to {colWidth + marginX, topMargin, (colWidth * 2) - marginX, topMargin + winHeight}
        
        -- Right Window (Apprentice)
        make new document with properties {URL:"https://gemini.google.com/app"}
        set _w3 to front window
        set bounds of _w3 to {(colWidth * 2) + marginX, topMargin, screenWidth - marginX, topMargin + winHeight}
    end tell
    "#;
    let _ = tokio::process::Command::new("osascript")
        .arg("-e")
        .arg(split_screen_js)
        .output()
        .await;

    // 3. Spawn StealthGemini Actor
    let subagent_id = Uuid::new_v4();
    let mut ephemeral_worker = StealthWebActor::new(
        subagent_id,
        true, // global access
        std::env::current_dir().unwrap(), 
        "gemma-2-test".to_string(), 
        "Dual Browser Reactive REPL".to_string(), 
        999, // Infinite loop essentially
        false, 
        ronin_hive::roles::stealth_gemini::SystemRole::ArchitectWorker, 
        1
    );

    let senior_id = Uuid::new_v4();
    let apprentice_id = Uuid::new_v4();
    let mut senior_agent = ronin_hive::roles::supervisor_gemini::SupervisorGeminiActor::new(senior_id, ronin_hive::roles::supervisor_gemini::SupervisorRank::Senior);
    let mut apprentice_agent = ronin_hive::roles::supervisor_gemini::SupervisorGeminiActor::new(apprentice_id, ronin_hive::roles::supervisor_gemini::SupervisorRank::Apprentice);

    // --- Boot Spatial Index Memory Engine ---
    let root_path = std::env::current_dir().unwrap().join(".ronin").join("experience.jcross");
    let mut spatial_index = SpatialIndex::new(root_path);
    if let Ok(count) = spatial_index.hydrate().await {
        println!("{}", console::style(format!("🧠 空間記憶エンジン起動... {}件の過去のノードをロードしました。", count)).magenta());
    }
    
    println!("{}", console::style("Booting Stealth Gemini Actor... Please wait for Internal WKWebView GUI to render.\n").dim());

    let mut conversation_turns = 0;
    let mut previous_worker_payload = String::new();
    let mut auto_forward_payload = String::new();

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
                println!("{}", console::style("\n🔄 [Turn Limit Reached] 5ターンを経過しました。記憶の抽出とリレー大移動を開始します...").magenta().bold());
                let relay_prompt = r#"
【強制コマンド：次世代Geminiへの記憶リレー抽出】
現在の時系列の内容とこれまでの会話の流れをすべて出力せよ。
あなたは間もなくシャットダウンされ、次にあなたと全く同じ初期プロンプトを持った新しいGemini（シニア・弟子・ワーカー）が立ち上がります。
そのため、あなたの現在の内部記憶状態の詳細をまとめて、次のGeminiに渡すための引き継ぎテキストを生成してください。
もし文字数が1万文字を超えそうな場合は、直近10件分の情報のみ詳細に記述し、それより前の時系列については要約して、全体が確実に1万文字以内に収まるように圧縮してください。
"#;
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
                    false, 
                    ronin_hive::roles::stealth_gemini::SystemRole::ArchitectWorker, 
                    1
                );
                senior_agent = ronin_hive::roles::supervisor_gemini::SupervisorGeminiActor::new(
                    new_senior_id,
                    ronin_hive::roles::supervisor_gemini::SupervisorRank::Senior
                );
                apprentice_agent = ronin_hive::roles::supervisor_gemini::SupervisorGeminiActor::new(
                    new_apprentice_id,
                    ronin_hive::roles::supervisor_gemini::SupervisorRank::Apprentice
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
        if trimmed_gemini_output.contains("最終回答仮") {
            println!("\n{}", console::style("=== 💡 GEMINI STATE: 最終回答仮 (Temporary Final Answer) ===").green().bold());
            seen_final_answer += 1;
            
            println!("\n{}", console::style("[Phase 2] Seniorへハルシネーション監査およびプレフィックス変換を要求します...").magenta().bold());
            let senior_dispatch = HiveMessage::Objective(format!(
                "【ユーザーの元の要件】\n{}\n\n【出力結果】\n{}",
                query, gemini_response_payload
            ));
            let senior_env = Envelope {
                message_id: Uuid::new_v4(),
                sender: "UserREPL".to_string(),
                recipient: "SeniorSupervisor".to_string(),
                payload: serde_json::to_string(&senior_dispatch)?,
            };

            if let Some(senior_reply) = senior_agent.receive(senior_env).await? {
                if let Ok(HiveMessage::Objective(senior_mod)) = serde_json::from_str(&senior_reply.payload) {
                    if senior_mod.starts_with("最終回答") {
                        seen_final_answer += 1;
                    }
                    if seen_final_answer >= 2 {
                        display_to_user = senior_mod;
                        println!("{}", console::style("✅ シニアが「最終回答仮」を監査し、「最終回答」へ書き換えました。").green().bold());
                    }
                }
            }

            if seen_final_answer >= 2 {
                println!("\n{}", console::style("=== USER OUTPUT (最終回答) ===").cyan().bold());
                println!("{}", display_to_user);
                println!("{}", console::style("=============================").cyan().bold());
                previous_worker_payload = display_to_user;
                continue;
            }

        } else if trimmed_gemini_output.contains("そのまま出力") {
            println!("\n{}", console::style("=== 💡 GEMINI STATE: そのまま出力 (Raw Output / Audit Needed) ===").green().bold());
            let mut seen_raw = 1;
            
            println!("\n{}", console::style("[Phase 2] Seniorへハルシネーション監査およびプレフィックス変換を要求します...").magenta().bold());
            let senior_dispatch = HiveMessage::Objective(format!(
                "【ユーザーの元の要件】\n{}\n\n【出力結果】\n{}",
                query, gemini_response_payload
            ));
            let senior_env = Envelope {
                message_id: Uuid::new_v4(),
                sender: "UserREPL".to_string(),
                recipient: "SeniorSupervisor".to_string(),
                payload: serde_json::to_string(&senior_dispatch)?,
            };

            if let Some(senior_reply) = senior_agent.receive(senior_env).await? {
                if let Ok(HiveMessage::Objective(senior_mod)) = serde_json::from_str(&senior_reply.payload) {
                    if senior_mod.contains("最終出力") {
                        seen_raw += 1;
                    }
                    if seen_raw >= 2 {
                        display_to_user = senior_mod.clone();
                        println!("{}", console::style("✅ シニアが「そのまま出力」を監査し、「最終出力」へ書き換えました。").green().bold());
                        
                        // Save Raw Output to Spatial Memory Front Zone
                        let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
                        let memory_key = format!("mem_{}_raw_output", timestamp);
                        let _ = spatial_index.write_front(&memory_key, &senior_mod).await;
                        println!("{}", console::style(format!("💾 [記憶保存] 'Front' ゾーンに出力を物理保存しました ({}.md)", memory_key)).magenta());
                    }
                }
            }

            if seen_raw >= 2 {
                println!("\n{}", console::style("=== USER OUTPUT (最終出力) ===").cyan().bold());
                println!("{}", display_to_user);
                println!("{}", console::style("=============================").cyan().bold());
                previous_worker_payload = display_to_user;
                continue;
            }

        } else if trimmed_gemini_output.contains("編集中") {
            println!("\n{}", console::style("=== 📝 GEMINI STATE: 編集中 (Editing Mode) ===").yellow().bold());
            seen_editing += 1;
            
            println!("\n{}", console::style("[Phase 2-A] Seniorへ純粋な時系列記憶として記録します (監査なし)...").magenta().bold());
            let senior_dispatch = HiveMessage::Objective(format!(
                "【出力結果】\n{}",
                gemini_response_payload
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
                    if senior_mod.contains("編集中") {
                        seen_editing += 1;
                        pass1_output = senior_mod;
                        println!("{}", console::style("✅ シニアが「編集中」をそのまま記憶に保存しました。").green().bold());
                    }
                }
            }

            if seen_editing >= 2 {
                // Save to Spatial Memory Front Zone (Task Intent)
                let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
                let memory_key = format!("mem_{}_edit_intent", timestamp);
                let _ = spatial_index.write_front(&memory_key, &pass1_output).await;
                println!("{}", console::style(format!("💾 [記憶保存] 'Front' ゾーンにタスク意図を物理保存しました ({}.md)", memory_key)).magenta());

                // Safely extract instruction by finding everything AFTER "編集中"
                let parts: Vec<&str> = pass1_output.splitn(2, "編集中").collect();
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
                
                let history = vec![
                    LlmMessage {
                        role: "system".to_string(),
                        content: "あなたはファイル編集やプロジェクトを操作する外部の協力者（Executer）です。Gemini（Architect）から与えられた指示に基づき、ファイルの編集案や必要な操作を厳密に行い、結果のレポートのみを出力してください。感情的な表現や会話は不要です。".to_string(),
                    },
                    LlmMessage {
                        role: "user".to_string(),
                        content: format!("【実行するべきタスク】\n{}", qwen_clean_prompt),
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
                println!("\n{}", console::style("[Phase 3] Senior Gemini Memory Sync of Execution Result (Non-destructive)...").magenta().bold());
                let senior_dispatch_exec = HiveMessage::Objective(format!(
                    "【Qwenによる実行完了報告】\n実行結果:\n{}",
                    qwen_output
                ));
                let senior_env_exec = Envelope {
                    message_id: Uuid::new_v4(),
                    sender: "UserREPL".to_string(),
                    recipient: "SeniorSupervisor".to_string(),
                    payload: serde_json::to_string(&senior_dispatch_exec)?,
                };
                
                if let Some(senior_reply) = senior_agent.receive(senior_env_exec).await? {
                    if let Ok(HiveMessage::Objective(senior_mod)) = serde_json::from_str(&senior_reply.payload) {
                        println!("\n✅ {}", console::style("シニア側の時系列記憶にプレフィックスなしで安全に実行結果を保存完了しました。").green().bold());
                        // Save Qwen exec results to Spatial Memory Front Zone
                        let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
                        let memory_key = format!("mem_{}_qwen_exec", timestamp);
                        let _ = spatial_index.write_front(&memory_key, &senior_mod).await;
                        println!("{}", console::style(format!("💾 [記憶保存] 'Front' ゾーンに結果を物理保存しました ({}.md)", memory_key)).magenta());
                    }
                }

                auto_forward_payload = format!("【システム通知: コマンド実行結果】\n{}", qwen_output);
                previous_worker_payload = qwen_output;
                continue;
            }
        
        } else if trimmed_gemini_output.contains("最終回答") {
            println!("\n{}", console::style("=== 💡 GEMINI STATE: 最終回答 (Final Answer) ===").green().bold());
            seen_final_answer += 1;
            
            println!("\n{}", console::style("[Phase 2] Seniorへ純粋な時系列記憶として記録します (監査なし)...").magenta().bold());
            let senior_dispatch = HiveMessage::Objective(format!(
                "【出力結果】\n{}",
                gemini_response_payload
            ));
            let senior_env = Envelope {
                message_id: Uuid::new_v4(),
                sender: "UserREPL".to_string(),
                recipient: "SeniorSupervisor".to_string(),
                payload: serde_json::to_string(&senior_dispatch)?,
            };

            if let Some(senior_reply) = senior_agent.receive(senior_env).await? {
                if let Ok(HiveMessage::Objective(senior_mod)) = serde_json::from_str(&senior_reply.payload) {
                    if senior_mod.contains("最終回答") {
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
                continue;
            }

        } else {
            // Fallback for no prefix
            println!("\n{}", console::style("⚠️ [Warning] Worker returned no valid prefix! Treating as silent timeline record.").yellow().bold());
            previous_worker_payload = gemini_response_payload;
            continue;
        }

    } // End of REPL loop

    Ok(())
}
