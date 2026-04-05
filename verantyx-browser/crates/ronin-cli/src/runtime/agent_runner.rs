//! Agent runner — the central integration bridge that ties all Ronin crates together.
//!
//! This is where `ronin-core`, `ronin-sandbox`, `ronin-diff-ux`, and `ronin-synapse`
//! converge into a single, coherent execution pipeline. The AgentRunner manages
//! the full lifecycle of a task: model selection → prompt construction → ReAct loop
//! → tool dispatch → HITL approval → observation → loop.

use anyhow::Result;
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use ronin_core::{
    domain::config::RoninConfig,
    engine::{
        prompt_builder::{PromptBuilder, ToolSchema},
        reactor::RoninReactor,
        tool_dispatcher::{ToolDispatcher, ToolResult},
    },
    memory_bridge::{
        context_injector::{ContextInjector, InjectorConfig},
        spatial_index::SpatialIndex,
    },
    models::{
        context_budget::ContextBudget,
        provider::{
            ollama::OllamaProvider, anthropic::AnthropicProvider,
            gemini::GeminiProvider, LlmProvider,
        },
        tier_calibration::TierProfile,
    },
};
use ronin_sandbox::{
    isolation::policy::SandboxPolicy,
    process::session::SandboxSession,
};
use ronin_hive::actor::Actor;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

// ─────────────────────────────────────────────────────────────────────────────
// Runner Configuration
// ─────────────────────────────────────────────────────────────────────────────

pub struct RunnerConfig {
    pub task: String,
    pub model_override: Option<String>,
    pub hitl_override: Option<bool>,
    pub force_stealth: bool,
    pub cwd: PathBuf,
    pub max_steps: Option<u32>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Run Result
// ─────────────────────────────────────────────────────────────────────────────

pub struct RunResult {
    pub task: String,
    pub final_response: String,
    pub steps_taken: u32,
    pub commands_executed: usize,
    pub files_modified: Vec<PathBuf>,
    pub success: bool,
}

// ─────────────────────────────────────────────────────────────────────────────
// Agent Runner
// ─────────────────────────────────────────────────────────────────────────────

pub struct AgentRunner {
    config: RoninConfig,
}

impl AgentRunner {
    pub fn new(config: RoninConfig) -> Self {
        Self { config }
    }

    pub async fn run(&self, runner_cfg: RunnerConfig) -> Result<RunResult> {
        let model = runner_cfg.model_override
            .as_deref()
            .unwrap_or(&self.config.agent.primary_model)
            .to_string();

        let hitl = runner_cfg.hitl_override
            .unwrap_or(self.config.agent.hitl_enabled);

        let max_steps = runner_cfg.max_steps
            .unwrap_or(self.config.agent.max_steps);

        info!("[Runner] Task: {}", &runner_cfg.task[..runner_cfg.task.len().min(80)]);
        info!("[Runner] Model: {} | HITL: {} | MaxSteps: {}", model, hitl, max_steps);

        // 1. Derive tier profile from model name
        let profile = TierProfile::extrapolate_from_model(&model);
        let budget = match profile.max_context_tokens {
            n if n <= 8192  => ContextBudget::for_8b(),
            n if n <= 32768 => ContextBudget::for_27b(),
            _               => ContextBudget::for_70b_plus(),
        };

        // 2. Setup autonomous Git workspace overlay
        if let Ok(git) = ronin_git::GitEngine::new(&runner_cfg.cwd) {
            let task_id = uuid::Uuid::new_v4().as_simple().to_string()[..8].to_string();
            let branch_name = format!("ronin/task-{}", task_id);
            let _ = git.checkout_branch(&branch_name);
            info!("[Runner] Switched to autonomous branch: {}", branch_name);
        }

        // 3. Hydrate JCross memory
        let mut spatial_index = SpatialIndex::new(self.config.memory.root_dir.clone());
        let hydrated = spatial_index.hydrate().await.unwrap_or(0);
        info!("[Runner] Memory: hydrated {} nodes", hydrated);

        let injector_cfg = InjectorConfig::from_budget(&budget);
        let injector = ContextInjector::new(&spatial_index, injector_cfg);
        let memory_block = injector.build_injection_block();

        // 3. Build Repo Map
        info!("[Runner] Generating Repo AST Map...");
        let repo_map = ronin_repomap::RepoMapGenerator::new(&runner_cfg.cwd)
            .generate()
            .map(|m| m.render())
            .unwrap_or_else(|_| String::new());

        // 4. Initialize Multi-Agent Hive Network (Commander, Planner, Coder, Reviewer)
        info!("[Runner] Booting Ronin Multi-Agent Hive System...");
        let spinner = Self::make_spinner("Coordinating Agent Hive Network…");
        
        let mut commander_actor = ronin_hive::roles::commander::CommanderActor;

        let mut task_objective = runner_cfg.task.clone();
        if runner_cfg.force_stealth {
            task_objective = format!("[STEALTH_FORCE] {}", task_objective);
        }

        let task_envelope = ronin_hive::actor::Envelope {
            message_id: uuid::Uuid::new_v4(),
            sender: "User_CLI".to_string(),
            recipient: "Commander".to_string(),
            payload: serde_json::to_string(&ronin_hive::messages::HiveMessage::Objective(task_objective))?,
        };

        // 5. Dispatch task directly into CommanderActor
        // In a real actor loop this would be a message bus. We simulate synchronous E2E call here.
        info!("[Runner] Injecting Objective into Commander Actor...");
        let commander_reply = commander_actor.receive(task_envelope).await?;
        
        let final_response = match commander_reply {
            Some(env) => {
                // If it spawned a sub-agent (StealthGemini) or delegated, parse payload
                if let Ok(ronin_hive::messages::HiveMessage::SpawnSubAgent { id: _, objective }) = serde_json::from_str(&env.payload) {
                    format!("Commander offloaded task to SubAgent/Hive pipeline: {}", objective)
                } else {
                    format!("Commander yielded response: {}", env.payload)
                }
            },
            None => {
                "Commander processed task without an explicit return message (e.g. delegated purely into background async queue).".to_string()
            }
        };

        spinner.finish_and_clear();
        let steps = 1; // Simulated for now since Actor Network does internal steps
        let commands_executed = 0; // Handled by ReviewerActor internally

        let mut files_modified_final = vec![];

        // 6. Output Validation & Diff Approval (HITL)
        let inspector = ronin_diff_ux::git::inspector::GitInspector::detect(&runner_cfg.cwd);
        if inspector.is_git_repo() {
            let modified = inspector.modified_files();
            if !modified.is_empty() {
                println!("\n{} Post-Run Audit: Reviewing Diffs...", console::style("⚡").cyan().bold());
                let engine = ronin_diff_ux::diff::engine::DiffEngine::new(ronin_diff_ux::diff::engine::DiffGranularity::Line);
                let mut prompt = ronin_diff_ux::tui::approval_prompt::ApprovalSession::new();

                for path in modified {
                    let relative = path.strip_prefix(&runner_cfg.cwd).unwrap_or(&path);
                    if let Ok(out) = std::process::Command::new("git")
                        .args(["show", &format!("HEAD:{}", relative.display())])
                        .current_dir(&runner_cfg.cwd)
                        .output() 
                    {
                        let old_text = String::from_utf8_lossy(&out.stdout).to_string();
                        let new_text = std::fs::read_to_string(&path).unwrap_or_default();
                        
                        let diff_result = engine.compute(&path.to_string_lossy(), &old_text, &new_text);
                        if diff_result.has_changes() {
                            let decision = prompt.prompt(&diff_result);
                            
                            if decision == ronin_diff_ux::tui::approval_prompt::ApprovalDecision::Reject 
                               || decision == ronin_diff_ux::tui::approval_prompt::ApprovalDecision::RejectAll {
                                // Revert specific file!
                                let _ = std::process::Command::new("git")
                                    .args(["checkout", "HEAD", "--", &relative.to_string_lossy()])
                                    .current_dir(&runner_cfg.cwd)
                                    .output();
                                println!("{} Reverted {}", console::style("🚫").red(), relative.display());
                            } else {
                                files_modified_final.push(std::path::PathBuf::from(relative));
                            }
                        }
                    }
                }
            }
        }

        if !files_modified_final.is_empty() {
            if let Ok(git) = ronin_git::GitEngine::new(&runner_cfg.cwd) {
                let _ = git.commit_all("Ronin: Auto Patch Apply", "Ronin Agent", "ronin@verantyx.com");
            }
        }

        Ok(RunResult {
            task: runner_cfg.task,
            final_response,
            steps_taken: steps,
            commands_executed: 0,
            files_modified: files_modified_final,
            success: true,
        })
    }

    fn build_provider(&self, model: &str) -> Box<dyn LlmProvider> {
        // Cloud fallback routing
        if model.starts_with("claude") {
            if let Some(cred) = &self.config.providers.anthropic {
                return Box::new(AnthropicProvider::new(&cred.api_key));
            }
        }
        if model.starts_with("gemini") {
            if let Some(cred) = &self.config.providers.gemini {
                return Box::new(GeminiProvider::new(&cred.api_key));
            }
        }
        // Default: local Ollama
        Box::new(OllamaProvider::new(
            &self.config.providers.ollama.host,
            self.config.providers.ollama.port,
        ))
    }

    fn default_tool_schemas() -> Vec<ToolSchema> {
        vec![
            ToolSchema {
                name: "shell_exec".to_string(),
                description: "Run a bash command in the sandboxed working directory".to_string(),
                parameters: vec![
                    ronin_core::engine::prompt_builder::ToolParameter {
                        name: "command".to_string(),
                        required: true,
                        description: "The bash command to execute".to_string(),
                    },
                ],
            },
            ToolSchema {
                name: "read_file".to_string(),
                description: "Read the contents of a file".to_string(),
                parameters: vec![
                    ronin_core::engine::prompt_builder::ToolParameter {
                        name: "path".to_string(),
                        required: true,
                        description: "Relative or absolute path to the file".to_string(),
                    },
                ],
            },
            ToolSchema {
                name: "write_file".to_string(),
                description: "Write or overwrite a file with new contents (triggers HITL approval)".to_string(),
                parameters: vec![
                    ronin_core::engine::prompt_builder::ToolParameter {
                        name: "path".to_string(),
                        required: true,
                        description: "Path to write to".to_string(),
                    },
                    ronin_core::engine::prompt_builder::ToolParameter {
                        name: "content".to_string(),
                        required: true,
                        description: "Full file content to write".to_string(),
                    },
                ],
            },
            ToolSchema {
                name: "replace_block".to_string(),
                description: "SAFELY edit an existing file using Aider Search/Replace Block protocol. You must match the 'search' block precisely. ALWAYS prefer this over shell_exec + sed.".to_string(),
                parameters: vec![
                    ronin_core::engine::prompt_builder::ToolParameter {
                        name: "path".to_string(),
                        required: true,
                        description: "Relative path to target file".to_string(),
                    },
                    ronin_core::engine::prompt_builder::ToolParameter {
                        name: "search".to_string(),
                        required: true,
                        description: "EXACT code chunk to be replaced (include leading indents)".to_string(),
                    },
                    ronin_core::engine::prompt_builder::ToolParameter {
                        name: "replace".to_string(),
                        required: true,
                        description: "New code chunk to insert".to_string(),
                    },
                ],
            },
            ToolSchema {
                name: "finish".to_string(),
                description: "Signal that the task is complete. Include a summary of what was done.".to_string(),
                parameters: vec![],
            },
            ToolSchema {
                name: "ask_gemini_browser".to_string(),
                description: "Ask Gemini via a private browser when you lack knowledge on a topic. VERY SLOW, use only as last resort.".to_string(),
                parameters: vec![
                    ronin_core::engine::prompt_builder::ToolParameter {
                        name: "question".to_string(),
                        required: true,
                        description: "The specific question or task to ask Gemini".to_string(),
                    },
                ],
            },
        ]
    }

    fn make_spinner(message: &str) -> ProgressBar {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::with_template("{spinner:.cyan.bold} {msg}")
                .unwrap()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
        );
        pb.set_message(message.to_string());
        pb.enable_steady_tick(std::time::Duration::from_millis(80));
        pb
    }
}
