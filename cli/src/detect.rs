use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ArtifactType {
    Skill,
    Plugin,
    Agent,
    App,
    Connector,
    Collection,
}

impl std::fmt::Display for ArtifactType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArtifactType::Skill => write!(f, "skill"),
            ArtifactType::Plugin => write!(f, "plugin"),
            ArtifactType::Agent => write!(f, "agent"),
            ArtifactType::App => write!(f, "app"),
            ArtifactType::Connector => write!(f, "connector"),
            ArtifactType::Collection => write!(f, "collection"),
        }
    }
}

impl ArtifactType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "skill" => Some(Self::Skill),
            "plugin" => Some(Self::Plugin),
            "agent" => Some(Self::Agent),
            "app" => Some(Self::App),
            "connector" => Some(Self::Connector),
            "collection" => Some(Self::Collection),
            _ => None,
        }
    }
}

/// Detect artifact type from directory contents.
///
/// Detection order (first match wins):
/// 1. manifest.json with artifact_type: "app" → App
/// 2. plugin.json → Plugin
/// 3. agent.json + AGENT.md → Agent
/// 4. SKILL.md → Skill
pub fn detect(path: &Path) -> Option<ArtifactType> {
    // Check for App (manifest.json with type/artifact_type == "app").
    // Accept both keys: docs use "type", older manifests use "artifact_type".
    let manifest_path = path.join("manifest.json");
    if manifest_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&manifest_path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                let is_app = json.get("type").and_then(|v| v.as_str()) == Some("app")
                    || json.get("artifact_type").and_then(|v| v.as_str()) == Some("app");
                if is_app {
                    return Some(ArtifactType::App);
                }
            }
        }
    }

    // Check for Plugin (plugin.json present)
    if path.join("plugin.json").exists() {
        return Some(ArtifactType::Plugin);
    }

    // Check for Connector (connector.json — an MCP `mcpServers` config block)
    if path.join("connector.json").exists() {
        return Some(ArtifactType::Connector);
    }

    // Check for Collection (collection.json — a bundle of existing artifacts)
    if path.join("collection.json").exists() {
        return Some(ArtifactType::Collection);
    }

    // Check for Agent (agent.json + AGENT.md)
    if path.join("agent.json").exists() && has_agent_md(path) {
        return Some(ArtifactType::Agent);
    }

    // Check for Skill (SKILL.md)
    if has_skill_md(path) {
        return Some(ArtifactType::Skill);
    }

    None
}

fn has_agent_md(path: &Path) -> bool {
    // Case-insensitive check
    path.join("AGENT.md").exists() || path.join("agent.md").exists()
}

fn has_skill_md(path: &Path) -> bool {
    // Case-insensitive check
    path.join("SKILL.md").exists() || path.join("skill.md").exists()
}
