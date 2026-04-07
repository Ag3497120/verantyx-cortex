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
                        println!("{}", console::style("🚀 ブラウザをアクティブにします。入力欄で Cmd+V を押して送信してください！").green().bold());
                        let _ = crate::roles::symbiotic_macos::SymbioticMacOS::focus_app("Safari").await;
                        break;
                    } else {
                        println!("{}", console::style("🔄 [もう一度クリップボードに保存] を選択しました。").yellow());
                        continue;
                    }
                }
                
                // Simulated API return for the internal REPL to track temporal state.
                let reply = HiveMessage::Objective(format!("[{}] 検閲完了（ユーザー手動送信確認済み）", role_name));
                Ok(Some(Envelope {
                    message_id: Uuid::new_v4(),
                    sender: self.name().to_string(),
                    recipient: env.sender,
                    payload: serde_json::to_string(&reply)?,
                }))
            },
            _ => Ok(None)
        }
    }
}
