use anyhow::{bail, Context, Result};
use std::path::Path;

use crate::detect::{self, ArtifactType};

pub fn run(path: &str, type_override: Option<&str>) -> Result<()> {
    let dir = Path::new(path);
    if !dir.is_dir() {
        bail!("Path is not a directory: {path}");
    }

    let artifact_type = match type_override {
        Some(t) => ArtifactType::from_str(t).context("Invalid artifact type")?,
        None => detect::detect(dir).context(
            "Could not detect artifact type. Ensure the directory contains the expected files \
             (SKILL.md, plugin.json, agent.json+AGENT.md, or manifest.json with artifact_type: \"app\")",
        )?,
    };

    println!("Detected type: {artifact_type}");

    match artifact_type {
        ArtifactType::Skill => validate_skill(dir)?,
        ArtifactType::Plugin => validate_plugin(dir)?,
        ArtifactType::Agent => validate_agent(dir)?,
        ArtifactType::App => validate_app(dir)?,
    }

    println!("\nValidation passed.");
    Ok(())
}

fn validate_skill(dir: &Path) -> Result<()> {
    let skill_md = read_skill_md(dir)?;
    let frontmatter = parse_frontmatter(&skill_md)?;

    // Required fields
    require_field(&frontmatter, "name", "SKILL.md")?;
    require_field(&frontmatter, "description", "SKILL.md")?;

    // Name validation
    if let Some(name) = frontmatter.get("name").and_then(|v| v.as_str()) {
        validate_name(name)?;
        // Check name matches directory
        let dir_name = dir.file_name().unwrap().to_string_lossy();
        if name != dir_name.as_ref() {
            println!("  Warning: name '{name}' doesn't match directory '{dir_name}'");
        }
    }

    // Body length check
    let body_lines = skill_md.lines().count();
    if body_lines > 500 {
        println!("  Warning: SKILL.md is {body_lines} lines (recommended < 500)");
    }

    println!("  SKILL.md: valid");
    Ok(())
}

fn validate_plugin(dir: &Path) -> Result<()> {
    // Validate plugin.json
    let plugin_json_path = dir.join("plugin.json");
    let plugin_json: serde_json::Value = read_json(&plugin_json_path)?;

    require_json_field(&plugin_json, "id", "plugin.json")?;
    require_json_field(&plugin_json, "slug", "plugin.json")?;
    require_json_field(&plugin_json, "version", "plugin.json")?;

    // Validate slug format
    if let Some(slug) = plugin_json.get("slug").and_then(|v| v.as_str()) {
        validate_name(slug)?;
    }

    // Validate version is semver
    if let Some(version) = plugin_json.get("version").and_then(|v| v.as_str()) {
        validate_semver(version)?;
    }

    // Check platforms
    if let Some(platforms) = plugin_json.get("platforms") {
        if platforms.as_object().is_none_or(|p| p.is_empty()) {
            bail!("plugin.json: platforms must have at least one entry");
        }
    }

    // Check for template variables
    let raw = std::fs::read_to_string(&plugin_json_path)?;
    if raw.contains("{{") {
        bail!("plugin.json: contains template variables ({{{{...}}}}). Hardcode all values.");
    }

    println!("  plugin.json: valid");

    // Validate PLUGIN.md if present
    if dir.join("PLUGIN.md").exists() {
        let plugin_md = std::fs::read_to_string(dir.join("PLUGIN.md"))?;
        let _ = parse_frontmatter(&plugin_md)?;
        println!("  PLUGIN.md: valid");
    }

    // Check for dist/ directory
    let dist = dir.join("dist");
    if dist.exists() {
        let platforms: Vec<_> = std::fs::read_dir(&dist)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        println!("  Binaries found for: {}", platforms.join(", "));

        if !platforms.contains(&"darwin-arm64".to_string())
            || !platforms.contains(&"linux-amd64".to_string())
        {
            println!("  Warning: missing recommended platforms (darwin-arm64, linux-amd64)");
        }
    } else {
        println!("  Warning: no dist/ directory found");
    }

    Ok(())
}

fn validate_agent(dir: &Path) -> Result<()> {
    // Validate AGENT.md
    let agent_md = read_agent_md(dir)?;
    let frontmatter = parse_frontmatter(&agent_md)?;
    require_field(&frontmatter, "name", "AGENT.md")?;
    require_field(&frontmatter, "description", "AGENT.md")?;
    println!("  AGENT.md: valid");

    // Validate agent.json
    let agent_json_path = dir.join("agent.json");
    let agent_json: serde_json::Value = read_json(&agent_json_path)?;

    // Check workflows have valid triggers
    if let Some(workflows) = agent_json.get("workflows").and_then(|v| v.as_object()) {
        for (name, binding) in workflows {
            if binding.get("trigger").is_none() {
                bail!("agent.json: workflow '{name}' missing trigger");
            }
            if let Some(trigger) = binding.get("trigger").and_then(|v| v.as_object()) {
                let trigger_type = trigger.get("type").and_then(|v| v.as_str()).unwrap_or("");
                match trigger_type {
                    "schedule" | "heartbeat" | "event" | "watch" | "manual" => {}
                    other => {
                        bail!("agent.json: workflow '{name}' has invalid trigger type: '{other}'")
                    }
                }
            }

            // Check activity ID uniqueness
            if let Some(activities) = binding.get("activities").and_then(|v| v.as_array()) {
                let mut ids: Vec<&str> = Vec::new();
                for activity in activities {
                    if let Some(id) = activity.get("id").and_then(|v| v.as_str()) {
                        if ids.contains(&id) {
                            bail!("agent.json: duplicate activity id '{id}' in workflow '{name}'");
                        }
                        ids.push(id);
                    }
                }
            }
        }
    }

    println!("  agent.json: valid");

    // Warn if manifest.json exists (common mistake source)
    if dir.join("manifest.json").exists() {
        println!("  Note: manifest.json found. Remember: only agent.json is uploaded as config.");
    }

    Ok(())
}

fn validate_app(dir: &Path) -> Result<()> {
    // Validate manifest.json
    let manifest_path = dir.join("manifest.json");
    let manifest: serde_json::Value = read_json(&manifest_path)?;

    require_json_field(&manifest, "id", "manifest.json")?;
    require_json_field(&manifest, "name", "manifest.json")?;
    require_json_field(&manifest, "version", "manifest.json")?;

    // Accept either "type" (documented) or "artifact_type" (legacy).
    let kind = manifest
        .get("type")
        .and_then(|v| v.as_str())
        .or_else(|| manifest.get("artifact_type").and_then(|v| v.as_str()));
    if kind != Some("app") {
        bail!("manifest.json: type must be \"app\"");
    }

    println!("  manifest.json: valid");

    // Validate AGENT.md
    if has_file(dir, "AGENT.md") || has_file(dir, "agent.md") {
        let agent_md = read_agent_md(dir)?;
        let _ = parse_frontmatter(&agent_md)?;
        println!("  AGENT.md: valid");
    } else {
        bail!("Apps require an AGENT.md file");
    }

    // Check ui/index.html
    if !dir.join("ui").join("index.html").exists() {
        bail!("Apps require ui/index.html");
    }
    println!("  ui/index.html: present");

    // Validate agent.json if present
    if dir.join("agent.json").exists() {
        let _: serde_json::Value = read_json(&dir.join("agent.json"))?;
        println!("  agent.json: valid");
    }

    Ok(())
}

// --- Helpers ---

fn read_skill_md(dir: &Path) -> Result<String> {
    let path = if dir.join("SKILL.md").exists() {
        dir.join("SKILL.md")
    } else {
        dir.join("skill.md")
    };
    std::fs::read_to_string(&path).context("Failed to read SKILL.md")
}

fn read_agent_md(dir: &Path) -> Result<String> {
    let path = if dir.join("AGENT.md").exists() {
        dir.join("AGENT.md")
    } else {
        dir.join("agent.md")
    };
    std::fs::read_to_string(&path).context("Failed to read AGENT.md")
}

fn read_json(path: &Path) -> Result<serde_json::Value> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    serde_json::from_str(&content).with_context(|| format!("Invalid JSON in {}", path.display()))
}

fn parse_frontmatter(content: &str) -> Result<serde_yaml::Value> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        bail!("Missing YAML frontmatter (must start with ---)");
    }
    let after_first = &trimmed[3..];
    let end = after_first
        .find("\n---")
        .context("Missing closing --- for frontmatter")?;
    let yaml_str = &after_first[..end];
    serde_yaml::from_str(yaml_str).context("Invalid YAML in frontmatter")
}

fn require_field(yaml: &serde_yaml::Value, field: &str, file: &str) -> Result<()> {
    if yaml.get(field).is_none() {
        bail!("{file}: missing required field '{field}'");
    }
    Ok(())
}

fn require_json_field(json: &serde_json::Value, field: &str, file: &str) -> Result<()> {
    if json.get(field).is_none() {
        bail!("{file}: missing required field '{field}'");
    }
    Ok(())
}

fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() || name.len() > 64 {
        bail!("Name must be 1-64 characters, got {}", name.len());
    }
    if name.starts_with('-') || name.ends_with('-') {
        bail!("Name must not start or end with a hyphen: '{name}'");
    }
    if name.contains("--") {
        bail!("Name must not contain consecutive hyphens: '{name}'");
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        bail!("Name must be lowercase alphanumeric + hyphens only: '{name}'");
    }
    Ok(())
}

fn validate_semver(version: &str) -> Result<()> {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() != 3 {
        bail!("Version must be semver (x.y.z): '{version}'");
    }
    for part in parts {
        if part.parse::<u32>().is_err() {
            bail!("Version must be semver (x.y.z): '{version}'");
        }
    }
    Ok(())
}

fn has_file(dir: &Path, name: &str) -> bool {
    dir.join(name).exists()
}
