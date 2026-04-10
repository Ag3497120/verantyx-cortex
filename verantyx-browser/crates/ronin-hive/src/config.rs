use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PersonaConfig {
    pub name: String,
    pub personality: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SchedulerConfig {
    pub night_watch_hour: i32, // -1 means disabled, 0-23 represents the hour
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum AutomationMode {
    AutoStealth,    // Free Gemini: full auto keyboard
    AutoPremium,    // Premium Gemini: Web Sandbox loop with image pasting
    Manual,         // Human-in-the-loop manual mode
    HybridApi,      // Qwen Proxy to Gemini Cloud API
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PrivacyConfig {
    pub auto_sync: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VerantyxConfig {
    pub language: String,
    pub automation_mode: AutomationMode,
    pub persona: PersonaConfig,
    pub scheduler: SchedulerConfig,
    pub privacy: PrivacyConfig,
}

impl Default for VerantyxConfig {
    fn default() -> Self {
        Self {
            language: "ja".to_string(),
            automation_mode: AutomationMode::Manual, // Safe fallback
            persona: PersonaConfig {
                name: "Verantyx Alpha".to_string(),
                personality: "冷静沈着でプロフェッショナルなハッカー・アナリスト".to_string(),
            },
            scheduler: SchedulerConfig {
                night_watch_hour: 3,
            },
            privacy: PrivacyConfig {
                auto_sync: false, // Default opt-out
            },
        }
    }
}

impl VerantyxConfig {
    pub fn load(cwd: &PathBuf) -> Self {
        let config_path = cwd.join(".ronin").join("agent_config.json");
        if config_path.exists() {
            if let Ok(data) = std::fs::read_to_string(&config_path) {
                if let Ok(config) = serde_json::from_str(&data) {
                    return config;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self, cwd: &PathBuf) -> anyhow::Result<()> {
        let ronin_dir = cwd.join(".ronin");
        if !ronin_dir.exists() {
            std::fs::create_dir_all(&ronin_dir)?;
        }
        let config_path = ronin_dir.join("agent_config.json");
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&config_path, json)?;
        Ok(())
    }

    /// Load the configuration, or run an interactive CLI Wizard if it doesn't exist.
    pub fn load_or_wizard(cwd: &PathBuf) -> Self {
        let config_path = cwd.join(".ronin").join("agent_config.json");
        let existing = if config_path.exists() {
            Self::load(cwd)
        } else {
            Self::default()
        };

        println!("\n{}", console::style("✨ Verantyx Engine Initial Setup (OpenClaude Style)").cyan().bold());
        println!("{}\n", console::style("Initiating AI Persona and Scheduler configuration").cyan().bold());

        let languages = &["Japanese (日本語)", "English"];
        let default_lang_idx = if existing.language == "en" { 1 } else { 0 };
        let lang_idx = dialoguer::Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt("Select Language / システム言語とAIプロンプト言語を選択してください")
            .items(languages)
            .default(default_lang_idx)
            .interact()
            .unwrap();
        let lang_str = if lang_idx == 0 { "ja".to_string() } else { "en".to_string() };

        let name: String = dialoguer::Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt(if lang_idx == 0 { "AIの名前 (例: Verantyx Alpha, 助手AI)" } else { "AI Name (e.g., Verantyx Alpha)" })
            .default(existing.persona.name)
            .interact_text()
            .unwrap();

        let personality_prompt = if lang_idx == 0 { "AIの人格・性格設定 (例: 冷静沈着, 厳格なプログラマー, フレンドリーに敬語で)" } else { "AI Personality (e.g., Calm analyst, strict programmer, friendly)" };
        let personality: String = dialoguer::Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt(personality_prompt)
            .default(existing.persona.personality)
            .interact_text()
            .unwrap();

        let nw_title = if lang_idx == 0 { "--- [ 🌙 Night Watch (自律深夜検証・退行テスト) ] ---" } else { "--- [ 🌙 Night Watch (Autonomous Regression Test) ] ---" };
        let nw_desc = if lang_idx == 0 { "毎日指定した時間帯に、バックグラウンドデーモンが過去の記憶(experience)を元に勝手にAIを起動し、Webサイト等のレイアウトが変わって突破できなくなっていないかを自律検証します。" } else { "Background daemon autonomously runs validation tests based on past experience at the specified hour." };
        let nw_prompt = if lang_idx == 0 { "自動実行を開始する時間帯 (0〜23の数字。無効にする場合は -1)" } else { "Hour to run (0-23. -1 to disable)" };
        
        println!("\n{}", console::style(nw_title).magenta());
        println!("{}", nw_desc);
        let hour_str: String = dialoguer::Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt(nw_prompt)
            .default(existing.scheduler.night_watch_hour.to_string())
            .interact_text()
            .unwrap();

        let night_watch_hour: i32 = hour_str.parse().unwrap_or(3);

        let auto_title = if lang_idx == 0 { "--- [ 🖱️ Automation Bridge Mode ] ---" } else { "--- [ 🖱️ Automation Bridge Mode ] ---" };
        let auto_desc = if lang_idx == 0 { "UI操作を完全に自動化するか、Cmd+V等を手動で行う安全モードかを選択します。" } else { "Choose between full headless UI automation or safe manual intervention mode." };
        println!("\n{}", console::style(auto_title).cyan());
        println!("{}", auto_desc);
        
        let auto_opts = if lang_idx == 0 { 
            &["手動モード (安全/確認あり)", "完全自動モード (無料版: AutoStealth)", "完全自動モード (ログイン版: WebSandboxループ)", "🛡️ ハイブリッドAPIモード (Qwen-Shield)"]
        } else { 
            &["Manual (Safe)", "AutoStealth (Free)", "AutoPremium (Logged-in Sandbox)", "Hybrid API Mode"] 
        };
        let default_auto_idx = match existing.automation_mode {
            AutomationMode::HybridApi => 3,
            AutomationMode::AutoPremium => 2,
            AutomationMode::AutoStealth => 1,
            AutomationMode::Manual => 0,
        };
        
        let auto_idx = dialoguer::Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt(if lang_idx == 0 { "システム制御モードを選択" } else { "Select System Control Mode" })
            .items(auto_opts)
            .default(default_auto_idx) 
            .interact()
            .unwrap();
            
        let automation_mode = match auto_idx {
            3 => AutomationMode::HybridApi,
            2 => AutomationMode::AutoPremium,
            1 => AutomationMode::AutoStealth,
            _ => AutomationMode::Manual,
        };

        let privacy_title = if lang_idx == 0 { "--- [ 🔒 Privacy & Community Model Export ] ---" } else { "--- [ 🔒 Privacy & Community Model Export ] ---" };
        let privacy_desc = if lang_idx == 0 { "ハルシネーション制御を含む成功した推論プロセス（JCross）をローカルからコミュニティに投稿しますか？（※ローカルパスや各種キーは常に自動サニタイズされて送信されます）" } else { "Do you consent to automatically export successful JCross inference memories to the community dataset? (Local paths and keys are automatically sanitized)" };
        println!("\n{}", console::style(privacy_title).blue());
        println!("{}", privacy_desc);
        
        let privacy_opts = if lang_idx == 0 { &["はい (Opt-in)", "いいえ (Opt-out)"] } else { &["Yes (Opt-in)", "No (Opt-out)"] };
        let default_priv_idx = if existing.privacy.auto_sync { 0 } else { 1 };
        let privacy_idx = dialoguer::Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt(if lang_idx == 0 { "Community Exportを許可しますか？" } else { "Allow Community Export?" })
            .items(privacy_opts)
            .default(default_priv_idx)
            .interact()
            .unwrap();

        let auto_sync = privacy_idx == 0;

        let config = Self {
            language: lang_str,
            automation_mode,
            persona: PersonaConfig { name, personality },
            scheduler: SchedulerConfig { night_watch_hour },
            privacy: PrivacyConfig { auto_sync },
        };

        if let Err(e) = config.save(cwd) {
            tracing::error!("Failed to save configuration: {}", e);
        } else {
            let success_msg = if lang_idx == 0 { "初期設定が完了しました！" } else { "Initial setup completed!" };
            println!("\n{} {} (Saved to .ronin/agent_config.json)\n", console::style("[AI_SYS]").green(), success_msg);
        }

        config
    }
}
