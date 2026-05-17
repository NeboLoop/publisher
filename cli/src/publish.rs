use anyhow::{bail, Context, Result};
use std::path::Path;

use crate::api;
use crate::detect::{self, ArtifactType};
use crate::validate;

const PLATFORMS: &[&str] = &[
    "darwin-arm64",
    "darwin-amd64",
    "linux-arm64",
    "linux-amd64",
    "windows-amd64",
];

pub async fn run(path: &str, type_override: Option<&str>, _resume: bool) -> Result<()> {
    let dir = Path::new(path);
    if !dir.is_dir() {
        bail!("Path is not a directory: {path}");
    }

    // Validate first
    println!("Validating...");
    validate::run(path, type_override)?;

    let artifact_type = match type_override {
        Some(t) => ArtifactType::from_str(t).unwrap(),
        None => detect::detect(dir).unwrap(),
    };

    println!("\nPublishing as {artifact_type}...");

    match artifact_type {
        ArtifactType::Skill => publish_skill(dir).await?,
        ArtifactType::Plugin => publish_plugin(dir).await?,
        ArtifactType::Agent => publish_agent(dir).await?,
        ArtifactType::App => publish_app(dir).await?,
    }

    Ok(())
}

async fn publish_skill(dir: &Path) -> Result<()> {
    let skill_md = read_file(dir, "SKILL.md")?;
    let frontmatter = extract_frontmatter_fields(&skill_md)?;
    let name = frontmatter.name;
    let version = frontmatter.version.unwrap_or_else(|| "1.0.0".to_string());

    println!("Creating/updating skill: {name}");

    // Create or update
    let id = api::create_artifact(&name, "productivity", &skill_md).await?;
    println!("  Artifact ID: {id}");

    // Submit
    println!("Submitting v{version} for review...");
    api::submit(&id, &version).await?;

    println!("\nDone! Skill '{name}' submitted for review.");
    Ok(())
}

async fn publish_plugin(dir: &Path) -> Result<()> {
    let plugin_md = read_file(dir, "PLUGIN.md")?;
    let plugin_json_path = dir.join("plugin.json");
    let plugin_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&plugin_json_path)?)?;

    let name = plugin_json
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("unnamed")
        .to_string();
    let version = plugin_json
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("1.0.0")
        .to_string();

    println!("Creating/updating plugin: {name}");

    let id = api::create_artifact(&name, "connectors", &plugin_md).await?;
    println!("  Artifact ID: {id}");

    // Build skills tarball if skills/ exists
    let skills_tarball = if dir.join("skills").exists() {
        let tarball_path = std::env::temp_dir().join("neboai-skills.tar.gz");
        build_skills_tarball(dir, &tarball_path)?;
        println!("  Skills tarball built");
        Some(tarball_path)
    } else {
        None
    };

    // Get upload token
    let upload_token = api::get_upload_token(&id).await?;

    // Find available platform binaries
    let dist_dir = dir.join("dist");
    let mut first = true;

    for platform in PLATFORMS {
        let platform_dir = dist_dir.join(platform);
        if !platform_dir.exists() {
            continue;
        }

        // Find binary in platform directory
        let binary_path = find_binary(&platform_dir)?;

        api::upload_binary(
            &id,
            &upload_token,
            platform,
            Some(&binary_path),
            &dir.join("PLUGIN.md"),
            if first { Some(&plugin_json_path) } else { None },
            if first {
                skills_tarball.as_deref()
            } else {
                None
            },
        )
        .await?;

        first = false;
    }

    if first {
        bail!("No platform binaries found in dist/. Expected at least one of: {PLATFORMS:?}");
    }

    // Submit
    println!("Submitting v{version} for review...");
    api::submit(&id, &version).await?;

    println!("\nDone! Plugin '{name}' submitted for review.");
    Ok(())
}

async fn publish_agent(dir: &Path) -> Result<()> {
    let agent_md = read_file(dir, "AGENT.md")?;
    let agent_json_path = dir.join("agent.json");
    let frontmatter = extract_frontmatter_fields(&agent_md)?;
    let name = frontmatter.name;
    let version = frontmatter
        .version
        .unwrap_or_else(|| "1.0.0".to_string());

    println!("Creating/updating agent: {name}");

    let id = api::create_artifact(&name, "productivity", &agent_md).await?;
    println!("  Artifact ID: {id}");

    // Get upload token and upload agent.json
    let upload_token = api::get_upload_token(&id).await?;

    api::upload_binary(
        &id,
        &upload_token,
        "linux-amd64", // Required but agents aren't platform-specific
        None,          // No binary file for agents
        &dir.join("AGENT.md"),
        Some(&agent_json_path),
        None,
    )
    .await?;

    // Submit
    println!("Submitting v{version} for review...");
    api::submit(&id, &version).await?;

    println!("\nDone! Agent '{name}' submitted for review.");
    Ok(())
}

async fn publish_app(dir: &Path) -> Result<()> {
    let agent_md = read_file(dir, "AGENT.md")?;
    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("manifest.json"))?)?;

    let name = manifest
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("unnamed")
        .to_string();
    let version = manifest
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("1.0.0")
        .to_string();

    println!("Creating/updating app: {name}");

    let id = api::create_artifact(&name, "apps", &agent_md).await?;
    println!("  Artifact ID: {id}");

    // Upload agent.json if present
    let upload_token = api::get_upload_token(&id).await?;

    let config_path = if dir.join("agent.json").exists() {
        Some(dir.join("agent.json"))
    } else {
        None
    };

    api::upload_binary(
        &id,
        &upload_token,
        "linux-amd64",
        None,
        &dir.join("AGENT.md"),
        config_path.as_deref(),
        None,
    )
    .await?;

    // Upload sidecar binaries if present
    let sidecar_dist = dir.join("sidecar").join("target").join("release");
    if sidecar_dist.exists() {
        println!("  Uploading sidecar binary...");
        if let Ok(binary) = find_binary(&sidecar_dist) {
            // For sidecar, upload for the current platform
            api::upload_binary(
                &id,
                &upload_token,
                current_platform(),
                Some(&binary),
                &dir.join("AGENT.md"),
                None,
                None,
            )
            .await?;
        }
    }

    // Submit
    println!("Submitting v{version} for review...");
    api::submit(&id, &version).await?;

    println!("\nDone! App '{name}' submitted for review.");
    Ok(())
}

// --- Helpers ---

struct FrontmatterFields {
    name: String,
    version: Option<String>,
}

fn extract_frontmatter_fields(content: &str) -> Result<FrontmatterFields> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        bail!("Missing frontmatter");
    }
    let after = &trimmed[3..];
    let end = after.find("\n---").context("Missing closing ---")?;
    let yaml: serde_yaml::Value = serde_yaml::from_str(&after[..end])?;

    let name = yaml
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("unnamed")
        .to_string();

    let version = yaml
        .get("metadata")
        .and_then(|m| m.get("version"))
        .and_then(|v| v.as_str())
        .or_else(|| yaml.get("version").and_then(|v| v.as_str()))
        .map(|s| s.to_string());

    Ok(FrontmatterFields { name, version })
}

fn read_file(dir: &Path, name: &str) -> Result<String> {
    let path = dir.join(name);
    let alt = dir.join(name.to_lowercase());
    let actual = if path.exists() { path } else { alt };
    std::fs::read_to_string(&actual).with_context(|| format!("Failed to read {name}"))
}

fn find_binary(dir: &Path) -> Result<std::path::PathBuf> {
    let entries: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .collect();

    if entries.len() == 1 {
        return Ok(entries[0].path());
    }

    // Look for file without extension (the binary)
    for entry in &entries {
        let path = entry.path();
        if path.extension().is_none() {
            return Ok(path);
        }
    }

    // Fallback: first file
    entries
        .first()
        .map(|e| e.path())
        .context("No binary found in directory")
}

fn build_skills_tarball(dir: &Path, output: &Path) -> Result<()> {
    let skills_dir = dir.join("skills");
    let file = std::fs::File::create(output)?;
    let enc = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut tar = tar::Builder::new(enc);
    tar.append_dir_all("skills", &skills_dir)?;
    tar.finish()?;
    Ok(())
}

fn current_platform() -> &'static str {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return "darwin-arm64";
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return "darwin-amd64";
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    return "linux-arm64";
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return "linux-amd64";
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    return "windows-amd64";
    #[cfg(not(any(
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "windows", target_arch = "x86_64"),
    )))]
    return "linux-amd64";
}
