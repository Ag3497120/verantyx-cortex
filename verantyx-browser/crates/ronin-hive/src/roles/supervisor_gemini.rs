use crate::actor::{Actor, Envelope};
use crate::messages::HiveMessage;
use async_trait::async_trait;
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Debug, PartialEq, Eq)]
pub enum SupervisorRank {
    Senior,
    Apprentice,
}

pub struct SupervisorGeminiActor {
    pub id: Uuid,
    pub rank: SupervisorRank,
}

impl SupervisorGeminiActor {
    pub fn new(id: Uuid, rank: SupervisorRank) -> Self {
        Self { id, rank }
    }
}

#[async_trait]
impl Actor for SupervisorGeminiActor {
    fn name(&self) -> &str {
        match self.rank {
            SupervisorRank::Senior => "SeniorSupervisorGemini",
            SupervisorRank::Apprentice => "ApprenticeSupervisorGemini",
        }
    }

    async fn receive(&mut self, env: Envelope) -> anyhow::Result<Option<Envelope>> {
        let msg: HiveMessage = match serde_json::from_str(&env.payload) {
            Ok(m) => m,
            Err(e) => {
                warn!("[{}] Failed to parse payload: {}", self.name(), e);
                return Ok(None);
            }
        };

        match msg {
            HiveMessage::Objective(task_data) => {
                let role_name = if self.rank == SupervisorRank::Senior { "シニア監視版" } else { "弟子監視版" };
                
                let prompt = format!(
                    "【{}へのプロンプト検証依頼】\n以下のローカルSLM出力を監視・検閲し、ユーザーの真意から逸脱していないか評価せよ。\n\n{}", 
                    role_name, task_data
                );

                // --- HUMAN IN THE LOOP CLIPBOARD FLOW ---
                loop {
                    println!("\n{}", console::style(format!("╭─ [ Verantyx: {} サブタスク ] ──────────────────", role_name)).yellow().bold());
                    println!("{} 📝 これから【{}】版に送ります。クリップボードに保存します...", console::style("│").yellow().bold(), role_name);

                    let _ = crate::roles::symbiotic_macos::SymbioticMacOS::set_clipboard(&prompt).await;
                    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

                    println!("{} ✔ 保存しました！内容は以下の通りです:", console::style("│").green().bold());
                    println!("{} {}", console::style("│").green(), console::style(prompt.chars().take(150).collect::<String>() + "...").dim());
                    println!("{}", console::style(format!("╰─────────────────────────────────────────────────────")).yellow().bold());

                    println!("\n{}", console::style("👉 クリップボードの準備が完了しました。送信先のブラウザを開きますか？").cyan().bold());
                    
                    let selections = &[" ブラウザを開いて送信する (Cmd+V)", " もう一度クリップボードに保存する"];
                    let selection = dialoguer::Select::new()
                        .with_prompt("どうしますか？ (矢印キーで選択)")
                        .default(0)
                        .items(&selections[..])
                        .interact()
                        .unwrap();

                    if selection == 0 {
                        println!("{}", console::style("🚀 クリップボードにロードしました！フローティング・ミニパネル（Safari）で Cmd+V を押して送信してください！").green().bold());
                        let _ = crate::roles::symbiotic_macos::SymbioticMacOS::open_safari_mini_panel("https://gemini.google.com/app").await;
                        tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
                        
                        // Wait using simple read_line (avoids dialoguer/crossterm seizing raw TTY focus)
                        println!("{}", console::style("✔ ミニパネルでGeminiの回答が生成されたら、回答を【コピー(Cmd+C)】してからこのCLIでエンターキーを押してください。").yellow().bold());
                        let mut wait_buf = String::new();
                        std::io::stdin().read_line(&mut wait_buf).unwrap();
                        
                        let gemini_response = crate::roles::symbiotic_macos::SymbioticMacOS::get_clipboard().await.unwrap_or_default();
                        println!("{}", console::style(format!("✔ クリップボードからGeminiの回答を読み取りました！({}文字)", gemini_response.chars().count())).green());

                        // Return the actual response
                        let reply = HiveMessage::Objective(gemini_response);
                        return Ok(Some(Envelope {
                            message_id: Uuid::new_v4(),
                            sender: self.name().to_string(),
                            recipient: env.sender,
                            payload: serde_json::to_string(&reply)?,
                        }));
                    } else {
                        println!("{}", console::style("🔄 [もう一度クリップボードに保存] を選択しました。").yellow());
                        continue;
                    }
                }
            },
            _ => Ok(None)
        }
    }
}
