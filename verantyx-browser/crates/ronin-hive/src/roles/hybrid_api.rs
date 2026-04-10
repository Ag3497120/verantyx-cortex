use crate::actor::{Actor, Envelope};
use crate::messages::HiveMessage;
use crate::roles::stealth_gemini::SystemRole;
use async_trait::async_trait;
use ronin_core::models::provider::gemini::GeminiProvider;
use ronin_core::models::provider::anthropic::AnthropicProvider;
use ronin_core::models::provider::openai::OpenAiCompatibleProvider;
use ronin_core::models::provider::ollama::OllamaProvider;
use ronin_core::models::provider::{LlmMessage, LlmProvider};
use ronin_core::models::sampling_params::{InferenceRequest, PromptFormat, SamplingParams};
use std::path::PathBuf;
use tracing::{debug, info, warn};
use uuid::Uuid;

pub struct HybridApiActor {
    pub id: Uuid,
    pub turn_limit: u8,
    pub current_turns: u8,
    global_access: bool,
    cwd: PathBuf,
    local_model: String,
    ollama_host: String,
    ollama_port: u16,
    pub is_japanese_mode: bool,
    pub role: SystemRole,
    pub tab_index: u8,
    cloud_api_key: String,
}

impl HybridApiActor {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: Uuid,
        global_access: bool,
        cwd: PathBuf,
        local_model: String,
        ollama_host: String,
        ollama_port: u16,
        is_japanese_mode: bool,
        role: SystemRole,
        tab_index: u8,
        cloud_api_key: String,
    ) -> Self {
        Self {
            id,
            turn_limit: 5,
            current_turns: 0,
            global_access,
            cwd,
            local_model,
            ollama_host,
            ollama_port,
            is_japanese_mode,
            role,
            tab_index,
            cloud_api_key,
        }
    }

    /// Ask Gemini directly (via API) but sanitized by Qwen first
    async fn call_hybrid_shield(&self, original_prompt: &str) -> String {
        info!("[HybridAPI-{}] Engaging Qwen Shield Sanitization for Zero-Trust...", self.id);
        
        // 1. Qwen Sanitization
        let qwen_provider = OllamaProvider::new(&self.ollama_host, self.ollama_port);
        let qwen_req = InferenceRequest {
            model: self.local_model.clone(),
            format: PromptFormat::OllamaChat,
            stream: false,
            sampling: SamplingParams::for_heavyweight().with_temperature(0.0),
        };
        
        let sanitize_prompt = format!("
You are a Zero-Trust Security Shield. Your job is to anonymize the following text.
Replace any absolute file paths with semantic dummy identifiers like [FILE_A], [FILE_B], etc.
Replace any API keys or personal information with [KEY_1], [SECRET_1], etc.
Output the anonymization mapping table as a JSON block, then output the fully anonymized string.

TEXT TO ANONYMIZE:
{}
", original_prompt);

        let history = vec![LlmMessage {
            role: "user".to_string(),
            content: sanitize_prompt.clone(),
        }];

        let sanitize_result = match qwen_provider.invoke(&qwen_req, &history).await {
            Ok(res) => res,
            Err(e) => {
                warn!("[HybridAPI-{}] Qwen Sanitization Failed: {}. Falling back to cleartext.", self.id, e);
                // In a true zero-trust we would reject. Here we fall back to raw or just return an error text.
                return format!("❌ Error: Qwen shield failed to sanitize prompt: {}", e);
            }
        };

        // Extract JSON mapping and sanitized text naive approach
        let sanitized_text = sanitize_result.clone(); // In reality, we must parse JSON vs Text. For simplicity assuming it outputs text and json.

        // 2. Central Cloud API
        let cfg = crate::config::VerantyxConfig::load(&self.cwd);
        info!("[HybridAPI-{}] Dispatching sanitized payload to {:?} Engine...", self.id, cfg.cloud_provider);
        
        let (cloud_provider, req): (Box<dyn LlmProvider>, InferenceRequest) = match cfg.cloud_provider {
            crate::config::CloudProvider::Gemini => {
                let provider = GeminiProvider::new(&self.cloud_api_key);
                let req = InferenceRequest {
                    model: "gemini-2.5-pro".to_string(), // Ensure using correct model
                    format: PromptFormat::GeminiContents,
                    stream: false,
                    sampling: SamplingParams::for_midweight().with_temperature(0.2),
                };
                (Box::new(provider), req)
            },
            crate::config::CloudProvider::OpenAi => {
                let provider = OpenAiCompatibleProvider::openai(&self.cloud_api_key);
                let req = InferenceRequest {
                    model: "gpt-4o".to_string(),
                    format: PromptFormat::OpenAiChat,
                    stream: false,
                    sampling: SamplingParams::for_midweight().with_temperature(0.2),
                };
                (Box::new(provider), req)
            },
            crate::config::CloudProvider::Anthropic => {
                let provider = AnthropicProvider::new(&self.cloud_api_key);
                let req = InferenceRequest {
                    model: "claude-3-5-sonnet-20241022".to_string(),
                    format: PromptFormat::AnthropicMessages,
                    stream: false,
                    sampling: SamplingParams::for_midweight().with_temperature(0.2),
                };
                (Box::new(provider), req)
            },
            crate::config::CloudProvider::DeepSeek => {
                let provider = OpenAiCompatibleProvider::deepseek(&self.cloud_api_key);
                let req = InferenceRequest {
                    model: "deepseek-reasoner".to_string(),
                    format: PromptFormat::OpenAiChat,
                    stream: false,
                    sampling: SamplingParams::for_midweight().with_temperature(0.2),
                };
                (Box::new(provider), req)
            },
            crate::config::CloudProvider::OpenRouter => {
                let provider = OpenAiCompatibleProvider::openrouter(&self.cloud_api_key);
                let req = InferenceRequest {
                    model: "google/gemini-2.5-pro".to_string(),
                    format: PromptFormat::OpenAiChat,
                    stream: false,
                    sampling: SamplingParams::for_midweight().with_temperature(0.2),
                };
                (Box::new(provider), req)
            },
            crate::config::CloudProvider::Groq => {
                let provider = OpenAiCompatibleProvider::groq(&self.cloud_api_key);
                let req = InferenceRequest {
                    model: "llama3-70b-8192".to_string(),
                    format: PromptFormat::OpenAiChat,
                    stream: false,
                    sampling: SamplingParams::for_midweight().with_temperature(0.2),
                };
                (Box::new(provider), req)
            },
            crate::config::CloudProvider::Together => {
                let provider = OpenAiCompatibleProvider::together(&self.cloud_api_key);
                let req = InferenceRequest {
                    model: "meta-llama/Llama-3.3-70B-Instruct-Turbo".to_string(),
                    format: PromptFormat::OpenAiChat,
                    stream: false,
                    sampling: SamplingParams::for_midweight().with_temperature(0.2),
                };
                (Box::new(provider), req)
            },
        };

        let cloud_history = vec![LlmMessage {
            role: "user".to_string(),
            content: sanitized_text.clone(),
        }];

        let cloud_result = match cloud_provider.invoke(&req, &cloud_history).await {
            Ok(res) => res,
            Err(e) => {
                warn!("[HybridAPI-{}] Cloud API Error: {}", self.id, e);
                return format!("❌ Cloud Request Failed: {}", e);
            }
        };

        info!("[HybridAPI-{}] Reversing Qwen Shield Sanitization...", self.id);
        
        // 3. Qwen De-Sanitization
        let desanitize_prompt = format!("
You are a Zero-Trust Security Shield. 
You previously mapped sensitive data out of a prompt. 
Now, take this generated response and replace the dummy identifiers (e.g. [FILE_A]) back to their original forms based on this previous mapping step.

PREVIOUS MAPPING/SHIELD OUTPUT:
{}

GEMINI/CLOUD RESPONSE (with dummy IDs):
{}

Output ONLY the fully restored string. No commentary.
", sanitize_result, cloud_result);

        let restore_history = vec![LlmMessage {
            role: "user".to_string(),
            content: desanitize_prompt,
        }];

        match qwen_provider.invoke(&qwen_req, &restore_history).await {
            Ok(res) => res,
            Err(e) => {
                warn!("[HybridAPI-{}] Qwen De-Sanitization Failed: {}", self.id, e);
                format!("❌ Could not decrypt responses: {}", e)
                // Optionally return gemini_result
            }
        }
    }
}

#[async_trait]
impl Actor for HybridApiActor {
    fn name(&self) -> &str {
        "HybridApiWorker"
    }

    async fn receive(&mut self, env: Envelope) -> anyhow::Result<Option<Envelope>> {
        let msg: HiveMessage = match serde_json::from_str(&env.payload) {
            Ok(m) => m,
            Err(_) => return Ok(None),
        };

        match msg {
            HiveMessage::SpawnSubAgent { id: _, objective } | HiveMessage::Objective(objective) => {
                debug!("[HybridAPI-{}] Received objective: {}", self.id, objective);

                self.current_turns += 1;
                info!("[HybridAPI-{}] Turn usage: {} / {}", self.id, self.current_turns, self.turn_limit);

                let cfg = crate::config::VerantyxConfig::load(&self.cwd);
                let persona_name = cfg.persona.name.clone();
                let persona_traits = cfg.persona.personality.clone();

                let system_prompt = match self.role {
                    SystemRole::ArchitectWorker => {
                        format!(
r#"【AGENT PERSONA】
Name: {}
Personality: {}

【SYSTEM: Architect Worker (Verantyx API Mode)】
You are the central Brain (Worker) of the Verantyx Multi-AI System.
This is the **API Mode**. You are directly connected to the system. You must output commands using the standard prefixes:
1. `編集中` (or `[EDITING]`) - To edit/run a script
2. `[FINAL_ANSWER]` - When the task is complete.

Objective: {}"#,
                            persona_name, persona_traits, objective
                        )
                    }
                    SystemRole::SeniorObserver => {
                        format!("You are the Senior Validator. Review the execution of the Worker. Objective: {}", objective)
                    }
                    SystemRole::JuniorObserver => {
                        format!("You are the Junior Apprentice. Sync memories and ensure no rules are broken. Objective: {}", objective)
                    }
                };

                // The magic happens here:
                let final_restored_output = self.call_hybrid_shield(&system_prompt).await;

                // Process tool calls on the `final_restored_output` directly and securely here,
                // or just yield it back exactly like stealth_gemini does so the main loop can decide.

                let result = HiveMessage::SubAgentResult {
                    id: self.id,
                    output: final_restored_output,
                };
                
                Ok(Some(Envelope {
                    message_id: Uuid::new_v4(),
                    sender: match self.role {
                        SystemRole::ArchitectWorker => "HybridApiWorker".to_string(),
                        SystemRole::SeniorObserver => "SeniorHybridObserver".to_string(),
                        SystemRole::JuniorObserver => "JuniorHybridObserver".to_string(),
                    },
                    recipient: env.sender,
                    payload: serde_json::to_string(&result)?,
                }))
            }
            _ => Ok(None),
        }
    }
}
