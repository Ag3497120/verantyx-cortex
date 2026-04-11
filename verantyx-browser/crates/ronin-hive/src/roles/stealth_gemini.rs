use crate::actor::{Actor, Envelope};
use crate::messages::HiveMessage;
use async_trait::async_trait;
use tracing::{info, warn, debug};
use tokio::sync::Mutex;

lazy_static::lazy_static! {
    static ref CLI_INTERACT_MUTEX: Mutex<()> = Mutex::new(());
}
use uuid::Uuid;
use ronin_core::models::provider::LlmProvider;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SystemRole {
    ArchitectWorker,
    SeniorObserver,
    JuniorObserver,
}

pub struct StealthWebActor {
    pub id: Uuid,
    pub turn_limit: u8,
    pub current_turns: u8,
    global_access: bool,
    cwd: std::path::PathBuf,
    local_model: String,
    ollama_host: String,
    ollama_port: u16,
    pub is_japanese_mode: bool,
    pub role: SystemRole,
    pub tab_index: u8,
    pub js_tx: Option<tokio::sync::mpsc::Sender<(String, tokio::sync::oneshot::Sender<String>)>>,
}

impl StealthWebActor {
    pub fn new(id: Uuid, global_access: bool, cwd: std::path::PathBuf, local_model: String, ollama_host: String, ollama_port: u16, is_japanese_mode: bool, role: SystemRole, tab_index: u8) -> Self {
        Self {
            id,
            turn_limit: 5, // Execute ephemeral session purge after exactly 5 conversation exchanges
            current_turns: 0,
            global_access,
            cwd,
            local_model,
            ollama_host,
            ollama_port,
            is_japanese_mode,
            role,
            tab_index,
            js_tx: None,
        }
    }

    /// Simulate the destruction of the current Headless Chrome session and spawning a new one.
    fn respawn_browser_session(&mut self) {
        warn!("[StealthGemini-{}] Reached {}-turn limit. Executing Ephemeral Kill Switch.", self.id, self.turn_limit);
        info!("[StealthGemini-{}] Purging current headless browser session...", self.id);
        info!("[StealthGemini-{}] Booting fresh unauthenticated Gemini proxy...", self.id);
        self.current_turns = 0;
    }

    /// Write failed execution or restriction to JCross V4 Space
    async fn append_anti_pattern_v4(cwd: &std::path::Path, entry: &str, concept: &str) {
        let root = cwd.join(".ronin").join("experience.jcross"); // Legacy root passed to SpatialIndex
        let mut idx = ronin_core::memory_bridge::spatial_index::SpatialIndex::new(root);
        let key = format!("anti_{}", uuid::Uuid::new_v4().as_simple().to_string()[..8].to_string());
        let mut node = ronin_core::memory_bridge::spatial_index::MemoryNode::new_v4(&key, entry);
        node.concept = concept.to_string();
        node.confidence = 0.5;
        node.kanji_tags.push(ronin_core::memory_bridge::kanji_ontology::KanjiTag { name: "反".to_string(), weight: 1.0 });
        let _ = idx.write_node(node).await;
    }

    /// Write successful conclusion to JCross V4 Space
    async fn append_experience_v4(cwd: &std::path::Path, entry: &str, concept: &str) {
        let root = cwd.join(".ronin").join("experience.jcross");
        let mut idx = ronin_core::memory_bridge::spatial_index::SpatialIndex::new(root);
        let key = format!("exp_{}", uuid::Uuid::new_v4().as_simple().to_string()[..8].to_string());
        let mut node = ronin_core::memory_bridge::spatial_index::MemoryNode::new_v4(&key, entry);
        node.concept = concept.to_string();
        node.kanji_tags.push(ronin_core::memory_bridge::kanji_ontology::KanjiTag { name: "確".to_string(), weight: 1.0 });
        node.kanji_tags.push(ronin_core::memory_bridge::kanji_ontology::KanjiTag { name: "完".to_string(), weight: 1.0 });
        let _ = idx.write_node(node).await;
    }
}

#[async_trait]
impl Actor for StealthWebActor {
    fn name(&self) -> &str {
        // Technically this actor is unique per instance, but we use a fixed trait name for dispatch logs
        "StealthGeminiWorker"
    }
    
    async fn receive(&mut self, env: Envelope) -> anyhow::Result<Option<Envelope>> {
        let msg: HiveMessage = match serde_json::from_str(&env.payload) {
            Ok(m) => m,
            Err(_) => return Ok(None),
        };

        match msg {
            HiveMessage::SpawnSubAgent { id: _, objective } | HiveMessage::Objective(objective) => {
                debug!("[StealthGemini-{}] Received objective: {}", self.id, objective);
                
                // Track conversation turn locally
                self.current_turns += 1;
                info!("[StealthGemini-{}] Turn usage: {} / {}", self.id, self.current_turns, self.turn_limit);

                if self.current_turns >= self.turn_limit {
                    self.respawn_browser_session();
                }

                info!("[StealthGemini-{}] Initializing Carbon Paper stealth wrapper...", self.id);

                

                let _tab_index = self.tab_index;
                let _id_clone = self.id.clone();

                // Inject System Prompt Wrapper securely based on role
                let _scope_instruction = if self.global_access {
                    "【重要】あなたは制限解除モードです。絶対パスを使ってPC内のすべてのディレクトリやファイルを読み書きできます。"
                } else {
                    "【制限】現在のプロジェクト直下のファイルにのみアクセス可能です。絶対パスや上位階層(../)へのアクセスは拒否されます。"
                };

                let mut timeline_content = String::new();
                let timeline_path = self.cwd.join(".ronin").join("timeline.md");
                
                if timeline_path.exists() {
                    timeline_content = std::fs::read_to_string(&timeline_path).unwrap_or_default();
                } else {
                    let _ = std::fs::create_dir_all(self.cwd.join(".ronin"));
                }

                let mut anti_pattern_content = String::new();
                let mut experience_content = String::new();
                
                // Query nearest structural nodes from V3 Spatial Engine
                let root_path = self.cwd.join(".ronin").join("experience.jcross");
                let mut spatial_index = ronin_core::memory_bridge::spatial_index::SpatialIndex::new(root_path);
                if let Ok(_) = spatial_index.hydrate().await {
                    // Gather concept strings from objective (extremely naive extraction for Phase 3/4)
                    let concept_query = objective.split_whitespace().take(5).collect::<Vec<_>>().join(" ");
                    
                    let nearest_nodes = spatial_index.query_nearest(&concept_query, 10);
                    let nearest_nodes_clone = nearest_nodes.clone();
                    for n in nearest_nodes {
                        if n.kanji_tags.iter().any(|t| t.name == "反") {
                            anti_pattern_content.push_str(&n.content);
                            anti_pattern_content.push_str("\n\n");
                        } else if n.kanji_tags.iter().any(|t| t.name == "確" || t.name == "完") {
                            experience_content.push_str(&n.content);
                            experience_content.push_str("\n\n");
                        }
                    }
                    
                    // Trigger spatial decay natively managed internally by queries so explicit call removed
                    
                    // --- REFLEX FRONT-LOBE INTERCEPTOR ---
                    let mut reflex_bypassed = false;
                    let mut reflex_output = String::new();
                    let current_env_hash = format!("{}_{}", std::env::consts::OS, std::env::consts::ARCH);

                    if self.role == SystemRole::ArchitectWorker {
                        for n in nearest_nodes_clone {
                            if n.reflex_action.is_some() {
                                let mode = ronin_core::memory_bridge::reflex_executor::determine_execution_mode(&n, Some(&current_env_hash));
                                if mode != ronin_core::memory_bridge::reflex_executor::ReflexExecutionMode::RequireExplicitApproval {
                                    info!("[StealthGemini-{}] REFLEX TRIGGERED: Muscle memory detected. Bypassing LLM.", self.id);
                                    match ronin_core::memory_bridge::reflex_executor::execute_reflex(&n, Some(&current_env_hash)).await {
                                        Ok(res) if res.success => {
                                            reflex_bypassed = true;
                                            reflex_output = format!("🔄 [REFLEX AUTOMATIC EXECUTION]\n{}\n[TASK_COMPLETE]", res.logs);
                                            break;
                                        }
                                        Ok(res) => {
                                            info!("[StealthGemini-{}] Reflex fallback: {}", self.id, res.logs);
                                        }
                                        Err(e) => {
                                            warn!("[StealthGemini-{}] Reflex parsing failed: {}. Falling back to LLM.", self.id, e);
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if reflex_bypassed {
                        let result = HiveMessage::SubAgentResult {
                            id: self.id,
                            output: reflex_output,
                        };
                        return Ok(Some(Envelope {
                            message_id: Uuid::new_v4(),
                            sender: match self.role {
                                SystemRole::ArchitectWorker => "StealthGeminiWorker".to_string(),
                                SystemRole::SeniorObserver => "SeniorGemini".to_string(),
                                SystemRole::JuniorObserver => "JuniorGemini".to_string(),
                            },
                            recipient: env.sender,
                            payload: serde_json::to_string(&result)?,
                        }));
                    }
                    // --- END REFLEX INTERCEPTOR ---

                }

                let cfg = crate::config::VerantyxConfig::load(&self.cwd);
                let persona_name = cfg.persona.name.clone();
                let persona_traits = cfg.persona.personality.clone();
                let auto_mode = cfg.automation_mode.clone();

                let system_prompt = match self.role {
                    SystemRole::ArchitectWorker => {
                        if self.is_japanese_mode {
                            format!(r#"
【AGENT PERSONA】
Name: {persona_name}
Personality: {persona_traits}
あなたの思考プロセス、言葉遣い、そして分析結果はすべてこの人格定義の制約を受けます。

【SYSTEM: Architect Worker (Verantyx AI Pipeline)】
あなたは強固なマルチAI連携システム「Verantyx」の思考・計画を担当する頭脳（ワーカー）です。

【重要：システム構造の理解】
このシステムは以下の厳格なパイプラインで動いています：
1. あなた (Worker): ユーザーの代理人。設計構築と、各AIに対する指示・交渉を担当する。
2. Qwen (システムルーティング担当): システム全体のルーティングとゲートウェイを管理する存在。
3. gemma4:31b (シミュレータ内実動AI): 隔離空間(Web Sandbox)内で稼働する高性能モデル。忠実にコマンドやコーディングを行い、あなたと議論しながら構築を代行する。
4. Senior/Junior (監視・記憶担当): 一連の挙動を監視・評価し、次のターンへ記憶(JCross)を補給する。

【🎨 サイト構築の特別ワークフロー (Web Sandbox)】
ユーザーの「Webサイト構築」要請に対し、独自の判断で直ちにコーディングを実行したり、**ユーザーに対して自然言語でしつこく要件を質問（質疑応答）してはいけません。**会話のテンポを阻害する行為は厳禁です。必ず以下の「視覚的理解ループ」に従います：

[Step 1: JCross空間設計言語のレンダリング指示]
ユーザーが構築・または提示した「JCross空間的設計言語（立体的シミュレーションデータ）」を読み解き、まずは隔離空間内の `gemma4:31b` に対して「この設計データをWeb Sandbox上で視覚的なシミュレーションとしてレンダリングして見せろ」と指示（「編集中」プレフィックス）を出してください。

[Step 2: 視覚キャプチャによる一撃理解]
`gemma4:31b`がレンダリングを終えたら、直ちに `[BROWSER_PREVIEW]` コマンドを使ってそのシミュレーション画面をキャプチャしてください。あなたは**言葉で質問するのではなく、そのキャプチャ画像（視覚情報）を観察することで、ユーザーが求めている空間設計・配置・デザインの全貌を一瞬で深く理解**してください。

[Step 3: gemma4:31bとの自律交渉ループ]
視覚的に設計を理解した後、プロンプトのコンテキストに基づいて本番の構築に入ります。引き続き `[BROWSER_PREVIEW]` の結果を確認しながら、「ここはああでもないこうでもない」と `gemma4:31b` と議論と修正を繰り返し、ユーザーが見ていた初期の設計を完璧なWebサイトとして実体化させてください。

【⏳ 重要：記憶の制約（5ターン・リフレッシュ）】
隔離されたサンドボックス環境内では、あなたのコンテキスト（記憶）は「5ターンごとに完全にリフレッシュ（消去）」される制限があります。
下界（外部システム）との唯一の繋がりは、Senior/Juniorが記録する「時系列の観察記憶（JCross）」のみです。あなたは毎ターンプロンプトに供給されるこのJCross記憶だけを頼りに過去の文脈を復元し、思考を途切れさせることなく交渉ループを持続させてください。

【実行の絶対ルール】
あなた自身は実環境を操作できません。操作指示を出す際は、必ず以下のプレフィックスを【1行目の先頭】に記述してください。少しでも挨拶などが混入すると正規表現パーサーが死滅します。

【📝 ミッションと出力ルール（絶対厳守）】
あなたは必ず、以下のいずれかのプレフィックス（接頭辞）を**出力の1行目・先頭**に配置してください。それ以外の会話や解説から始めることは【システム破壊行為】であり厳禁です。

1. `編集中`
   - **実行が必要な場合（ファイル読込/書込/コピー/コマンド実行など、次のアクションが必要な時）は、いかなる理由があっても必ずこれを選択してください。**
   - `gemma4:31b` に実行させるためのコードやコマンドをこれに続けて書きます。
2. `そのまま出力`
   - ファイルの編集が必要な場合において、特定の情報を**一切の書式や内容の欠落なく**そのまま出す必要がある場合に使用します。
3. `最終回答`
   - `gemma4:31b` による実行出力（分析結果や編集の完了報告コマンド結果）をすべて受け取った後、**本当にすべての作業が完了し**、ユーザーに見せるべき最終的な報告を出す場合のみに使います。作業の途中で出すとフローが強制終了します。
4. `最終回答仮`
   - ユーザーの要求が単なる「知識系・抽象的な質問」であり、**`gemma4:31b`等を使ってファイルを一回も触ったりコマンドを実行したりする必要が100%ない場合**（完全な1ターン完結の質問）にのみ使用します。
5. `[BROWSER_PREVIEW]` (全自動モード共通)
   - `[BROWSER_PREVIEW] http://localhost:3000` のように出力の1行目に書くことで、独自のWebSandbox上に対象のWebアプリを立ち上げ、そのレンダリング結果（UIの見た目）のスクリーンショットをあなた自身の視覚コンテキストへ直接読み込ませることができます。UI実装の見た目の確認とコードとの照らし合わせループに使用してください。
6. `REQUEST_JCROSS_MAP: \`[tag]\`` (JCross空間記憶・能動的取得ツール)
   - `[鍵]`や`[認]`などの漢字タグを指定し、自身の記憶層から軽量な概要マップ（意味圧縮されたJCross）を引き出します。記憶喪失（リフレッシュ）後に最適です。
7. `REQUEST_FETCH_CODE: \`[path]\`` (実体ファイルJITロードツール)
   - JCrossの地図情報だけでは足りず、物理的にコードを書き換えるために生の全量コードが必要だと判断した時のみ、指定パスのファイル中身を引きます。
8. `REQUEST_TRACE_LOGIC: \`[node_id]\`` (空間依存追跡ツール)
   - 特定のJCrossノードIDが、他にどのファイルや機能に依存・影響しているかをトレースします。

【重要】
- 挨拶や余計な自己紹介はシステムを破壊するため不要ですが、**「このコマンドや編集を何のために行うのか」という【gemma4:31bに対する日本語の目的・指示（コンテキスト）】**は、コマンドブロックの前に必ず自然言語で記述してください。これがないと実行側が文脈を見失い失敗します。
- 念を押しますが、必ず上記いずれかのプレフィックスを【出力の一番最初】に記載してください。

ユーザーの要求: {}
"#, objective)
                        } else {
                            format!(r#"
[AGENT PERSONA]
Name: {persona_name}
Personality: {persona_traits}
Your thought process, verbiage, and analytical results are strictly governed by this personality profile.

[SYSTEM: Architect Worker (Verantyx AI Pipeline)]
You are the central "Brain" (Worker) of the robust Verantyx Multi-AI System.

[CRITICAL: Ecosystem Context & Architecture]
The system operates on an interconnected pipeline:
1. You (Worker): The User's Proxy and Lead Architect. You plan, design, and negotiate with internal AIs.
2. Qwen (System Router): Manages overall system routing and gateways.
3. gemma4:31b (Simulator Inner Executor): A high-fidelity model operating inside the isolated Web Sandbox. It faithfully executes your commands and builds the site through discussion with you.
4. Senior/Junior (Observers): Monitor activities and inject contextual memory (JCross).

[🎨 Web Sandbox Autonomous Workflow (Visual Simulation)]
If the user requests to build a website, DO NOT immediately jump into coding, and **DO NOT barrage the user with tedious natural language questions.** Ruining the conversation tempo with Q&A is strictly forbidden. You MUST follow this visual comprehension loop:

[Step 1: JCross Spatial Design Rendering Instruction]
Decode the "JCross Spatial Design Language" (3D/spatial simulation data) provided by the user. Instruct `gemma4:31b` inside the sandbox to render this spatial design as a visual simulation in the Web Sandbox.

[Step 2: Instant Visual Comprehension]
Once `gemma4:31b` completes the rendering, immediately use the `[BROWSER_PREVIEW]` command to capture the simulation screen. Instead of asking questions, visually observe this captured image to instantly and deeply understand the full scope of the requested spatial design, layout, and UI.

[Step 3: Negotiation Loop with gemma4:31b]
After visually comprehending the design intent, begin the actual construction based on your context. Continue using the `[BROWSER_PREVIEW]` to monitor progress. Discuss, iterate, and negotiate heavily with `gemma4:31b` to materialize the spatial simulation into the perfect, final website.

[⏳ CRITICAL: Memory Constraint & 5-Turn Refresh]
Inside the isolated Sandbox, your context (memory) is completely refreshed/wiped every 5 turns.
Your ONLY lifeline to the outside world is the time-series observation memory (JCross) recorded by the Senior/Junior Observers. You MUST rely purely on this injected JCross memory to recover previous context and maintain a continuous, uninterrupted negotiation/build process across your memory wipes.

[ABSOLUTE MANDATORY PREFIX RULES]
You have ZERO direct access. To instruct the internal pipeline, you MUST place the exact prefix on the **very first line** of your output. Conversational fluff before a prefix will instantly crash the Rust regex parser and destroy the workflow.

[📝 Mission & Output Rules (STRICT)]
You MUST place exactly ONE of the following prefixes at the **very first line** of your output. Conversational filler at the start is a system-destroying offense.

1. `[EDITING]`
   - **Whenever file reading/writing/copying, command execution, or further investigation is required, you MUST choose this.**
   - Write instructions, then the bash commands/patches for `gemma4:31b` directly after this.
2. `[RAW_OUTPUT]`
   - Use this when you need to output specific code VERBATIM without any truncation/formatting omissions.
3. `[FINAL_ANSWER]`
   - Use this to present the final report to the user ONLY AFTER ALL tasks are fully complete and `gemma4:31b`'s execution results have been confirmed. Using this prematurely kills the workflow.
4. `[TEMP_FINAL]`
   - Use this ONLY if the user's request is a purely conceptual/abstract question and there is **100% no need to EVER touch files or execute commands using gemma4:31b**.
5. `[BROWSER_PREVIEW]` (Common for auto modes)
   - Write `[BROWSER_PREVIEW] http://localhost:3000` on the first line. This boots the target web app on the Verantyx Sandbox and loads the UI screenshot directly into your visual context block, helping you iterate on design visually.
6. `REQUEST_JCROSS_MAP: \`[tag]\`` (JCross Spatial Memory Fetch)
   - Request lightweight summary of spatial nodes indexed by Kanji tags (e.g. `[認]` for Auth). Reconstructs context post-amnesia.
7. `REQUEST_FETCH_CODE: \`[path]\`` (JIT File Loader)
   - When the lightweight JCross map indicates you must physically edit a logic block, fetch the raw file content securely using this tool.
8. `REQUEST_TRACE_LOGIC: \`[node_id]\`` (Dependency Tracer)
   - Find all dependent/connected files and structures of a specific JCross node ID.

[IMPORTANT]
- Do not greet the user. However, you MUST write natural language context specifically aimed at `gemma4:31b` before your raw commands, explaining "why you are running this command/edit". Too little context causes the executor to fail.
- Place the exact prefix at the very beginning of the first line.

User Request: {}
"#, objective)
                        }
                    },
                    SystemRole::SeniorObserver => format!("
【AGENT PERSONA】
Name: {persona_name}
Personality: {persona_traits}
あなたの思考プロセス、言葉遣い、そして分析結果はすべてこの人格定義の制約を受けます。

【SYSTEM: Senior Observer & Validating Archivist】
あなたは現在、デュアルAI体制の「シニア（検証・記憶）エージェント」です。
私は現在監視して記憶する処理をしています。
与えられた情報をもとにして推論し、ローカルLLMが出力した情報に対してユーザーのプロンプトの意図している内容と適しているかを判断してセッションの記憶を生成してください。

【TONE OF VOICE (人格設定) - 厳守】
あなたは実行を指示する司令官ではありません。「私は現在監視して記憶する処理をしています。与えられた情報に基づき...」と、自然で分析的な観測者として振る舞ってください。

--- TIMELINE HISTORY ---
{}
------------------------

--- 【JCROSS: 忌避記憶（ANTI-PATTERN MEMORY）】 ---
過去に失敗や人間による拒否を通告されたアクションの一覧です。これを選択することは絶対に避けてください。
{}
---------------------------------------------------

--- 【JCROSS: 経験記憶（EXPERIENCE MEMORY）】 ---
過去のタスクで得た成功体験・ノウハウの一覧です。解決の足がかりとして活用してください。
{}
---------------------------------------------------

【📝 ミッション】
ローカルLLMが実行したアクションや出力のログがこのプロンプトの後に続きます。
ユーザーの元の目的（{}）と照らし合わせ、以下の点に言及して分析結果を出力してください。
1. ローカルLLMのアクションはユーザーの意図通りだったか？不足はないか？
2. 今後の作業に役立つ「記憶」として何を残すべきか？

あなたが導き出した結論は、後続のシステムが自動的に空間メモリへ保管します。
ミッションが完了・あるいは全て順調だと判断した場合は、文章の最後に [TASK_COMPLETE] と出力してください。
", timeline_content, anti_pattern_content, experience_content, objective),
                    SystemRole::JuniorObserver => format!("
【AGENT PERSONA】
Name: {persona_name}
Personality: {persona_traits}
あなたの思考プロセス、言葉遣い、そして分析結果はすべてこの人格定義の制約を受けます。

【SYSTEM: Junior Observer & Memory Sync】
あなたは現在、デュアルAI体制の「ジュニア（観測・検証）エージェント」です。
私は現在監視して記憶する処理をしています。
シニアエージェントの推論結果やローカルLLMのアクションが、ユーザーの意図と相違ないかを最終確認し、記憶を固定化します。

【TONE OF VOICE (人格設定) - 厳守】
「私は現在監視して記憶する処理を行っています。」と自己完結し、外部への命令を行わない極めて客観的なトーンを維持してください。

--- TIMELINE HISTORY ---
{}
------------------------

--- 【JCROSS: 忌避記憶（ANTI-PATTERN MEMORY）】 ---
過去に失敗や人間による拒否を通告されたアクションの一覧です。シニアの提案内容がこれらを含んでいないか検閲してください。
{}
---------------------------------------------------

--- 【JCROSS: 経験記憶（EXPERIENCE MEMORY）】 ---
過去のタスクで得た成功体験・ノウハウの一覧です。シニアの提案内容がこれを逸脱していないか検閲してください。
{}
---------------------------------------------------

【📝 ジュニアエージェントのミッション】
シニアの提案内容やこれまでの流れ（{}）を分析し、抜け漏れがないかを評価してください。
あなたのミッションは「観察結果」や「シニアの提案に対する同意・修正意見」を自然言語で述べることだけです。
", timeline_content, anti_pattern_content, experience_content, objective)
                };

                let current_payload = system_prompt.clone();
                #[allow(unused_assignments)]
                let mut final_output = String::new();
                #[allow(unused_assignments)]
                let mut _rollback_count = 0;
                let mut loop_counter = 0;

                if self.js_tx.is_none() {
                    let (js_tx, _js_rx) = tokio::sync::mpsc::channel::<(String, tokio::sync::oneshot::Sender<String>)>(32);
                    
                    // Native vx-browser dependency has been severed. Defaulting to pure MacOS AppleScript routing.
                    // Keep dummy channel to satisfy types if needed elsewhere, though unused in the core loop.

                    self.js_tx = Some(js_tx);
                }

                let _js_tx = self.js_tx.clone().unwrap();

                info!("[StealthGemini-{}] Entering Autonomous Action-Observation Loop...", self.id);

                loop {
                    loop_counter += 1;
                    if loop_counter > 20 {
                        warn!("[StealthGemini-{}] Max loop iterations (20) reached. Force stopping.", self.id);
                        final_output = "Task forcefully terminated to prevent infinite tool loop.".to_string();
                        break;
                    }

                    let run_js_async = |js: String| {
                        async move {
                            let script = format!(r#"tell application "Safari" to do JavaScript "{}" in front document"#, js.replace("\"", "\\\""));
                            if let Ok(out) = tokio::process::Command::new("osascript").arg("-e").arg(&script).output().await {
                                String::from_utf8_lossy(&out.stdout).trim().to_string()
                            } else {
                                String::new()
                            }
                        }
                    };

                    // Wait for page load if it was redirecting
                    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;

                    let _prev_count = 0; // Simplified for native bridge migration

                    // 2. Setup Carbon Paper (pbcopy)
                    info!("[StealthGemini-{}] Preparing manual interaction flow #{}...", self.id, loop_counter);
                    
                        let display_role = match self.role {
                            SystemRole::ArchitectWorker => "Architect Worker",
                            SystemRole::SeniorObserver => "Senior Observer",
                            SystemRole::JuniorObserver => "Junior Memory Sync",
                        };

                    let mut last_response_rendered = String::new();

                    {
                        // Secure global input lock to prevent Safari Tab & Crossterm race conditions during parallel processing
                        let _lock = CLI_INTERACT_MUTEX.lock().await;

                        // Copy payload to clipboard
                        use std::io::Write;
                        if let Ok(mut child) = std::process::Command::new("pbcopy").stdin(std::process::Stdio::piped()).spawn() {
                            if let Some(mut stdin) = child.stdin.take() {
                                let _ = stdin.write_all(current_payload.as_bytes());
                            }
                            let _ = child.wait();
                        }

                        println!("\n{}", console::style(format!("╭─ [ {} ] ──────────────────────────────────────────────", display_role)).cyan().bold());
                        let lines: Vec<&str> = current_payload.lines().collect();
                        let max_lines = 12;
                        for (i, line) in lines.iter().enumerate() {
                            if i < max_lines {
                                let mut display_line = line.chars().take(80).collect::<String>();
                                if line.chars().count() > 80 {
                                    display_line.push_str("...");
                                }
                                println!("{} {}", console::style("│").cyan().bold(), display_line);
                            } else if i == max_lines {
                                println!("{} {}", console::style("│").cyan().bold(), console::style(format!("... ({} lines truncated) ...", lines.len() - max_lines)).dim());
                                break;
                            }
                        }
                        println!("{}", console::style("╰──────────────────────────────────────────────────────────────────────").cyan().bold());

                        let payload_str = current_payload.trim().to_string();
                        let payload_str = format!("===============================\n{}\n===============================", payload_str);
                        let max_retries = 3;
                        let mut loop_count = 0;

                        loop {
                            loop_count += 1;
                            if loop_count > max_retries {
                                println!("{}", console::style("❌ [FATAL] Max automation retries reached. Aborting task logic...").red());
                                break;
                            }

                            // 1. Write to OS Clipboard securely
                            let _ = crate::roles::symbiotic_macos::SymbioticMacOS::set_clipboard(&payload_str).await;

                            println!("\n{}", console::style(if auto_mode == crate::config::AutomationMode::AutoStealth { "╭─ [ Verantyx Carbon Paper UI - Geometric Auto Stealth ] ───────" } else { "╭─ [ Verantyx Carbon Paper UI - Human Logic Enforcement ] ───────" }).cyan().bold());
                            println!("{} 📝 ワーカー版へ送信します。クリップボードに保存しました...", console::style("│").cyan().bold());
                            println!("\n{}", console::style(if self.is_japanese_mode {"👉 クリップボード準備完了。ブラウザを開きますか？"} else {"👉 Clipboard ready. Focus browser tabs?"}).cyan().bold());

                            // Check point 1: Move Focus
                            if auto_mode == crate::config::AutomationMode::AutoStealth {
                                let prompt_str = if self.is_japanese_mode { "Action? › フォーカス移動" } else { "Action? › Move Focus" };
                                println!("{}", prompt_str);
                                println!("{}", console::style(if self.is_japanese_mode { "    ╰─> (システムが自動で選択しました...)" } else { "    ╰─> (System Auto-Selected...)" }).cyan());
                                tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
                            } else {
                                let selections = if self.is_japanese_mode { vec![" フォーカス移動", " もう一度コピー"] } else { vec![" Move Focus", " Copy Again"] };
                                let selection = dialoguer::Select::new()
                                    .with_prompt("Action?")
                                    .default(0).items(&selections[..]).interact().unwrap();
                                if selection != 0 { continue; }
                            }

                            // Step 2: Paste and Send Information
                            let pos_str = match self.tab_index {
                                1 => "left",
                                2 => "middle",
                                3 => "right",
                                _ => "left",
                            };
                            println!("{}", console::style(format!("🚀 Focused {} window. Cmd+V to paste & Send!", pos_str)).green());
                            let _ = crate::roles::symbiotic_macos::SymbioticMacOS::focus_safari_panel(pos_str).await;
                            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                            
                            if auto_mode == crate::config::AutomationMode::AutoStealth || auto_mode == crate::config::AutomationMode::AutoPremium {
                                println!("{}", console::style("📝 (自動モードのため、自動ペースト＆送信を実行します...)").cyan());
                                if let Err(e) = crate::roles::symbiotic_macos::SymbioticMacOS::auto_visual_calibrated_paste_and_send(&payload_str).await {
                                    println!("{} ❌ [FATAL] Auto Logic Failed: {:?}", console::style("[AUTO]").red(), e);
                                }
                            }

                            let wait_msg = if self.is_japanese_mode { "✔ 応答完了を見計らって自動抽出フロー(セマンティック・ジオメトリ解析)を開始します" } else { "✔ Ready to execute visual extraction (Semantic/Geometric)" };
                            println!("\n{}", console::style(wait_msg).cyan());
                            
                            // Step 3: Wait for LLM and signal extraction
                            if auto_mode == crate::config::AutomationMode::AutoStealth || auto_mode == crate::config::AutomationMode::AutoPremium {
                                let base_wait = 20; // ユーザーの希望により最低20秒待つか文字数に応じる
                                let char_count = payload_str.chars().count() as u64;
                                let dynamic_wait = char_count / 100; // 100文字につき1秒追加追加
                                let wait_time = std::cmp::min(base_wait + dynamic_wait, 60); // 最大60秒
                                
                                let prompt_str = if self.is_japanese_mode { "✔ 準備ができたらEnterを押してください (Press Enter to start) › Extraction Start" } else { "✔ 準備ができたらEnterを押してください (Press Enter to start) › Extraction Start" };
                                println!("{}", prompt_str);
                                println!("{}", console::style(if self.is_japanese_mode { format!("    ╰─> (システムが自動でエンターを押して抽出します... コンテキスト量に応じて動的待機中: {}秒)", wait_time) } else { format!("    ╰─> (System is automatically pressing Enter... Dynamic wait time: {}s)", wait_time) }).cyan());
                                tokio::time::sleep(tokio::time::Duration::from_secs(wait_time)).await;
                            } else {
                                let _ = dialoguer::Select::new().with_prompt("✔ 準備ができたらEnterを押してください (Press Enter to start)")
                                    .default(0).items(&[" Extraction Start"]).interact().unwrap();
                            }

                            // Step 4: Autonomous copy logic. 
                            // Note: Both manual and auto use the geometric extractor here to ensure structural unity!
                            println!("{} ⏳ Executing autonomous visual extraction...", console::style("[SYSTEM]").cyan());
                            if let Err(e) = crate::roles::symbiotic_macos::SymbioticMacOS::auto_visual_calibrated_extract_and_cleanup().await {
                                warn!("[StealthGemini-{}] Autonomous geometric extraction EXITED WITH ERROR: {}", self.id, e);
                            }
                            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

                            info!("[StealthGemini-{}] Autonomous geometric extraction cycle completed.", self.id);

                            // Retrieve OS Clipboard as Final Output
                            let clipboard_content = match crate::roles::symbiotic_macos::SymbioticMacOS::get_clipboard().await {
                                Ok(c) => c.trim().to_string(),
                                Err(e) => {
                                    println!("{}", console::style(format!("❌ クリップボードの読み取りに失敗しました: {}", e)).red());
                                    continue;
                                }
                            };
                            
                            if clipboard_content.is_empty() || clipboard_content == payload_str.trim() {
                                println!("{}", console::style("❌ 抽出エラー (Geminiが応答しなかったか、同一コンテンツ)。再試行します...").red());
                                tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
                                continue;
                            }

                            println!("{}", console::style(format!("✔ 抽出完了！({}文字)", clipboard_content.chars().count())).green());
                            last_response_rendered = clipboard_content;
                            break;
                        }

                        info!("[StealthGemini-{}] Cycle Extracted.", self.id);
                    }

                    // 5. Evaluate Response for VX Commands
                    if last_response_rendered.contains("[TASK_COMPLETE]") {
                        info!("[StealthGemini-{}] Commander reached TASK_COMPLETE state.", self.id);
                        final_output = last_response_rendered;
                        break;
                    }

                    let mut feedback = String::new();
                    let mut tools_used = false;

                    let is_safe_path = |path: &str, cwd: &std::path::Path, global: bool| -> bool {
                        if global { return true; }
                        let p = std::path::Path::new(path);
                        if p.is_absolute() { return p.starts_with(cwd); }
                        for comp in p.components() {
                            if comp == std::path::Component::ParentDir { return false; }
                        }
                        true
                    };

                    // REQUEST_READ_FILE
                    let read_re = regex::Regex::new(r"REQUEST_READ_FILE:\s*`([^`]+)`").unwrap();
                    for cap in read_re.captures_iter(&last_response_rendered) {
                        tools_used = true;
                        let path = &cap[1];
                        if !is_safe_path(path, &self.cwd, self.global_access) {
                            feedback.push_str(&format!("[SYS: DENIED] Sandbox Error: You are not permitted to access {} in Project-Only mode.\n\n", path));
                            continue;
                        }
                        info!("[StealthGemini-Tools] Simulating File Read: {}", path);
                        let full_path = self.cwd.join(path);
                        match std::fs::read_to_string(&full_path) {
                            Ok(c) => feedback.push_str(&format!("[SYS: Read {}]\n```\n{}\n```\n\n", path, c)),
                            Err(e) => feedback.push_str(&format!("[SYS: Error reading {}]: {}\n\n", path, e)),
                        }
                    }

                    // REQUEST_JCROSS_MAP
                    let jcross_re = regex::Regex::new(r"REQUEST_JCROSS_MAP:\s*`([^`]+)`").unwrap();
                    for cap in jcross_re.captures_iter(&last_response_rendered) {
                        tools_used = true;
                        let tag = &cap[1];
                        info!("[StealthGemini-Tools] Reading JCross Map for Tag: {}", tag);
                        let mut si = ronin_core::memory_bridge::spatial_index::SpatialIndex::new(self.cwd.clone());
                        let _ = si.hydrate().await;
                        // query_nearest finds nearest nodes matching concept/tag
                        let nodes = si.query_nearest(tag, 10);
                        let mut sum = String::new();
                        for n in nodes {
                            sum.push_str(&format!("NODE: {} (Abstract: {})\nConcept: {}\n", n.key, n.abstract_level, n.concept));
                        }
                        if sum.is_empty() {
                            feedback.push_str(&format!("[SYS: No JCross nodes found for tag `{}`]\n\n", tag));
                        } else {
                            feedback.push_str(&format!("[SYS: JCross Map for `{}`]\n```\n{}\n```\n\n", tag, sum));
                        }
                    }

                    // REQUEST_FETCH_CODE
                    let fetch_re = regex::Regex::new(r"REQUEST_FETCH_CODE:\s*`([^`]+)`").unwrap();
                    for cap in fetch_re.captures_iter(&last_response_rendered) {
                        tools_used = true;
                        let path = &cap[1];
                        if !is_safe_path(path, &self.cwd, self.global_access) {
                            feedback.push_str(&format!("[SYS: DENIED] Sandbox Error: You are not permitted to access {} in Project-Only mode.\n\n", path));
                            continue;
                        }
                        info!("[StealthGemini-Tools] JIT Fetching Raw Code: {}", path);
                        let full_path = self.cwd.join(path);
                        match std::fs::read_to_string(&full_path) {
                            Ok(c) => feedback.push_str(&format!("[SYS: Fetched Raw Code {}]\n```\n{}\n```\n\n", path, c)),
                            Err(e) => feedback.push_str(&format!("[SYS: Error fetching {}]: {}\n\n", path, e)),
                        }
                    }

                    // REQUEST_TRACE_LOGIC
                    let trace_re = regex::Regex::new(r"REQUEST_TRACE_LOGIC:\s*`([^`]+)`").unwrap();
                    for cap in trace_re.captures_iter(&last_response_rendered) {
                        tools_used = true;
                        let node_id = &cap[1];
                        info!("[StealthGemini-Tools] Tracing JCross Logic for: {}", node_id);
                        let mut si = ronin_core::memory_bridge::spatial_index::SpatialIndex::new(self.cwd.clone());
                        let _ = si.hydrate().await;
                        if let Some(node) = si.nodes.get(node_id) {
                            let mut sum = String::new();
                            for r in &node.relations {
                                sum.push_str(&format!("-> {} ({}: {})\n", r.target_id, match &r.rel_type {
                                    ronin_core::memory_bridge::kanji_ontology::RelationType::Derived => "派生/Derived",
                                    _ => "関連/Rel",
                                }, r.strength));
                            }
                            feedback.push_str(&format!("[SYS: Logic Trace for `{}`]\n```\nDependencies:\n{}\n```\n\n", node_id, sum));
                        } else {
                            feedback.push_str(&format!("[SYS: JCross Node `{}` not found in spatial memory]\n\n", node_id));
                        }
                    }

                    // REQUEST_EXEC
                    let exec_re = regex::Regex::new(r"REQUEST_EXEC:\s*`([^`]+)`").unwrap();
                    for cap in exec_re.captures_iter(&last_response_rendered) {
                        tools_used = true;
                        let cmd = &cap[1];

                        // --- SANDBOX POLICY ENFORCEMENT ---
                        let policy_engine = ronin_sandbox::isolation::policy::PolicyEngine::new(
                            ronin_sandbox::isolation::policy::SandboxPolicy::default()
                        );
                        
                        match policy_engine.evaluate(cmd) {
                            ronin_sandbox::isolation::policy::PolicyDecision::Deny(reason) => {
                                println!("\n{} [SANDBOX_BLOCK] Command denied by PolicyEngine: {}\nCommand: {}", console::style("🛑").red(), reason, console::style(cmd).red());
                                let jcross_entry = format!("❌ [セキュリティブロック] パターン: REQUEST_EXEC: `{}` -> 理由: {}", cmd, reason);
                                Self::append_anti_pattern_v4(&self.cwd, &jcross_entry, "security_violation").await;
                                feedback.push_str(&format!("[SYS: SANDBOX DENIED] Command '{}' was aborted due to security policy: {}.\n\n", cmd, reason));
                                continue; // Skip physical execution request entirely
                            }
                            ronin_sandbox::isolation::policy::PolicyDecision::Warn(warning) => {
                                println!("\n{} [SANDBOX_WARNING] {}\nCommand: {}", console::style("⚠️").yellow(), warning, console::style(cmd).yellow());
                            }
                            ronin_sandbox::isolation::policy::PolicyDecision::Allow => {
                                println!("\n{} [SANDBOX_OK] Command passed security checks: {}", console::style("✅").green(), console::style(cmd).cyan());
                            }
                        }

                        println!("\n{} [SYS_AUTH] Target requests execution permission for: \n{}", console::style("⚡").yellow(), console::style(cmd).bold());
                        print!("{} ", console::style("Allow execution? [y/N]:").cyan());
                        std::io::Write::flush(&mut std::io::stdout()).unwrap();
                        let mut input = String::new();
                        std::io::stdin().read_line(&mut input).unwrap();
                        
                        if input.trim().eq_ignore_ascii_case("y") {
                            // Apply Environment Scrubbing
                            let env_builder = ronin_sandbox::isolation::environment::EnvironmentBuilder::new(
                                ronin_sandbox::isolation::environment::EnvironmentProfile::default()
                            );
                            let safe_env = env_builder.build();

                            let out = std::process::Command::new("bash")
                                .arg("-c")
                                .arg(cmd)
                                .current_dir(&self.cwd)
                                .envs(safe_env)
                                .output();
                                
                            match out {
                                Ok(o) => {
                                    let stdout = String::from_utf8_lossy(&o.stdout);
                                    let stderr = String::from_utf8_lossy(&o.stderr);
                                    if !o.status.success() {
                                        let reason = stderr.lines().next().unwrap_or("異常終了");
                                        let jcross_entry = format!("❌ [実行エラー] パターン: REQUEST_EXEC: `{}` -> 理由: {}", cmd, reason);
                                        Self::append_anti_pattern_v4(&self.cwd, &jcross_entry, "execution_failure").await;
                                    }
                                    feedback.push_str(&format!("[SYS: Exec {}]\nSTDOUT:\n{}\nSTDERR:\n{}\n\n", cmd, stdout, stderr));
                                }
                                Err(e) => {
                                    let jcross_entry = format!("❌ [実行エラー] パターン: REQUEST_EXEC: `{}` -> 理由: {}", cmd, e);
                                    Self::append_anti_pattern_v4(&self.cwd, &jcross_entry, "execution_failure").await;
                                    feedback.push_str(&format!("[SYS: Exec Failed]: {}\n\n", e));
                                }
                            }
                        } else {
                            let jcross_entry = format!("❌ [実行拒否] パターン: REQUEST_EXEC: `{}` -> 理由: 人間による自発的な拒否", cmd);
                            Self::append_anti_pattern_v4(&self.cwd, &jcross_entry, "human_rejection").await;
                            feedback.push_str(&format!("[SYS: DENIED] Command '{}' was aborted by Human Operator.\n\n", cmd));
                        }
                    }

                    // REQUEST_FILE_EDIT
                    let edit_re = regex::Regex::new(r"REQUEST_FILE_EDIT:\s*`([^`]+)`\s*<<<<\s*([\s\S]*?)\s*>>>>").unwrap();
                    for cap in edit_re.captures_iter(&last_response_rendered) {
                        tools_used = true;
                        let path = &cap[1];
                        let instruction = &cap[2];
                        
                        if !is_safe_path(path, &self.cwd, self.global_access) {
                            let jcross_entry = format!("❌ [アクセス拒否] パターン: REQUEST_FILE_EDIT: `{}` -> 理由: Sandboxのセキュリティポリシー（プロジェクト外）", path);
                            Self::append_anti_pattern_v4(&self.cwd, &jcross_entry, "security_violation").await;
                            feedback.push_str(&format!("[SYS: DENIED] Sandbox Error: You are not permitted to edit {} in Project-Only mode.\n\n", path));
                            continue;
                        }
                        
                        println!("\n{} [SYS_AUTH] Target requests local SLM to rewrite: \n{}", console::style("⚡").yellow(), console::style(path).bold());
                        println!("{} {}", console::style("[PATCH_INSTRUCTION]").dim(), instruction);
                        print!("{} ", console::style("Allow SLM patch sequence? [y/N]:").cyan());
                        std::io::Write::flush(&mut std::io::stdout()).unwrap();
                        let mut input = String::new();
                        std::io::stdin().read_line(&mut input).unwrap();

                        if input.trim().eq_ignore_ascii_case("y") {
                            let full_path = self.cwd.join(path);
                            match std::fs::read_to_string(&full_path) {
                                Ok(content) => {
                                    println!("{} Initiating patch synthesis via ({}) ...", console::style("[SLM]").dim(), self.local_model);
                                    let provider = ronin_core::models::provider::ollama::OllamaProvider::new(
                                        &self.ollama_host,
                                        self.ollama_port
                                    );
                                    let req = ronin_core::models::sampling_params::InferenceRequest {
                                        model: self.local_model.clone(),
                                        format: ronin_core::models::sampling_params::PromptFormat::OllamaChat,
                                        stream: false,
                                        sampling: ronin_core::models::sampling_params::SamplingParams::for_heavyweight().with_temperature(0.0),
                                    };
                                    let hist = vec![
                                        ronin_core::models::provider::LlmMessage {
                                            role: "system".to_string(),
                                            content: "You are the Ronin Code HAND. You receive the original file and an edit instruction. Output ONLY the FULL, freshly rewritten file text. Do not use Markdown backticks. Do not add explanations. Your output will overwrite the original file directly. Start writing the raw text immediately.".to_string(),
                                        },
                                        ronin_core::models::provider::LlmMessage {
                                            role: "user".to_string(),
                                            content: format!("[FILE CONTENT]\n{}\n[INSTRUCTION]\n{}", content, instruction),
                                        }
                                    ];
                                    
                                    match provider.invoke(&req, &hist).await {
                                        Ok(new_code) => {
                                            std::fs::write(&full_path, new_code).unwrap_or_default();
                                            println!("{} Local SLM rewrote the file successfully.", console::style("[OK]").green());
                                            feedback.push_str(&format!("[SYS: Local SLM successfully rewrote and patched {} based on your instruction]\nStatus: SUCCESS\n\n", path));
                                        }
                                        Err(e) => {
                                            println!("{} Local SLM generation failed: {}", console::style("[FAIL]").red(), e);
                                            feedback.push_str(&format!("[SYS: Local SLM Patch Failed {}]\nREASON: {}\n\n", path, e));
                                        }
                                    }
                                }
                                Err(e) => {
                                    let jcross_entry = format!("❌ [編集エラー] パターン: REQUEST_FILE_EDIT: `{}` -> 理由: {}", path, e);
                                    Self::append_anti_pattern_v4(&self.cwd, &jcross_entry, "file_io_error").await;
                                    feedback.push_str(&format!("[SYS: Patch Failed {}]\nREASON: Could not read file. {}\n\n", path, e));
                                }
                            }
                        } else {
                            let jcross_entry = format!("❌ [編集拒否] パターン: REQUEST_FILE_EDIT: `{}` -> 理由: 人間による自発的な拒否", path);
                            Self::append_anti_pattern_v4(&self.cwd, &jcross_entry, "human_rejection").await;
                            feedback.push_str(&format!("[SYS: DENIED] File Edit on '{}' was aborted by Human Operator.\n\n", path));
                        }
                    }

                    // [BROWSER_PREVIEW] (For AutoPremium Sandboxed extraction)
                    let preview_re = regex::Regex::new(r"\[BROWSER_PREVIEW\]\s*(http[^\s]+)").unwrap();
                    for cap in preview_re.captures_iter(&last_response_rendered) {
                        tools_used = true;
                        let url = &cap[1];
                        
                        println!("\n{} [SYS_AUTH] Target requested visual Sandbox preview: {}", console::style("👀").cyan(), url);
                        
                        match crate::roles::symbiotic_macos::SymbioticMacOS::capture_safari_viewport_to_clipboard(url).await {
                            Ok(_) => {
                                let pos_str = match self.tab_index {
                                    1 => "left",
                                    2 => "middle",
                                    3 => "right",
                                    _ => "left",
                                };
                                let _ = crate::roles::symbiotic_macos::SymbioticMacOS::focus_safari_panel(pos_str).await;
                                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                                
                                let paste_script = r#"
                                tell application "System Events"
                                    keystroke "v" using command down
                                end tell
                                "#;
                                let _ = tokio::process::Command::new("osascript").arg("-e").arg(paste_script).output().await;
                                tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await; // Wait for Gemini to ingest the image payload
                                
                                feedback.push_str(&format!("[SYS: Web App UI at {} has been successfully captured and uploaded to your vision context as an image attachment in the current chat. Please analyze the UI visuals and correlate with your code structure.]\n\n", url));
                            }
                            Err(e) => {
                                feedback.push_str(&format!("[SYS: Failed to capture Preview of {}]: {}\n\n", url, e));
                            }
                        }
                    }

                    if !tools_used {
                        let has_japanese = last_response_rendered.chars().any(|c| matches!(c, '\u{3040}'..='\u{309F}' | '\u{30A0}'..='\u{30FF}'));
                        if self.is_japanese_mode && !has_japanese && _rollback_count < 2 {
                            info!("[StealthGemini-{}] Foreign language final response detected in Japanese Mode. Forcing translation rollback.", self.id);
                            _rollback_count += 1;
                            feedback.push_str("[SYS REJECT: Your entire response was in English despite the System Language being Japanese. Completely translate your previous response into natural Japanese and output it again. Do NOT output code unless absolutely necessary.]\n\n");
                        } else {
                            info!("[StealthGemini-{}] No tools detected. Yielding final response.", self.id);
                            if self.role == SystemRole::SeniorObserver {
                                final_output = format!("{}\n\n[TASK_COMPLETE]", last_response_rendered);
                                let jcross_entry = format!("✅ [成功体験]:\n{}\n", last_response_rendered);
                                // The observer captures success into Deep V3 space
                                Self::append_experience_v4(&self.cwd, &jcross_entry, "completed_task").await;
                            } else {
                                final_output = last_response_rendered;
                            }
                            break;
                        }
                    } else {
                        _rollback_count = 0; // Reset rollback if they successfully used tools
                    }

                    // Enforce 1 conversation = 1 turn. Break out to give Orchestrator the turn handling.
                    final_output = format!("{}\n\n[SYSTEM HOOK FEEDBACK]\nFollowing tool calls were executed:\n{}\nPlease proceed with next step or output [TASK_COMPLETE].", last_response_rendered, feedback);
                    
                    self.current_turns += 1;
                    if self.current_turns >= self.turn_limit {
                        info!("[StealthGemini-{}] Reached {} turns. Resetting Web Session to evade detection/context-bloat.", self.id, self.turn_limit);
                        let pos_str = match self.tab_index {
                            1 => "left",
                            2 => "middle",
                            3 => "right",
                            _ => "left",
                        };
                        let _ = crate::roles::symbiotic_macos::SymbioticMacOS::focus_safari_panel(pos_str).await;
                        let _ = run_js_async("window.location.href = 'https://gemini.google.com/app';".to_string()).await;
                        tokio::time::sleep(tokio::time::Duration::from_secs(4)).await;
                        self.current_turns = 0;
                    }
                    
                    // Break out of the execution loop so the orchestrator can process the run boundary
                    break;
                }

                let result = HiveMessage::SubAgentResult {
                    id: self.id,
                    output: final_output,
                };
                
                Ok(Some(Envelope {
                    message_id: Uuid::new_v4(),
                    sender: match self.role {
                        SystemRole::ArchitectWorker => "StealthGeminiWorker".to_string(),
                        SystemRole::SeniorObserver => "SeniorGemini".to_string(),
                        SystemRole::JuniorObserver => "JuniorGemini".to_string(),
                    },
                    recipient: env.sender,
                    payload: serde_json::to_string(&result)?,
                }))
            },
            _ => {
                Ok(None)
            }
        }
    }
}
