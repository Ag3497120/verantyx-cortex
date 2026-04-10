//! JCross v4 6-Axis Semantic Engine (Kanji Spatial Ontology)
//!
//! Upgraded from v3 JSON graphs to flat text `.jcross` semantic documents with a 
//! lightweight `.jidx` indexing layer. Memories are driven by symbolic Kanji operators.

use crate::domain::error::{Result, RoninError};
use crate::domain::types::MemoryZone;
use crate::memory_bridge::kanji_ontology::{KanjiOp, KanjiTag, TypedRelation, RelationType};
use chrono::{DateTime, Utc, TimeZone};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use tokio::fs;
use tracing::info;

// ─────────────────────────────────────────────────────────────────────────────
// Spatial Memory Node (JCross V4)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct MemoryNode {
    pub key: String,
    
    // Core Kanji Semantic Engine Tensors
    pub kanji_tags: Vec<KanjiTag>,
    pub relations: Vec<TypedRelation>,
    
    // 6-Axis Contextual Dimensions
    pub concept: String,
    pub time_stamp: f64,
    pub abstract_level: f64,
    
    // Core payload
    pub content: String,
    
    // Temporary variables matching legacy compat
    pub zone: MemoryZone,
    pub confidence: f64,
    pub utility: f64,
    pub created_at: DateTime<Utc>,
    pub weight: f32,
    
    // Reflex Engine (Muscle Memory)
    pub reflex_action: Option<String>,
    pub env_hash: Option<String>,
}

impl Default for MemoryNode {
    fn default() -> Self {
        Self {
            key: "UNCLASSIFIED".to_string(),
            kanji_tags: vec![],
            relations: vec![],
            concept: String::new(),
            time_stamp: Utc::now().timestamp() as f64,
            abstract_level: 0.5,
            content: String::new(),
            zone: MemoryZone::Mid,
            confidence: 1.0,
            utility: 1.0,
            created_at: Utc::now(),
            weight: 1.0,
            reflex_action: None,
            env_hash: None,
        }
    }
}

impl MemoryNode {
    pub fn new_v4(key: &str, content: &str) -> Self {
        Self {
            key: key.to_string(),
            content: content.to_string(),
            ..Default::default()
        }
    }

    pub fn new_front(key: &str, content: &str) -> Self {
        let mut node = Self::new_v4(key, content);
        node.zone = MemoryZone::Front;
        node
    }

    pub fn parse_jcross(raw: &str) -> Option<Self> {
        let mut node = Self::default();
        let mut reflex_buffer = String::new();
        let mut content_buffer = String::new();
        let mut in_content = false;
        let mut in_reflex = false;
        
        let lines: Vec<&str> = raw.lines().collect();
        for mut i in 0..lines.len() {
            let line = lines[i].trim();
            
            if in_content {
                if line == "===" { 
                    in_content = false;
                    continue; 
                }
                content_buffer.push_str(line);
                content_buffer.push('\n');
                continue;
            }

            if in_reflex {
                if line == "===" {
                    in_reflex = false;
                    continue;
                }
                reflex_buffer.push_str(line);
                reflex_buffer.push('\n');
                continue;
            }

            if line.starts_with("■ JCROSS_NODE_") {
                node.key = line.replace("■ JCROSS_NODE_", "").trim().to_string();
            } else if line.starts_with("【空間座相】") {
                if i + 1 < lines.len() {
                    let next_line = lines[i+1].trim();
                    let parts: Vec<&str> = next_line.split("] [").collect();
                    for ref p in parts {
                        if let Some(tag) = KanjiTag::parse(p) {
                            node.kanji_tags.push(tag);
                        }
                    }
                }
            } else if line.starts_with("【次元概念】") {
                if i + 1 < lines.len() {
                    node.concept = lines[i+1].trim().to_string();
                }
            } else if line.starts_with("【時間刻印】") {
                if i + 1 < lines.len() {
                    if let Ok(dt) = DateTime::parse_from_rfc3339(lines[i+1].trim()) {
                        node.time_stamp = dt.timestamp() as f64;
                    }
                }
            } else if line.starts_with("【環境刻印】") {
                if i + 1 < lines.len() {
                    node.env_hash = Some(lines[i+1].trim().to_string());
                }
            } else if line.starts_with("【連帯】") {
                while i + 1 < lines.len() && !lines[i+1].trim().is_empty() && !lines[i+1].starts_with("【") {
                    let rel_line = lines[i+1].trim();
                    let r_parts: Vec<&str> = rel_line.split(':').collect();
                    if r_parts.len() >= 2 {
                        let target = r_parts[0].trim().to_string();
                        let r_type = RelationType::from_str(r_parts[1].trim());
                        let str_val = if r_parts.len() > 2 { r_parts[2].parse::<f32>().unwrap_or(0.5) } else { 0.5 };
                        node.relations.push(TypedRelation { target_id: target, rel_type: r_type, strength: str_val });
                    }
                    i += 1;
                }
            } else if line.starts_with("【抽象度】") {
                if i + 1 < lines.len() {
                    node.abstract_level = lines[i+1].trim().parse::<f64>().unwrap_or(0.5);
                }
            } else if line.starts_with("【反射】") {
                in_reflex = true;
            } else if line.starts_with("[本質記憶]") {
                in_content = true;
            }
        }

        node.content = content_buffer.trim().to_string();
        if !reflex_buffer.trim().is_empty() {
            node.reflex_action = Some(reflex_buffer.trim().to_string());
        }
        
        if node.key == "UNCLASSIFIED" { return None; }
        
        Some(node)
    }

    /// Serializes back into human-readable `.jcross` format
    pub fn to_jcross(&self) -> String {
        let stamps_str = self.kanji_tags.iter().map(|t| format!("[{}:{}]", t.name, t.weight)).collect::<Vec<_>>().join(" ");
        let relations_str = self.relations.iter()
            .map(|r| {
                let r_name = match &r.rel_type {
                    RelationType::Derived => "派生",
                    RelationType::Base => "基底",
                    RelationType::Similar => "類似",
                    RelationType::Opposite => "対立",
                    RelationType::Prev => "前項",
                    RelationType::Next => "次項",
                    RelationType::Cause => "因果",
                    RelationType::Fix => "訂正",
                    RelationType::Context => "補足",
                    RelationType::Unknown(name) => name.as_str()
                };
                format!("{}:{}:{}", r.target_id, r_name, r.strength)
            })
            .collect::<Vec<_>>().join("\n");
        let dt = Utc.timestamp_opt(self.time_stamp as i64, 0).unwrap();

        let mut out = format!(
r#"■ JCROSS_NODE_{}

【空間座相】
{}

【次元概念】
{}

【時間刻印】
{}

【連帯】
{}

【抽象度】
{}
"#,
            self.key, stamps_str, self.concept, dt.to_rfc3339(), relations_str, self.abstract_level
        );

        if let Some(ref env) = self.env_hash {
            out.push_str("\n【環境刻印】\n");
            out.push_str(env);
            out.push_str("\n");
        }

        if let Some(ref reflex) = self.reflex_action {
            out.push_str("\n【反射】\n");
            out.push_str(reflex);
            out.push_str("\n===\n");
        }

        out.push_str("\n---\n[本質記憶]\n");
        out.push_str(&self.content);
        out.push_str("\n===\n");
        
        out
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Token Overlap (Bi-gram String Similarity for English & Japanese)
// ─────────────────────────────────────────────────────────────────────────────

fn token_overlap(a: &str, b: &str) -> f64 {
    fn bigrams(text: &str) -> HashSet<String> {
        let chars: Vec<char> = text.chars().filter(|c| !c.is_whitespace()).collect();
        let mut set = HashSet::new();
        if chars.len() < 2 {
            if chars.len() == 1 { set.insert(chars[0].to_string()); }
            return set;
        }
        for i in 0..chars.len() - 1 {
            let mut s = String::new();
            s.push(chars[i]);
            s.push(chars[i+1]);
            set.insert(s);
        }
        set
    }
    
    let set_a = bigrams(a);
    let set_b = bigrams(b);
    if set_a.is_empty() || set_b.is_empty() { return 0.0; }
    let intersection = set_a.intersection(&set_b).count() as f64;
    let union = set_a.union(&set_b).count() as f64;
    intersection / union
}

// ─────────────────────────────────────────────────────────────────────────────
// Spatial Index (V4 Indexing Layer)
// ─────────────────────────────────────────────────────────────────────────────

pub struct SpatialIndex {
    pub root: PathBuf,
    pub nodes: HashMap<String, MemoryNode>,
    pub ontology: HashMap<String, KanjiOp>,
}

impl SpatialIndex {
    pub fn new(root: PathBuf) -> Self {
        Self { 
            root, 
            nodes: HashMap::new(),
            ontology: KanjiOp::standard_ontology()
        }
    }

    /// Hydrates isolated `.jcross` text nodes utilizing `.jidx` caches
    pub async fn hydrate(&mut self) -> Result<usize> {
        let mut total = 0;
        let v4_dir = self.root.parent().unwrap_or(&self.root).join("jcross_v4");

        // Temporary Migration: if V4 doesn't exist, just create the dir and continue.
        // Deep parsing logic of V3 JSONs is omitted here for brevity; assume new DB starting point or manual migration outside this scope.
        if !v4_dir.exists() {
            fs::create_dir_all(&v4_dir).await.map_err(RoninError::Io)?;
        }

        let mut entries = fs::read_dir(&v4_dir).await.map_err(RoninError::Io)?;
        while let Some(entry) = entries.next_entry().await.map_err(RoninError::Io)? {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("jcross") {
                if let Ok(content) = fs::read_to_string(&path).await {
                    if let Some(node) = MemoryNode::parse_jcross(&content) {
                        self.nodes.insert(node.key.clone(), node);
                        total += 1;
                    }
                }
            }
        }

        info!("[SpatialIndex] Hydrated {} V4 JCross files from {}", total, v4_dir.display());
        Ok(total)
    }

    /// V4 Scoring Algorithm: Merges token overlap with Kanji Operational Gravity Tensors
    pub fn query_nearest(&self, query_concept: &str, limit: usize) -> Vec<&MemoryNode> {
        let now = Utc::now().timestamp() as f64;
        
        let mut scored_nodes: Vec<(f64, &MemoryNode)> = self.nodes.values().filter_map(|n| {
            // 1. Calculate base string similarity (Concept Vector)
            let base_score = token_overlap(&n.concept, query_concept);
            if base_score < 0.05 && limit < 100 { // Fast prune if utterly unrelated
                // Continue though, because urgent Kanji tags might save it
            }

            // 2. Extrapolate physics modifier from Kanji Tags
            let mut gravity = 1.0;
            let mut decay_rate = 0.05;
            let mut should_purge = false;

            for tag in &n.kanji_tags {
                if let Some(op) = self.ontology.get(&tag.name) {
                    if op.is_purge_flag { should_purge = true; }
                    gravity += op.gravity_mod * tag.weight;
                    decay_rate *= 1.0 - (1.0 - op.decay_mod) * tag.weight;
                }
            }

            if should_purge { return None; } // "破" tag ejects from spatial search

            // 3. Time delay projection
            let age_hours = (now - n.time_stamp) / 3600.0;
            let time_penalty = (age_hours * decay_rate as f64).clamp(0.0, 1.0);
            
            // 4. Transform Score (Physics evaluation)
            let final_score = (base_score * gravity as f64) 
                            + (n.confidence * 0.2) 
                            + (n.utility * 0.2) 
                            - time_penalty;
                      
            Some((final_score, n))
        }).collect();

        // Sort descending by calculated gravitation pull score
        scored_nodes.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        
        scored_nodes.into_iter().take(limit).map(|(_s, n)| n).collect()
    }

    /// Writes a V4 JCross Graph Node to physical disk
    pub async fn write_node(&mut self, mut node: MemoryNode) -> Result<()> {
        let v4_dir = self.root.parent().unwrap_or(&self.root).join("jcross_v4");
        fs::create_dir_all(&v4_dir).await.map_err(RoninError::Io)?;

        if node.time_stamp == 0.0 {
            node.time_stamp = Utc::now().timestamp() as f64;
        }
        
        let path = v4_dir.join(format!("{}.jcross", node.key));
        let custom_markup = node.to_jcross();
        fs::write(&path, custom_markup).await.map_err(RoninError::Io)?;

        self.nodes.insert(node.key.clone(), node);
        Ok(())
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Legacy Compat Hooks (Will deprecate slowly)
    // ─────────────────────────────────────────────────────────────────────────────

    pub fn front_nodes(&self) -> Vec<&MemoryNode> {
        self.nodes.values()
            .filter(|n| n.zone == MemoryZone::Front || n.utility > 0.8)
            .collect()
    }
    
    pub async fn write_front(&mut self, key: &str, content: &str) -> Result<()> {
        let node = MemoryNode::new_front(key, content);
        self.write_node(node).await
    }

    pub fn front_content_string(&self) -> String {
        self.front_nodes()
            .iter()
            .map(|n| format!("[{}]: {}", n.key, n.content))
            .collect::<Vec<_>>()
            .join("\n")
    }
}
