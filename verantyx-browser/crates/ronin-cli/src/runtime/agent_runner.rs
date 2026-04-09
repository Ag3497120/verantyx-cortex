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
        sampling_params::{InferenceRequest, SamplingParams},
        provider::{
            ollama::OllamaProvider, anthropic::AnthropicProvider,
            gemini::GeminiProvider, LlmProvider, LlmMessage,
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

    pub async fn run(&self, mut runner_cfg: RunnerConfig) -> Result<RunResult> {
        let model = runner_cfg.model_override
            .as_deref()
            .unwrap_or(&self.config.agent.primary_model)
            .to_string();

        let hitl = runner_cfg.hitl_override
            .unwrap_or(self.config.agent.hitl_enabled);

        let max_steps = runner_cfg.max_steps
            .unwrap_or(self.config.agent.max_steps);

        let display_task: String = runner_cfg.task.chars().take(80).collect();
        info!("[Runner] Task: {}", display_task);
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

        // Capture Start Time to exclude pre-existing unstaged git changes from audit
        let run_start_time = std::time::SystemTime::now();

        // 4. Pre-Flight Intent Router (Local SLM Analysis)
        let provider = self.build_provider(&model);
        println!("\n{} Analyzing Objective Intent...", style("🧠").magenta().bold());

        let lang_desc = match self.config.agent.system_language {
            ronin_core::domain::config::SystemLanguage::Japanese => "プロンプト言語は必ず「日本語」で出力してください。",
            ronin_core::domain::config::SystemLanguage::English => "Ensure the generated prompts are written strictly in English.",
        };

        let meta_prompt = format!("
You are the Ronin Intent Router. 
User Prompt: {}
Decompose the context and purpose. If the prompt asks to 'analyze', 'look into', 'investigate' or implies scanning the project, generate a dedicated system prompt for the Local SLM to analyze the file hierarchy (PureThrough mode), AND a contextual framework for the Observer AI (Gemini) that will run afterwards. NOTE: Gemini is ONLY an observer, it DOES NOT run commands. DO NOT generate git, shell, or execution commands for Gemini!
{}
Output ONLY valid JSON matching this schema:
{{
    \"needs_mapping\": true,
    \"target_directory\": \"/path/to/extracted/absolute/directory/if/present/in/prompt (optional)\",
    \"local_analysis_prompt\": \"Prompt telling local SLM how to summarize the repository tree\",
    \"gemini_directive_prompt\": \"Context instructions for Gemini to observe the objective. Do NOT include shell commands.\"
}}
If no mapping is needed, set needs_mapping to false.
", runner_cfg.task, lang_desc);

        let req = InferenceRequest {
            model: model.clone(),
            format: ronin_core::models::sampling_params::PromptFormat::OllamaChat,
            stream: false,
            sampling: SamplingParams::for_midweight().with_max_tokens(1500).with_temperature(0.2),
        };
        let history = vec![
            LlmMessage { role: "system".to_string(), content: "You return only JSON.".to_string() },
            LlmMessage { role: "user".to_string(), content: meta_prompt }
        ];
        
        let mut final_objective = runner_cfg.task.clone();

        if let Ok(json_res) = provider.invoke(&req, &history).await {
            // Primitive JS-style JSON stripping
            let clean_json = json_res.replace("```json", "").replace("```", "");
            #[derive(serde::Deserialize)]
            struct IntentRoute {
                needs_mapping: bool,
                target_directory: Option<String>,
                local_analysis_prompt: Option<String>,
                gemini_directive_prompt: Option<String>,
            }

            if let Ok(route) = serde_json::from_str::<IntentRoute>(&clean_json) {
                // Dynamically intercept path shifts if the prompt asked to analyze a specific absolute path
                if let Some(mut target_dir) = route.target_directory {
                    target_dir = target_dir.trim().to_string();
                    let p = std::path::Path::new(&target_dir);
                    if p.is_absolute() && p.exists() {
                        runner_cfg.cwd = p.to_path_buf();
                        println!("{} Redirecting context to: {}", console::style("[SYSTEM]").dim(), target_dir);
                    }
                }
                if route.needs_mapping {
                    println!("{} Intent [MAP_AND_EXECUTE] Detected.", style("[SYSTEM]").dim());
                    
                    let mut repo_map = "No Map".to_string();
                    if let Ok(generator) = ronin_repomap::RepoMapGenerator::new(&runner_cfg.cwd).generate() {
                        repo_map = generator.render();
                    }
                    
                    let analysis_prompt = route.local_analysis_prompt.unwrap_or_else(|| "Summarize this repo.".to_string());
                    println!("{} Executing PureThrough Analysis...", style("[SLM]").cyan());
                    
                    let pt_req = InferenceRequest {
                        model: model.clone(),
                        format: ronin_core::models::sampling_params::PromptFormat::OllamaChat,
                        stream: false,
                        sampling: SamplingParams::for_midweight().with_max_tokens(2000).with_temperature(0.2),
                    };
                    let pt_hist = vec![
                        LlmMessage { role: "system".to_string(), content: "You are the PureThrough spatial analyzer. Output a Markdown explanation of the repository structure.".to_string() },
                        LlmMessage { role: "user".to_string(), content: format!("{}\n\nTree:\n{}", analysis_prompt, repo_map) }
                    ];
                    
                    if let Ok(pt_res) = provider.invoke(&pt_req, &pt_hist).await {
                        let memory_dir = runner_cfg.cwd.join(".ronin/memory/front");
                        let _ = tokio::fs::create_dir_all(&memory_dir).await;
                        let out_file = memory_dir.join("purethrough_map.md");
                        let pt_content = format!("# PureThrough Spatial Map\n\n{}\n\n## Auto-Generated AST Map\n```\n{}\n```", pt_res, repo_map);
                        let _ = tokio::fs::write(&out_file, pt_content).await;
                        println!("{} Spatial Map anchored into Memory.", style("[SYSTEM]").green().bold());

                        let pt_distilled = format!("# ローカルLLMのリポジトリ分析結果\n\n{}", pt_res);
                        // Automatically inject the distilled map into Gemini's objective
                        final_objective = format!("{}\n\n[SYSTEM REPOSITORY MAP]\n```\n{}\n```", route.gemini_directive_prompt.unwrap_or(final_objective), pt_distilled);
                    } else {
                        final_objective = route.gemini_directive_prompt.unwrap_or(final_objective);
                    }
                } else {
                    println!("{} Intent [EXECUTE_DIRECTLY] Detected.", style("[SYSTEM]").dim());
                    final_objective = route.gemini_directive_prompt.unwrap_or(final_objective);
                }
            } else {
                warn!("[Router] Failed to parse SLM JSON: {}", clean_json);
            }
        } else {
            warn!("[Router] LLM execution failed.");
        }

        // 5. Initialize Multi-Agent Hive Network
        info!("[Runner] Booting Ronin Multi-Agent Hive System...");
        let spinner = Self::make_spinner("[SYSTEM] Synchronizing Autonomous Hive Network...");
        
        let mut commander_actor = ronin_hive::roles::commander::CommanderActor;
        let mut planner_actor = ronin_hive::roles::planner::PlannerActor::new(&self.config.memory.root_dir);
        let mut coder_actor = ronin_hive::roles::coder::CoderActor::new(&runner_cfg.cwd);
        let mut reviewer_actor = ronin_hive::roles::reviewer::ReviewerActor::new(&runner_cfg.cwd);
        let consensus_actor = ronin_hive::roles::consensus::LocalConsensusActor::new(
            self.config.providers.ollama.host.clone(),
            self.config.providers.ollama.port,
            self.config.agent.primary_model.clone()
        );

        let mut worker_actor = ronin_hive::roles::stealth_gemini::StealthWebActor::new(
            uuid::Uuid::new_v4(),
            self.config.sandbox.allow_filesystem_escape,
            runner_cfg.cwd.clone(),
            self.config.agent.primary_model.clone(),
            self.config.providers.ollama.host.clone(),
            self.config.providers.ollama.port,
            self.config.agent.system_language == ronin_core::domain::config::SystemLanguage::Japanese,
            ronin_hive::roles::stealth_gemini::SystemRole::ArchitectWorker,
            1,
        );

        let mut senior_actor = ronin_hive::roles::stealth_gemini::StealthWebActor::new(
            uuid::Uuid::new_v4(),
            self.config.sandbox.allow_filesystem_escape,
            runner_cfg.cwd.clone(),
            self.config.agent.primary_model.clone(),
            self.config.providers.ollama.host.clone(),
            self.config.providers.ollama.port,
            self.config.agent.system_language == ronin_core::domain::config::SystemLanguage::Japanese,
            ronin_hive::roles::stealth_gemini::SystemRole::SeniorObserver,
            2,
        );

        let mut junior_actor: Option<ronin_hive::roles::stealth_gemini::StealthWebActor> = Some(ronin_hive::roles::stealth_gemini::StealthWebActor::new(
            uuid::Uuid::new_v4(),
            self.config.sandbox.allow_filesystem_escape,
            runner_cfg.cwd.clone(),
            self.config.agent.primary_model.clone(),
            self.config.providers.ollama.host.clone(),
            self.config.providers.ollama.port,
            self.config.agent.system_language == ronin_core::domain::config::SystemLanguage::Japanese,
            ronin_hive::roles::stealth_gemini::SystemRole::JuniorObserver,
            3,
        ));

        if runner_cfg.force_stealth {
            final_objective = format!("[STEALTH_FORCE] {}", final_objective);
        }

        let extract_output = |opt_env: Option<ronin_hive::actor::Envelope>| -> String {
            if let Some(env) = opt_env {
                if let Ok(ronin_hive::messages::HiveMessage::SubAgentResult { output, .. }) = serde_json::from_str(&env.payload) {
                    return output;
                }
            }
            String::new()
        };

        // 5. Run the Triple-Helix Swarm Network
        info!("[Runner] Injecting Objective into Triple-Helix Swarm...");
        let mut final_response = String::new();
        let mut step_count = 0;
        let mut current_objective = final_objective;
        let mut next_tab_index = 3; // Junior will spawn on tab 3

        loop {
            step_count += 1;
            
            // Check Sliding Window Expiration
            if worker_actor.current_turns >= worker_actor.turn_limit {
                info!("[Runner] Memory limit reached. Handing over Swarm tokens and purging old memory context...");
                
                // Junior promotes to Senior
                if let Some(mut j) = junior_actor.take() {
                    j.role = ronin_hive::roles::stealth_gemini::SystemRole::SeniorObserver;
                    senior_actor = j;
                }
                
                // Spawn new Junior
                junior_actor = Some(ronin_hive::roles::stealth_gemini::StealthWebActor::new(
                    uuid::Uuid::new_v4(),
                    self.config.sandbox.allow_filesystem_escape,
                    runner_cfg.cwd.clone(),
                    self.config.agent.primary_model.clone(),
                    self.config.providers.ollama.host.clone(),
                    self.config.providers.ollama.port,
                    self.config.agent.system_language == ronin_core::domain::config::SystemLanguage::Japanese,
                    ronin_hive::roles::stealth_gemini::SystemRole::JuniorObserver,
                    next_tab_index,
                ));
                next_tab_index = if next_tab_index == 2 { 3 } else { 2 };
            }

            // Stop spinner permanently before entering concurrent TUI interaction steps
            spinner.finish_and_clear();

            // Run Worker first
            let env_worker = ronin_hive::actor::Envelope {
                message_id: uuid::Uuid::new_v4(),
                sender: "System".to_string(),
                recipient: "ArchitectWorker".to_string(),
                payload: serde_json::to_string(&ronin_hive::messages::HiveMessage::Objective(current_objective.clone()))?,
            };
            
            info!("[Runner] Executing Worker Actor...");
            let res_worker = worker_actor.receive(env_worker).await;
            let out_w = extract_output(res_worker?);

            if out_w.contains("[TASK_COMPLETE]") || out_w.contains("[FINAL_ANSWER]") {
                info!("[Runner] Task Complete Signal Received from Worker.");
                final_response = format!("Final Answer:\n{}", out_w);
                break;
            }

            // Observers analyze the worker's execution path and logs
            let env_senior = ronin_hive::actor::Envelope {
                message_id: uuid::Uuid::new_v4(),
                sender: "System".to_string(),
                recipient: "SeniorGemini".to_string(),
                payload: serde_json::to_string(&ronin_hive::messages::HiveMessage::Objective(out_w.clone()))?,
            };

            let env_junior = ronin_hive::actor::Envelope {
                message_id: uuid::Uuid::new_v4(),
                sender: "System".to_string(),
                recipient: "JuniorGemini".to_string(),
                payload: serde_json::to_string(&ronin_hive::messages::HiveMessage::Objective(out_w.clone()))?,
            };

            // Run observers sequentially to avoid CLI Mutex deadlocks during human-in-the-loop interactions
            info!("[Runner] Executing Senior Observer...");
            let res_senior = senior_actor.receive(env_senior).await;
            let out_s = extract_output(res_senior?);
            
            let out_j = if let Some(ref mut j) = junior_actor {
                info!("[Runner] Executing Junior Observer...");
                let res_j = j.receive(env_junior).await;
                extract_output(res_j?)
            } else {
                String::new()
            };

            // Fan In: Merge Consensus
            if junior_actor.is_some() {
                info!("[Runner] Consolidating Dual-Observer Reports...");
                let merged = consensus_actor.merge_observations(&out_s, &out_j).await;
                
                // Overwrite the timeline file with pure chronological reality
                let timeline_path = runner_cfg.cwd.join(".ronin").join("timeline.md");
                let mut current_timeline = String::new();
                if timeline_path.exists() {
                    current_timeline = tokio::fs::read_to_string(&timeline_path).await.unwrap_or_default();
                }
                current_timeline.push_str(&format!("\n\n-- TURN {} --\n{}", step_count, merged));
                let _ = tokio::fs::write(&timeline_path, current_timeline).await;
                
                // Assign new unified reality as the objective context for next worker turn
                current_objective = format!("Local System Unified Context:\n{}", merged);
            }
        }
        
        if final_response.is_empty() {
            final_response = "Execution completed asynchronously with no final callback to User_CLI.".to_string();
        }

        spinner.finish_and_clear();
        let steps = step_count;
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
                    // Check if file was actually modified DURING this run
                    if let Ok(meta) = std::fs::metadata(&path) {
                        if let Ok(mod_time) = meta.modified() {
                            if mod_time < run_start_time {
                                continue;
                            }
                        }
                    }

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
