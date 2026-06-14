use anyhow::{bail, Context, Result};
use std::path::Path;

use crate::api;
use crate::auth;
use crate::detect::{self, ArtifactType};
use crate::validate;

/// Ensures the user is authenticated before publishing.
/// If not authenticated, automatically starts the login flow — zero friction.
async fn ensure_authenticated() -> Result<()> {
    match auth::load_credentials()? {
        Some(creds) if !creds.is_expired() => {
            // Already authenticated
            Ok(())
        }
        Some(_) => {
            // Token expired — auto-refresh or re-login
            println!("Session expired. Logging in...");
            auth::login().await
        }
        None => {
            // Never authenticated — start login automatically
            println!("First time publishing — let's get you authenticated.");
            println!();
            auth::login().await
        }
    }
}

const PLATFORMS: &[&str] = &[
    "darwin-arm64",
    "darwin-amd64",
    "linux-arm64",
    "linux-amd64",
    "windows-amd64",
];

pub async fn run(
    path: &str,
    type_override: Option<&str>,
    visibility: &str,
    _resume: bool,
) -> Result<()> {
    let dir = Path::new(path);
    if !dir.is_dir() {
        bail!("Path is not a directory: {path}");
    }

    // Check auth before doing anything — auto-login if needed
    ensure_authenticated().await?;

    // Validate first
    println!("Validating...");
    validate::run(path, type_override)?;

    let artifact_type = match type_override {
        Some(t) => ArtifactType::from_str(t).unwrap(),
        None => detect::detect(dir).unwrap(),
    };

    println!("\nPublishing as {artifact_type}...");

    // Resolve the developer account to publish under (required by the create
    // endpoint). Honors $NEBOAI_ACCOUNT (a slug) if set, else the first account.
    let account_slug = std::env::var("NEBOAI_ACCOUNT").ok();
    let account_id = api::resolve_account(account_slug.as_deref()).await?;

    match artifact_type {
        ArtifactType::Skill => publish_skill(dir, &account_id, visibility).await?,
        ArtifactType::Plugin => publish_plugin(dir, &account_id, visibility).await?,
        ArtifactType::Agent => publish_agent(dir, &account_id, visibility).await?,
        ArtifactType::App => publish_app(dir, &account_id, visibility).await?,
    }

    Ok(())
}

async fn publish_skill(dir: &Path, account_id: &str, visibility: &str) -> Result<()> {
    let skill_md = read_file(dir, "SKILL.md")?;
    let fm = extract_frontmatter_fields(&skill_md)?;
    let name = fm.name;
    let version = fm.version.unwrap_or_else(|| "1.0.0".to_string());
    let category = category_display_name(fm.category.as_deref().unwrap_or(""));
    let description = cap_description(&fm.description);

    println!("Creating skill: {name}");
    let id = api::create_artifact(account_id, &name, "skill", category, &description, &version, visibility, &skill_md).await?;
    println!("  Artifact ID: {id}");

    // Upload the whole directory as a bundle so references/, scripts/, and
    // assets/ ship alongside SKILL.md. The server re-extracts SKILL.md into the
    // manifest, so this is safe (and a no-op in effect) for single-file skills.
    let file_count = api::upload_bundle(&id, dir).await?;
    println!("  Uploaded bundle ({file_count} files: SKILL.md + references/scripts/assets)");

    finalize(&id, &name, "Skill", &version, visibility).await?;
    Ok(())
}

async fn publish_plugin(dir: &Path, account_id: &str, visibility: &str) -> Result<()> {
    let plugin_md = read_file(dir, "PLUGIN.md")?;
    let plugin_json_path = dir.join("plugin.json");
    let plugin_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&plugin_json_path)?)?;

    let name = plugin_json
        .get("slug")
        .or_else(|| plugin_json.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("unnamed")
        .to_string();
    let version = plugin_json
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("1.0.0")
        .to_string();
    let category = category_display_name(
        plugin_json.get("category").and_then(|v| v.as_str()).unwrap_or(""),
    );
    // Description from PLUGIN.md frontmatter (falls back to plugin.json), capped at 480 chars.
    let fm = extract_frontmatter_fields(&plugin_md).ok();
    let description = fm
        .as_ref()
        .map(|f| f.description.clone())
        .filter(|d| !d.is_empty())
        .or_else(|| plugin_json.get("description").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .unwrap_or_else(|| format!("{name} — NeboAI plugin"));
    let description = cap_description(&description);

    println!("Creating plugin: {name}");
    let id = api::create_artifact(account_id, &name, "plugin", category, &description, &version, visibility, &plugin_md).await?;
    println!("  Artifact ID: {id}");

    // Build skills tarball if skills/ exists
    let skills_tarball = if dir.join("skills").exists() {
        let tarball_path = std::env::temp_dir().join(format!("neboai-{name}-skills.tar.gz"));
        build_skills_tarball(dir, &tarball_path)?;
        println!("  Skills tarball built");
        Some(tarball_path)
    } else {
        None
    };

    // Upload available platform binaries (config + skills on the first one).
    let dist_dir = dir.join("dist").join("plugin");
    let mut first = true;
    for platform in PLATFORMS {
        let platform_dir = dist_dir.join(platform);
        if !platform_dir.exists() {
            continue;
        }
        let binary_path = find_binary(&platform_dir)?;
        api::upload_binary(
            &id,
            platform,
            Some(&binary_path),
            &dir.join("PLUGIN.md"),
            if first { Some(&plugin_json_path) } else { None },
            if first { skills_tarball.as_deref() } else { None },
        )
        .await?;
        first = false;
    }

    if first {
        bail!("No platform binaries found in dist/plugin/. Run ./build.sh first. Expected at least one of: {PLATFORMS:?}");
    }

    finalize(&id, &name, "Plugin", &version, visibility).await?;
    Ok(())
}

async fn publish_agent(dir: &Path, account_id: &str, visibility: &str) -> Result<()> {
    let agent_md = read_file(dir, "AGENT.md")?;
    let agent_json_path = dir.join("agent.json");
    let fm = extract_frontmatter_fields(&agent_md)?;
    let name = fm.name;
    let version = fm.version.unwrap_or_else(|| "1.0.0".to_string());
    let category = category_display_name(fm.category.as_deref().unwrap_or(""));
    let description = cap_description(&fm.description);

    println!("Creating agent: {name}");
    let id = api::create_artifact(account_id, &name, "agent", category, &description, &version, visibility, &agent_md).await?;
    println!("  Artifact ID: {id}");

    api::upload_binary(
        &id,
        "linux-amd64", // Required field, but agents aren't platform-specific
        None,          // No binary file for agents
        &dir.join("AGENT.md"),
        Some(&agent_json_path),
        None,
    )
    .await?;

    finalize(&id, &name, "Agent", &version, visibility).await?;
    Ok(())
}

async fn publish_app(dir: &Path, account_id: &str, visibility: &str) -> Result<()> {
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
    let fm = extract_frontmatter_fields(&agent_md).ok();
    let description = fm
        .as_ref()
        .map(|f| f.description.clone())
        .filter(|d| !d.is_empty())
        .or_else(|| manifest.get("description").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .unwrap_or_else(|| format!("{name} — NeboAI app"));
    let description = cap_description(&description);
    let category = category_display_name(
        manifest.get("category").and_then(|v| v.as_str()).unwrap_or(""),
    );

    println!("Creating app: {name}");
    let id = api::create_artifact(account_id, &name, "agent", category, &description, &version, visibility, &agent_md).await?;
    println!("  Artifact ID: {id}");

    let config_path = if dir.join("agent.json").exists() {
        Some(dir.join("agent.json"))
    } else {
        None
    };

    api::upload_binary(
        &id,
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
            api::upload_binary(
                &id,
                current_platform(),
                Some(&binary),
                &dir.join("AGENT.md"),
                None,
                None,
            )
            .await?;
        }
    }

    finalize(&id, &name, "App", &version, visibility).await?;
    Ok(())
}

// --- Helpers ---

/// Finish a publish: submit for review when going public, otherwise leave the
/// artifact unlisted (private/loop have nothing to review). Keeps the
/// create → bundle/upload → finalize shape identical across artifact types.
async fn finalize(id: &str, name: &str, kind: &str, version: &str, visibility: &str) -> Result<()> {
    if visibility == "public" {
        println!("Submitting v{version} for review...");
        api::submit(id, version).await?;
        println!("\nDone! {kind} '{name}' submitted for review.");
    } else {
        println!("\nDone! {kind} '{name}' published ({visibility}). Not submitted for review.");
    }
    Ok(())
}

/// Marketplace submission caps the description at 500 characters. The skill/agent
/// frontmatter `description` doubles as the trigger text and is often longer, so
/// cap it here (counting chars, not bytes) before sending it to create/submit.
const MAX_DESCRIPTION: usize = 500;

/// Truncate `desc` to at most MAX_DESCRIPTION characters, cutting on a word
/// boundary and appending an ellipsis when it has to shorten. The full text
/// still lives in the manifest's frontmatter; this only trims the marketplace
/// metadata field.
fn cap_description(desc: &str) -> String {
    let desc = desc.trim();
    if desc.chars().count() <= MAX_DESCRIPTION {
        return desc.to_string();
    }
    // Reserve one char for the ellipsis so the result stays within the cap.
    let limit = MAX_DESCRIPTION - 1;
    let truncated: String = desc.chars().take(limit).collect();
    let body = match truncated.rfind(char::is_whitespace) {
        // Only snap back to a word boundary if it doesn't lose too much text.
        Some(idx) if idx >= limit / 2 => &truncated[..idx],
        _ => &truncated,
    };
    format!("{}…", body.trim_end())
}

struct FrontmatterFields {
    name: String,
    version: Option<String>,
    description: String,
    category: Option<String>,
}

/// Map a category slug (as used in plugin.json / frontmatter) to the marketplace
/// display name the create endpoint expects. Unknown slugs fall back to "Build & connect".
fn category_display_name(slug: &str) -> &'static str {
    match slug {
        "business" => "Run your business",
        "content" => "Create content",
        "customers" => "Find customers",
        "money" => "Manage money",
        "organized" => "Get organized",
        "communicate" | "communication" => "Communicate",
        "learn" => "Learn & grow",
        "research" => "Research & decide",
        "documents" => "Handle documents",
        _ => "Build & connect",
    }
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

    let description = yaml
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let category = yaml
        .get("category")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let version = yaml
        .get("metadata")
        .and_then(|m| m.get("version"))
        .and_then(|v| v.as_str())
        .or_else(|| yaml.get("version").and_then(|v| v.as_str()))
        .map(|s| s.to_string());

    Ok(FrontmatterFields {
        name,
        version,
        description,
        category,
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_description_is_unchanged() {
        let d = "A concise skill description.";
        assert_eq!(cap_description(d), d);
    }

    #[test]
    fn long_description_is_capped_within_limit() {
        let d = "word ".repeat(200); // 1000 chars
        let out = cap_description(&d);
        assert!(out.chars().count() <= MAX_DESCRIPTION);
        assert!(out.ends_with('…'));
    }

    #[test]
    fn cap_respects_char_boundaries_for_multibyte() {
        let d = "é".repeat(600); // 600 chars, 1200 bytes
        let out = cap_description(&d);
        assert!(out.chars().count() <= MAX_DESCRIPTION);
        // Must not panic and must be valid UTF-8 (guaranteed by String).
        assert!(out.ends_with('…'));
    }
}
