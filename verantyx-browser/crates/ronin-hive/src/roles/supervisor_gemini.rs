use crate::actor::{Actor, Envelope};
use crate::messages::HiveMessage;
use async_trait::async_trait;
use tracing::warn;
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
                    "【{}への時系列監視・記憶および特例監査依頼】\nあなたは時系列や会話の流れを監視し記憶する役割です。\n\n[超重要・絶対遵守ルール]\n出力を生成する際、挨拶、要約、説明、過程の解説は【一切禁止】します。「了解しました」「データを監査しました」「結果を確認しました」などのテキストを含めてはなりません。プレフィックスの付与や変更は以下のルールに従ってください。\n\n1. データが「最終回答」または「編集中」のプレフィックスを持つ場合:\n決して内容を要約・修正せず、受け取ったテキストデータを『一言一句全く同じ配置・同じ文面』でそのまま出力してください。\n\n2. データが「最終回答仮」のプレフィックスを持つ場合:\n内容にハルシネーションがないか監査・編集を行ってください。その後、先頭のプレフィックスを必ず『最終回答』に変更して出力してください（例: `最終回答\\n[監査済みの結果]`）。\n\n3. データが「そのまま出力」のプレフィックスを持つ場合:\n内容にハルシネーションがないか監査・編集を行ってください。その後、先頭のプレフィックスを必ず『最終出力』に変更して出力してください。\n\n4. データに上記のどのプレフィックスも含まれていない場合（純粋な実行結果データやターミナルログなど）:\n絶対に内容を要約したり感想を述べたりせず、さらに何のプレフィックス（最終回答等）も後付けせず、『受け取ったデータの一言一句全く同じ配置・同じ文面』でそのまま出力（エコーバック）してください。本文の書き換えは厳禁です。\n\nデータ: {}", 
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
                    
                    let selections = &[" コピー完了・フォーカス移動待ち", " もう一度クリップボードに保存する"];
                    let selection = dialoguer::Select::new()
                        .with_prompt("どうしますか？ (矢印キーで選択)")
                        .default(0)
                        .items(&selections[..])
                        .interact()
                        .unwrap();

                    if selection == 0 {
                        let (window_name, pos_id) = if self.rank == SupervisorRank::Senior { 
                            ("【中央のシニア用ウィンドウ】", "middle") 
                        } else { 
                            ("【右側の弟子用ウィンドウ】", "right") 
                        };
                        
                        println!("{}", console::style(format!("🚀 クリップボードにロードしました！ {} にフォーカスを移動しました。Cmd+Vを押して送信してください！", window_name)).green().bold());
                        let _ = crate::roles::symbiotic_macos::SymbioticMacOS::focus_safari_panel(pos_id).await;
                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                        
                        let selections_confirm = &[" コピー完了。抽出を開始する"];
                        let _ = dialoguer::Select::new()
                            .with_prompt("✔ ミニパネルでGeminiの回答が生成されたら、回答を【コピー(Cmd+C)】してから選択してください")
                            .default(0)
                            .items(&selections_confirm[..])
                            .interact()
                            .unwrap();
                        
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
