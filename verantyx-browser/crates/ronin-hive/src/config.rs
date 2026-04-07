use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::info;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PersonaConfig {
    pub name: String,
    pub personality: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SchedulerConfig {
    pub night_watch_hour: i32, // -1 means disabled, 0-23 represents the hour
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VerantyxConfig {
    pub persona: PersonaConfig,
    pub scheduler: SchedulerConfig,
}

impl Default for VerantyxConfig {
    fn default() -> Self {
        Self {
            persona: PersonaConfig {
                name: "Verantyx Alpha".to_string(),
                personality: "冷静沈着でプロフェッショナルなハッカー・アナリスト".to_string(),
            },
            scheduler: SchedulerConfig {
                night_watch_hour: 3,
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
        if config_path.exists() {
            return Self::load(cwd);
        }

        println!("\n{}", console::style("╭─ [ ✨ Verantyx Engine Initial Setup ] ────────────────────────────────").cyan().bold());
        println!("{} OpenClawアーキテクチャに基づくAI人格およびスケジューラの初期設定を開始します。", console::style("│").cyan().bold());
        println!("{}", console::style("╰──────────────────────────────────────────────────────────────────────").cyan().bold());

        let name: String = dialoguer::Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt("AIの名前 (例: Verantyx Alpha, 助手AI)")
            .default("Verantyx Alpha".to_string())
            .interact_text()
            .unwrap();

        let personality: String = dialoguer::Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt("AIの人格・性格設定 (例: 冷静沈着, 厳格なプログラマー, フレンドリーに敬語で)")
            .default("冷静沈着でプロフェッショナルなハッカー・アナリスト".to_string())
            .interact_text()
            .unwrap();

        println!("\n{}", console::style("--- [ 🌙 Night Watch (自律深夜検証・退行テスト) ] ---").magenta());
        println!("毎日指定した時間帯に、バックグラウンドデーモンが過去の記憶(experience)を元に勝手にAIを起動し、Webサイト等のレイアウトが変わって突破できなくなっていないかを自律検証します。");
        let hour_str: String = dialoguer::Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt("自動実行を開始する時間帯 (0〜23の数字。無効にする場合は -1)")
            .default("3".to_string())
            .interact_text()
            .unwrap();

        let night_watch_hour: i32 = hour_str.parse().unwrap_or(3);

        let config = Self {
            persona: PersonaConfig { name, personality },
            scheduler: SchedulerConfig { night_watch_hour },
        };

        if let Err(e) = config.save(cwd) {
            tracing::error!("Failed to save configuration: {}", e);
        } else {
            println!("\n{} 初期設定が完了しました！ (Saved to .ronin/agent_config.json)\n", console::style("[AI_SYS]").green());
        }

        config
    }
}
