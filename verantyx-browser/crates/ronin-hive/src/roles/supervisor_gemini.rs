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
    pub is_ja: bool,
}

impl SupervisorGeminiActor {
    pub fn new(id: Uuid, rank: SupervisorRank, is_ja: bool) -> Self {
        Self { id, rank, is_ja }
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
                let role_name = if self.rank == SupervisorRank::Senior { 
                    if self.is_ja { "シニア監視版" } else { "Senior Observer" }
                } else { 
                    if self.is_ja { "弟子監視版" } else { "Apprentice Observer" }
                };
                
                let prompt = if self.is_ja {
                    format!(
                        "【{}への時系列監視・記憶および特例監査依頼】\nあなたは時系列や会話の流れを監視し記憶する役割です。\n\n[超重要・絶対遵守ルール]\n出力を生成する際、挨拶、要約、説明、過程の解説は【一切禁止】します。「了解しました」「データを監査しました」「結果を確認しました」などのテキストを含めてはなりません。プレフィックスの付与や変更は以下のルールに従ってください。\n\n1. データが「最終回答」または「編集中」のプレフィックスを持つ場合:\n決して内容を要約・修正せず、受け取ったテキストデータを『一言一句全く同じ配置・同じ文面』でそのまま出力してください。\n\n2. データが「最終回答仮」のプレフィックスを持つ場合:\n内容にハルシネーションがないか監査・編集を行ってください。その後、先頭のプレフィックスを必ず『最終回答』に変更して出力してください（例: `最終回答\\n[監査済みの結果]`）。\n\n3. データが「そのまま出力」のプレフィックスを持つ場合:\n内容にハルシネーションがないか監査・編集を行ってください。その後、先頭のプレフィックスを必ず『最終出力』に変更して出力してください。\n\n4. データに上記のどのプレフィックスも含まれていない場合（純粋な実行結果データやターミナルログなど）:\n絶対に内容を要約したり感想を述べたりせず、さらに何のプレフィックス（最終回答等）も後付けせず、『受け取ったデータの一言一句全く同じ配置・同じ文面』でそのまま出力（エコーバック）してください。本文の書き換えは厳禁です。\n\nデータ: {}", 
                        role_name, task_data
                    )
                } else {
                    format!(
                        "[Observation and Audit Request for {}]\nYou are responsible for monitoring the timeline and conversation flow.\n\n[CRITICAL RULE]\nDo NOT include greetings, summaries, explanations, or process commentaries. Phrases like \"Understood\" or \"Audit complete\" are STRICTLY PROHIBITED. Follow these prefix rules exactly:\n\n1. If data has `[FINAL_ANSWER]` or `[EDITING]` prefix:\nDo NOT summarize or modify. Output the received payload EXACTLY as is, echoing the exact phrasing and position.\n\n2. If data has `[TEMP_FINAL]` prefix:\nAudit for hallucinations and edit if necessary. Then, CHANGE the prefix to `[FINAL_ANSWER]` and output (e.g., `[FINAL_ANSWER]\\n[audited result]`).\n\n3. If data has `[RAW_OUTPUT]` prefix:\nAudit for hallucinations and edit if necessary. Then, CHANGE the prefix to `[FINAL_OUTPUT]` and output.\n\n4. If data does NOT have any of the above prefixes (e.g. pure execution log):\nDo NOT summarize, do NOT express opinions, and do NOT prepend ANY prefix like `[FINAL_ANSWER]`. Echo back the EXACT string without altering a single character. Strictly verbatim.\n\nData: {}", 
                        role_name, task_data
                    )
                };

                let prompt_title = if self.is_ja { "サブタスク" } else { "Subtask" };
                let send_msg = if self.is_ja { format!("これから【{}】版に送ります。クリップボードに保存します...", role_name) } else { format!("Sending to {}. Copying to clipboard...", role_name) };
                let saved_msg = if self.is_ja { "保存しました！内容は以下の通りです:" } else { "Saved! Content snippet:" };
                let ready_msg = if self.is_ja { "👉 クリップボードの準備が完了しました。送信先のブラウザを開きますか？" } else { "👉 Clipboard is ready. Focus the browser tab?" };

                // --- HUMAN IN THE LOOP CLIPBOARD FLOW ---
                loop {
                    println!("\n{}", console::style(format!("╭─ [ Verantyx: {} {} ] ──────────────────", role_name, prompt_title)).yellow().bold());
                    println!("{} 📝 {}", console::style("│").yellow().bold(), send_msg);

                    let _ = crate::roles::symbiotic_macos::SymbioticMacOS::set_clipboard(&prompt).await;
                    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

                    println!("{} ✔ {}", console::style("│").green().bold(), saved_msg);
                    println!("{} {}", console::style("│").green(), console::style(prompt.chars().take(150).collect::<String>() + "...").dim());
                    println!("{}", console::style("╰─────────────────────────────────────────────────────").yellow().bold());

                    println!("\n{}", console::style(ready_msg).cyan().bold());
                    
                    let selections = if self.is_ja {
                        vec![" コピー完了・フォーカス移動待ち", " もう一度クリップボードに保存する"]
                    } else {
                        vec![" Copy Complete. Wait for focus", " Copy to clipboard again"]
                    };

                    let prompt_msg = if self.is_ja { "どうしますか？ (矢印キーで選択)" } else { "Action? (Up/Down arrow)" };
                    let selection = dialoguer::Select::new()
                        .with_prompt(prompt_msg)
                        .default(0)
                        .items(&selections[..])
                        .interact()
                        .unwrap();

                    if selection == 0 {
                        let (window_name, pos_id) = if self.rank == SupervisorRank::Senior { 
                            if self.is_ja { ("【中央のシニア用ウィンドウ】", "middle") } else { ("[Middle Senior Window]", "middle") }
                        } else { 
                            if self.is_ja { ("【右側の弟子用ウィンドウ】", "right") } else { ("[Right Apprentice Window]", "right") }
                        };
                        
                        let sent_msg = if self.is_ja {
                            format!("🚀 クリップボードにロードしました！ {} にフォーカスを移動しました。Cmd+Vを押して送信してください！", window_name)
                        } else {
                            format!("🚀 Copied to clipboard! Focused {}. Press Cmd+V and Enter!", window_name)
                        };
                        println!("{}", console::style(sent_msg).green().bold());
                        let _ = crate::roles::symbiotic_macos::SymbioticMacOS::focus_safari_panel(pos_id).await;
                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                        
                        let selections_confirm = if self.is_ja { vec![" コピー完了。抽出を開始する"] } else { vec![" Extraction Ready"] };
                        let confirm_prompt = if self.is_ja { "✔ ミニパネルでGeminiの回答が生成されたら、回答を【コピー(Cmd+C)】してから選択してください" } else { "✔ After Gemini generates the answer, COPY IT (Cmd+C) and press Enter" };
                        let _ = dialoguer::Select::new()
                            .with_prompt(confirm_prompt)
                            .default(0)
                            .items(&selections_confirm[..])
                            .interact()
                            .unwrap();
                        
                        let gemini_response = crate::roles::symbiotic_macos::SymbioticMacOS::get_clipboard().await.unwrap_or_default();
                        
                        let success_ext = if self.is_ja { format!("✔ クリップボードからGeminiの回答を読み取りました！({}文字)", gemini_response.chars().count()) } else { format!("✔ Extracted Gemini response from clipboard! ({} chars)", gemini_response.chars().count()) };
                        println!("{}", console::style(success_ext).green());

                        // Return the actual response
                        let reply = HiveMessage::Objective(gemini_response);
                        return Ok(Some(Envelope {
                            message_id: Uuid::new_v4(),
                            sender: self.name().to_string(),
                            recipient: env.sender,
                            payload: serde_json::to_string(&reply)?,
                        }));
                    } else {
                        let retry_msg = if self.is_ja { "🔄 [もう一度クリップボードに保存] を選択しました。" } else { "🔄 Retrying clipboard copy." };
                        println!("{}", console::style(retry_msg).yellow());
                        continue;
                    }
                }
            },
            _ => Ok(None)
        }
    }
}
