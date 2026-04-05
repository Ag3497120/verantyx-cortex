//! JCross spatial memory index.
//!
//! Reads and parses the `.jcross` topology files created by the TypeScript
//! memory engine, enabling the Rust core to consume JCross memory nodes
//! without requiring a full port of the spatial graph logic.
//!
//! Zone hierarchy: Front → Near → Mid → Deep (hottest to coldest).

use crate::domain::error::{Result, RoninError};
use crate::domain::types::MemoryZone;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, info};

// ─────────────────────────────────────────────────────────────────────────────
// Spatial Memory Node
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryNode {
    pub key: String,
    pub content: String,
    pub zone: MemoryZone,
    pub created_at: DateTime<Utc>,
    pub last_accessed: Option<DateTime<Utc>>,
    pub tags: Vec<String>,
    pub weight: f32,
}

impl MemoryNode {
    /// Creates a new Front-zone memory node with default weight.
    pub fn new_front(key: &str, content: &str) -> Self {
        Self {
            key: key.to_string(),
            content: content.to_string(),
            zone: MemoryZone::Front,
            created_at: Utc::now(),
            last_accessed: None,
            tags: vec![],
            weight: 1.0,
        }
    }

    pub fn is_hot(&self) -> bool {
        matches!(self.zone, MemoryZone::Front | MemoryZone::Near)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Spatial Index (in-memory graph)
// ─────────────────────────────────────────────────────────────────────────────

pub struct SpatialIndex {
    root: PathBuf,
    nodes: HashMap<String, MemoryNode>,
}

impl SpatialIndex {
    pub fn new(root: PathBuf) -> Self {
        Self { root, nodes: HashMap::new() }
    }

    /// Loads all nodes from the four zone directories under `root`.
    pub async fn hydrate(&mut self) -> Result<usize> {
        let zones = [
            ("front", MemoryZone::Front),
            ("near", MemoryZone::Near),
            ("mid", MemoryZone::Mid),
            ("deep", MemoryZone::Deep),
        ];

        let mut total = 0;

        for (zone_dir, zone) in &zones {
            let zone_path = self.root.join(zone_dir);
            if !zone_path.exists() {
                continue;
            }

            let mut entries = fs::read_dir(&zone_path).await.map_err(RoninError::Io)?;
            while let Some(entry) = entries.next_entry().await.map_err(RoninError::Io)? {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("md") {
                    let content = fs::read_to_string(&path).await.map_err(RoninError::Io)?;
                    let key = path.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string();

                    let node = MemoryNode {
                        key: key.clone(),
                        content,
                        zone: *zone,
                        created_at: Utc::now(),
                        last_accessed: None,
                        tags: vec![],
                        weight: zone_weight(zone),
                    };

                    self.nodes.insert(key, node);
                    total += 1;
                }
            }
        }

        info!("[SpatialIndex] Hydrated {} nodes from {}", total, self.root.display());
        Ok(total)
    }

    /// Returns all nodes in the Front zone for hot context injection.
    pub fn front_nodes(&self) -> Vec<&MemoryNode> {
        self.nodes.values()
            .filter(|n| n.zone == MemoryZone::Front)
            .collect()
    }

    /// Builds a flat string of all Front-zone contents for system prompt injection.
    pub fn front_content_string(&self) -> String {
        let mut nodes: Vec<&MemoryNode> = self.front_nodes();
        nodes.sort_by(|a, b| b.weight.partial_cmp(&a.weight).unwrap());
        nodes.iter()
            .map(|n| format!("### {}\n{}", n.key, n.content))
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Writes a new node to the Front zone and persists it to disk.
    pub async fn write_front(&mut self, key: &str, content: &str) -> Result<()> {
        let zone_dir = self.root.join("front");
        fs::create_dir_all(&zone_dir).await.map_err(RoninError::Io)?;

        let path = zone_dir.join(format!("{}.md", key));
        fs::write(&path, content).await.map_err(RoninError::Io)?;

        let node = MemoryNode::new_front(key, content);
        self.nodes.insert(key.to_string(), node);
        debug!("[SpatialIndex] Wrote front node: {}", key);
        Ok(())
    }

    pub fn find_by_tag(&self, tag: &str) -> Vec<&MemoryNode> {
        self.nodes.values()
            .filter(|n| n.tags.iter().any(|t| t == tag))
            .collect()
    }

    pub fn total_nodes(&self) -> usize {
        self.nodes.len()
    }
}

fn zone_weight(zone: &MemoryZone) -> f32 {
    match zone {
        MemoryZone::Front => 1.0,
        MemoryZone::Near  => 0.7,
        MemoryZone::Mid   => 0.4,
        MemoryZone::Deep  => 0.1,
    }
}
