use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Defines the operational effect a single Kanji Label exerts over the spatial physics of memory nodes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KanjiOp {
    pub name: String,
    pub gravity_mod: f32,       // Additive modifier to core spatial gravity (e.g., Dense +0.5)
    pub decay_mod: f32,         // Multiplicative modifier to time decay (e.g., Eternal *0.0)
    pub radius_mod: f32,        // Multiplicative modifier to query radius space
    pub is_purge_flag: bool,    // If true, this operator rejects the node from safe processing (e.g. Broken)
}

impl KanjiOp {
    pub fn new(name: &str, gravity: f32, decay: f32, radius: f32, purge: bool) -> Self {
        Self {
            name: name.to_string(),
            gravity_mod: gravity,
            decay_mod: decay,
            radius_mod: radius,
            is_purge_flag: purge,
        }
    }

    /// Retrieve the standard Kanji ontology global vocabulary dictionary
    pub fn standard_ontology() -> HashMap<String, KanjiOp> {
        let mut map = HashMap::new();
        // 空間特性 (Spatial Density / Scale)
        map.insert("密".to_string(), Self::new("密", 0.5, 1.0, 1.0, false));
        map.insert("疎".to_string(), Self::new("疎", -0.3, 1.0, 1.0, false));
        map.insert("巨".to_string(), Self::new("巨", 0.2, 1.0, 2.0, false));
        map.insert("微".to_string(), Self::new("微", -0.1, 1.0, 0.5, false));

        // 時間・鮮度特性 (Time / Decay Characteristics)
        map.insert("古".to_string(), Self::new("古", -0.2, 2.0, 1.0, false)); // Harder decay
        map.insert("新".to_string(), Self::new("新", 0.3, 1.0, 1.0, false));
        map.insert("恒".to_string(), Self::new("恒", 0.0, 0.0, 1.0, false)); // Eternal decay = 0

        // 信頼・状態特性 (Confidence / Utility Characteristics)
        map.insert("確".to_string(), Self::new("確", 0.4, 0.8, 1.0, false)); // Fact, degrades slower
        map.insert("疑".to_string(), Self::new("疑", -0.4, 1.5, 1.0, false)); // Hypothesis, degrades fast
        map.insert("破".to_string(), Self::new("破", -1.0, 5.0, 0.0, true)); // Reject / Purge
        map.insert("完".to_string(), Self::new("完", -0.3, 1.5, 1.0, false)); // Completed, lose priority

        // 行動・感情特性 (Reflex Characteristics)
        map.insert("緊".to_string(), Self::new("緊", 2.0, 0.0, 3.0, false)); // Urgent, massive pull, no decay
        map.insert("創".to_string(), Self::new("創", 0.0, 1.0, 2.5, false)); // Broadens search radius
        map.insert("反".to_string(), Self::new("反", 0.5, 0.5, 1.0, false)); // Reflect on failure

        map
    }
}

/// A parsed Tag holding its internal value constraint (e.g. `[密:0.8]`)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KanjiTag {
    pub name: String,
    pub weight: f32, // The 0.0 to 1.0 coefficient for continuous operations
}

impl KanjiTag {
    /// Parse raw text string like `[密:0.8]` or `密8` or `密` -> returns name and float weight
    pub fn parse(raw: &str) -> Option<Self> {
        let clean = raw.trim_matches(|c| c == '[' || c == ']' || c == ' ' || c == '【' || c == '】');
        if clean.is_empty() {
            return None;
        }
        
        let parts: Vec<&str> = clean.split(':').collect();
        if parts.len() == 2 {
            let name = parts[0].trim().to_string();
            let weight = parts[1].parse::<f32>().unwrap_or(1.0);
            return Some(Self { name, weight });
        }
        
        // Handle no colon fallback (e.g., `密` -> 1.0)
        let name = clean.trim().to_string();
        Some(Self { name, weight: 1.0 })
    }
}

/// Represents the Type and Magnitude of an edge connected to another Memory Node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypedRelation {
    pub target_id: String,
    pub rel_type: RelationType,
    pub strength: f32, // 0.0 - 1.0 continuously
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RelationType {
    Derived, // 派生
    Base,    // 基底
    Similar, // 類似
    Opposite,// 対立
    Prev,    // 前項
    Next,    // 次項
    Cause,   // 因果
    Fix,     // 訂正
    Context, // 補足
    Unknown(String),
}

impl RelationType {
    pub fn from_str(val: &str) -> Self {
        match val {
            "派生" | "derived" => RelationType::Derived,
            "基底" | "base" => RelationType::Base,
            "類似" | "similar" => RelationType::Similar,
            "対立" | "opposite" => RelationType::Opposite,
            "前項" | "prev" => RelationType::Prev,
            "次項" | "next" => RelationType::Next,
            "因果" | "cause" => RelationType::Cause,
            "訂正" | "fix" => RelationType::Fix,
            "補足" | "context" => RelationType::Context,
            other => RelationType::Unknown(other.to_string()),
        }
    }
}
