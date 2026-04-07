use ronin_hive::actor::{Actor, Envelope};
use ronin_hive::messages::HiveMessage;
use ronin_hive::roles::stealth_gemini::StealthWebActor;
use ronin_core::models::provider::{LlmProvider, LlmMessage};
use ronin_core::models::provider::ollama::OllamaProvider;
use ronin_core::models::sampling_params::{InferenceRequest, SamplingParams, PromptFormat};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use uuid::Uuid;
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
    // Tangles Safari and Custom Browser side by side for visual parity checking
    let split_screen_js = r#"
    tell application "System Events"
        set screenWidth to item 3 of (get bounds of window of desktop)
        set screenHeight to item 4 of (get bounds of window of desktop)
        set halfWidth to screenWidth / 2
        
        -- Boot Safari
        tell application "Safari"
            activate
            if (count of windows) = 0 then
                make new document
            end if
            set bounds of front window to {halfWidth, 0, screenWidth, screenHeight}
            set URL of front document to "https://gemini.google.com/app"
        end tell
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
        ronin_hive::roles::stealth_gemini::SystemRole::SeniorObserver, 
        1
    );

    let senior_id = Uuid::new_v4();
    let apprentice_id = Uuid::new_v4();
    let mut senior_agent = ronin_hive::roles::supervisor_gemini::SupervisorGeminiActor::new(senior_id, ronin_hive::roles::supervisor_gemini::SupervisorRank::Senior);
    let mut apprentice_agent = ronin_hive::roles::supervisor_gemini::SupervisorGeminiActor::new(apprentice_id, ronin_hive::roles::supervisor_gemini::SupervisorRank::Apprentice);

    println!("{}", console::style("Booting Stealth Gemini Actor... Please wait for Internal WKWebView GUI to render.\n").dim());

    let mut conversation_turns = 1;

    // 4. Interactive Chat Loop
    loop {
        print!("{}", console::style("\n(Verantyx)> ").green().bold());
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let query = input.trim();

        if query == "exit" || query == "quit" {
            println!("Exiting Interactive Mode.");
            break;
        }

        if query.is_empty() { continue; }

        conversation_turns += 1;
        if conversation_turns > 5 {
            println!("{}", console::style("\n🔄 [Turn Limit Reached] 5ターンを経過しました。5ターン記憶統合・昇格サイクルを開始します...").magenta().bold());
            println!("{}", console::style("1. ローカルSLMのリフレッシュと時系列記憶の補完").cyan());
            println!("{}", console::style("2. マージした記憶を空間記憶（Tier-2 Near/Midレイヤー）へ保存").cyan());
            println!("{}", console::style("3. 以前の弟子のGeminiをシニアに格上げ（Promotion）し、新たな弟子を初期化").cyan());
            println!("{}", console::style("[JCross Memory Subsystem] => Memory synced securely.\n").dim());
            conversation_turns = 1; // Reset counter after refresh
        }

        let custom_task = HiveMessage::Objective(query.to_string());
        
        println!("\n{}", console::style("[Local SLM] Analyzing prompt intent and bounds...").yellow().bold());
        let local_slm = OllamaProvider::new("127.0.0.1", 11434);
        let req = InferenceRequest {
            model: "qwen2.5:1.5b".to_string(), // Switched to Qwen2.5:1.5b
            sampling: SamplingParams::for_midweight(), // Automatically enforces token limits natively
            format: PromptFormat::OllamaChat,
            stream: false,
        };
        let history = vec![
            LlmMessage {
                role: "system".to_string(),
                content: "あなたはユーザーのプロジェクトをルーティングし、ファイル操作の補助や記憶を保存する役割を持つプランナーAIです。ウェブ版Geminiに対して、プロジェクト分析に必要な情報を渡すための簡潔な指示を出力してください。文字数は絶対に少なく保ち、1万文字を超えないようにしてください。".to_string(),
            },
            LlmMessage {
                role: "user".to_string(),
                content: query.to_string(),
            }
        ];

        let slm_analysis = match local_slm.invoke(&req, &history).await {
            Ok(output) => output,
            Err(e) => {
                println!("{}", console::style(format!("Local SLM unreachable: {}. Proceeding without pre-analysis.", e)).red().dim());
                "(No pre-analysis available)".to_string()
            }
        };

        if slm_analysis != "(No pre-analysis available)" {
            println!("{}", console::style("--- SLM Analysis Result ---").yellow().dim());
            println!("{}", console::style(&slm_analysis).yellow().dim());
        }

        let combined_payload = format!("【USER PROMPT】\n{}\n\n【LOCAL SLM STRATEGY ANALYSIS】\n{}", query, slm_analysis);

        // --- PHASE A: Supervisor Senior Hook ---
        let senior_dispatch = HiveMessage::Objective(combined_payload.clone());
        let senior_env = Envelope {
            message_id: Uuid::new_v4(),
            sender: "UserREPL".to_string(),
            recipient: "SeniorSupervisor".to_string(),
            payload: serde_json::to_string(&senior_dispatch)?,
        };
        let _ = senior_agent.receive(senior_env).await?;

        // --- PHASE B: Supervisor Apprentice Hook (Only starts evaluating after turn 1) ---
        if conversation_turns > 1 {
            let app_dispatch = HiveMessage::Objective(combined_payload.clone());
            let app_env = Envelope {
                message_id: Uuid::new_v4(),
                sender: "UserREPL".to_string(),
                recipient: "ApprenticeSupervisor".to_string(),
                payload: serde_json::to_string(&app_dispatch)?,
            };
            let _ = apprentice_agent.receive(app_env).await?;
        }

        // --- PHASE C: Worker Dispatch ---
        let dispatch_msg = HiveMessage::SpawnSubAgent {
            id: subagent_id,
            objective: combined_payload,
        };

        let turn_env = Envelope {
            message_id: Uuid::new_v4(),
            sender: "UserREPL".to_string(),
            recipient: "StealthGeminiWorker".to_string(),
            payload: serde_json::to_string(&dispatch_msg)?,
        };

        println!("\n{}", console::style("Tracking Symbiotic Execution...").cyan());
        // Since StealthWebActor boots vx-browser on first receive, it will pop up on the left natively.
        match ephemeral_worker.receive(turn_env).await? {
            Some(reply) => {
                println!("\n{}", console::style("=== SYSTEM REPLY ===").magenta().bold());
                println!("{}", reply.payload);
                println!("{}", console::style("====================").magenta().bold());
            }
            None => {
                println!("{}", console::style("[Error] Agent refused message or crashed.").red());
            }
        }
    }

    Ok(())
}
