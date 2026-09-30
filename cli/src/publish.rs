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
    "windows-arm64",
];

pub async fn run(path: &str, type_override: Option<&str>, visibility: &str) -> Result<()> {
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

    // The publisher account to publish under. Your personal account is set up
    // for free on first use; $NEBOAI_ACCOUNT (a handle) picks a team account.
    let account_slug = std::env::var("NEBOAI_ACCOUNT").ok();
    let account = api::resolve_account(account_slug.as_deref()).await?;
    println!("Publisher: @{}", account.slug);

    match artifact_type {
        ArtifactType::Skill => publish_skill(dir, &account.id, visibility).await?,
        ArtifactType::Plugin => publish_plugin(dir, &account.id, visibility).await?,
        ArtifactType::Agent => publish_agent(dir, &account.id, visibility).await?,
        ArtifactType::App => publish_app(dir, &account.id, visibility).await?,
        ArtifactType::Connector => publish_connector(dir, &account.id, visibility).await?,
        ArtifactType::Collection => publish_collection(dir, &account, visibility).await?,
    }

    Ok(())
}

/// What `upsert` did: the item's ID and, for an item that already existed,
/// its status before this publish.
struct Upserted {
    id: String,
    previous_status: Option<String>,
}

/// Create the item, or — when you already published one with this name and
/// type — update it to this version. Publishing is the same command either way.
#[allow(clippy::too_many_arguments)]
async fn upsert(
    account_id: &str,
    name: &str,
    artifact_type: &str,
    category: &str,
    description: &str,
    version: &str,
    visibility: &str,
    manifest: &str,
) -> Result<Upserted> {
    if visibility == "public" && description.chars().count() < 10 {
        bail!("A public listing needs a description of at least 10 characters.");
    }
    match api::find_artifact(name, artifact_type).await? {
        Some(existing) => {
            println!("Updating {artifact_type}: {name} -> v{version}");
            // The version is set before any upload: uploads are recorded
            // under the item's current version.
            api::update_manifest(&existing.id, manifest, version, description).await?;
            println!("  Artifact ID: {}", existing.id);
            Ok(Upserted {
                id: existing.id,
                previous_status: Some(existing.status),
            })
        }
        None => {
            println!("Creating {artifact_type}: {name}");
            let id = api::create_artifact(
                account_id, name, artifact_type, category, description, version, visibility, manifest,
            )
            .await?;
            println!("  Artifact ID: {id}");
            Ok(Upserted {
                id,
                previous_status: None,
            })
        }
    }
}

async fn publish_collection(dir: &Path, account: &api::Account, visibility: &str) -> Result<()> {
    // A collection bundles existing artifacts. collection.json carries the
    // metadata plus an `items` array of {targetId, targetType}.
    let raw = read_file(dir, "collection.json")?;
    let json: serde_json::Value =
        serde_json::from_str(&raw).context("collection.json is not valid JSON")?;
    let name = json
        .get("name")
        .and_then(|v| v.as_str())
        .context("collection.json must have a 'name'")?
        .to_string();
    let description = cap_description(json.get("description").and_then(|v| v.as_str()).unwrap_or(""));
    let version = json
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("1.0.0")
        .to_string();
    let title = json.get("title").and_then(|v| v.as_str());
    let items = json.get("items").and_then(|v| v.as_array()).cloned().unwrap_or_default();

    if api::find_artifact(&name, "collection").await?.is_some() {
        bail!("You already have a collection named '{name}'. Change its items on neboai.com or through the NeboAI MCP.");
    }

    println!("Creating collection: {name}");
    let id = api::create_collection(&account.namespace_id, &name, &description, visibility).await?;
    println!("  Collection ID: {id}");

    for (i, item) in items.iter().enumerate() {
        let target_id = item
            .get("targetId")
            .and_then(|v| v.as_str())
            .with_context(|| format!("collection item {i} is missing 'targetId'"))?;
        let target_type = item
            .get("targetType")
            .and_then(|v| v.as_str())
            .with_context(|| format!("collection item {i} is missing 'targetType'"))?;
        api::add_collection_item(&id, target_id, target_type, i as i64).await?;
    }
    if !items.is_empty() {
        println!("  Added {} item(s)", items.len());
    }

    apply_listing(dir, &id, &name, title).await?;
    finalize(&id, &name, "Collection", &version, visibility, None).await?;
    Ok(())
}

async fn publish_connector(dir: &Path, account_id: &str, visibility: &str) -> Result<()> {
    // A connector's manifest IS its MCP config: connector.json is the standard
    // `mcpServers` block (it may also carry name/description/category/version
    // metadata alongside — the server ignores the extra keys).
    let raw = read_file(dir, "connector.json")?;
    let json: serde_json::Value =
        serde_json::from_str(&raw).context("connector.json is not valid JSON")?;
    let servers = json
        .get("mcpServers")
        .or_else(|| json.get("servers"))
        .and_then(|s| s.as_object());
    let Some(servers) = servers.filter(|s| !s.is_empty()) else {
        bail!("connector.json must contain an 'mcpServers' (or 'servers') object with at least one server");
    };
    // Mirror the server-side validation so authors fail fast: each server is
    // stdio (`command`) or remote (`url`), and remote auth is declared via
    // `authType` — without it the desktop registers the server as
    // unauthenticated and it will fail against protected APIs.
    for (name, entry) in servers {
        let command = entry.get("command").and_then(|v| v.as_str()).unwrap_or("");
        let url = entry.get("url").and_then(|v| v.as_str()).unwrap_or("");
        if command.is_empty() && url.is_empty() {
            bail!("connector server '{name}' must have a 'command' (stdio) or 'url' (remote)");
        }
        match entry.get("authType").and_then(|v| v.as_str()).unwrap_or("") {
            "" | "none" | "oauth" | "api_key" => {}
            other => bail!(
                "connector server '{name}' has invalid authType '{other}' (must be 'none', 'oauth', or 'api_key')"
            ),
        }
    }

    let name = json
        .get("name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            dir.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "connector".to_string())
        });
    let version = json
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("1.0.0")
        .to_string();
    let category = category_display_name(json.get("category").and_then(|v| v.as_str()).unwrap_or(""));
    let description = cap_description(json.get("description").and_then(|v| v.as_str()).unwrap_or(""));
    let title = json.get("title").and_then(|v| v.as_str());

    let item = upsert(account_id, &name, "connector", category, &description, &version, visibility, &raw).await?;

    apply_listing(dir, &item.id, &name, title).await?;
    finalize(&item.id, &name, "Connector", &version, visibility, item.previous_status.as_deref()).await?;
    Ok(())
}

async fn publish_skill(dir: &Path, account_id: &str, visibility: &str) -> Result<()> {
    let skill_md = read_file(dir, "SKILL.md")?;
    let fm = extract_frontmatter_fields(&skill_md)?;
    let name = fm.name;
    let version = fm.version.unwrap_or_else(|| "1.0.0".to_string());
    let category = category_display_name(fm.category.as_deref().unwrap_or(""));
    let description = cap_description(&fm.description);

    let item = upsert(account_id, &name, "skill", category, &description, &version, visibility, &skill_md).await?;

    // Upload the whole directory as a bundle so references/, scripts/, and
    // assets/ ship alongside SKILL.md. The server re-extracts SKILL.md into the
    // manifest, so this is safe (and a no-op in effect) for single-file skills.
    let file_count = api::upload_bundle(&item.id, dir).await?;
    println!("  Uploaded bundle ({file_count} files: SKILL.md + references/scripts/assets)");

    // Set the human marketplace listing: a clean Title Case display name (the
    // frontmatter name is the lowercase runtime id) and the "What it does" long
    // description from LISTING.md, if present.
    apply_listing(dir, &item.id, &name, fm.title.as_deref()).await?;
    finalize(&item.id, &name, "Skill", &version, visibility, item.previous_status.as_deref()).await?;
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
    // Description from PLUGIN.md frontmatter (falls back to plugin.json), capped at 500 chars.
    let fm = extract_frontmatter_fields(&plugin_md).ok();
    let description = fm
        .as_ref()
        .map(|f| f.description.clone())
        .filter(|d| !d.is_empty())
        .or_else(|| plugin_json.get("description").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .unwrap_or_else(|| format!("{name} — NeboAI plugin"));
    let description = cap_description(&description);

    // A plugin installs from its binaries: refuse before creating anything.
    let dist_dir = dir.join("dist").join("plugin");
    if !PLATFORMS.iter().any(|p| dist_dir.join(p).exists()) {
        bail!("No platform binaries found in dist/plugin/. Run ./build.sh first. Expected at least one of: {PLATFORMS:?}");
    }

    let item = upsert(account_id, &name, "plugin", category, &description, &version, visibility, &plugin_md).await?;

    // Build skills tarball if skills/ exists
    let skills_tarball = if dir.join("skills").exists() {
        let tarball_path = std::env::temp_dir().join(format!("neboai-{name}-skills.tar.gz"));
        build_tarball(&dir.join("skills"), "skills", &tarball_path)?;
        println!("  Skills tarball built");
        Some(tarball_path)
    } else {
        None
    };

    // Upload available platform binaries (config + skills on the first one).
    // Each upload is scanned before it can ship.
    let mut first = true;
    for platform in PLATFORMS {
        let platform_dir = dist_dir.join(platform);
        if !platform_dir.exists() {
            continue;
        }
        let binary_path = find_binary(&platform_dir)?;
        api::upload_binary(
            &item.id,
            platform,
            Some(&binary_path),
            &dir.join("PLUGIN.md"),
            if first { Some(&plugin_json_path) } else { None },
            if first { skills_tarball.as_deref() } else { None },
            None,
        )
        .await?;
        first = false;
    }

    apply_listing(dir, &item.id, &name, None).await?;
    finalize(&item.id, &name, "Plugin", &version, visibility, item.previous_status.as_deref()).await?;
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

    let item = upsert(account_id, &name, "agent", category, &description, &version, visibility, &agent_md).await?;

    // agent.json is stored as the employee's config; employees have no binary.
    api::upload_binary(
        &item.id,
        "linux-amd64", // Required field, but agents aren't platform-specific
        None,          // No binary file for agents
        &dir.join("AGENT.md"),
        Some(&agent_json_path),
        None,
        None,
    )
    .await?;

    apply_listing(dir, &item.id, &name, fm.title.as_deref()).await?;
    finalize(&item.id, &name, "Agent", &version, visibility, item.previous_status.as_deref()).await?;
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

    // An app installs from its sidecar binaries (the UI rides along with
    // them), so at least one is required — refuse before creating anything.
    let sidecars = app_sidecars(dir)?;
    if sidecars.is_empty() {
        bail!(
            "No sidecar binary found. Put one per platform in dist/app/<platform>/ (one of {PLATFORMS:?}), or build sidecar/ for this machine."
        );
    }

    let item = upsert(account_id, &name, "app", category, &description, &version, visibility, &agent_md).await?;

    let ui_tarball = std::env::temp_dir().join(format!("neboai-{}-ui.tar.gz", item.id));
    build_tarball(&dir.join("ui"), "ui", &ui_tarball)?;
    let config_path = dir.join("agent.json");
    let config_path = config_path.exists().then_some(config_path);

    // The UI bundle and agent.json go with the first binary.
    for (i, (platform, binary)) in sidecars.iter().enumerate() {
        api::upload_binary(
            &item.id,
            platform,
            Some(binary),
            &dir.join("AGENT.md"),
            if i == 0 { config_path.as_deref() } else { None },
            None,
            if i == 0 { Some(ui_tarball.as_path()) } else { None },
        )
        .await?;
    }

    apply_listing(dir, &item.id, &name, fm.as_ref().and_then(|f| f.title.as_deref())).await?;
    finalize(&item.id, &name, "App", &version, visibility, item.previous_status.as_deref()).await?;
    Ok(())
}

/// An app's sidecar binaries: one per platform in dist/app/<platform>/, or the
/// local build in sidecar/target/release/ for this machine's platform.
fn app_sidecars(dir: &Path) -> Result<Vec<(&'static str, std::path::PathBuf)>> {
    let dist_dir = dir.join("dist").join("app");
    let mut found = Vec::new();
    for platform in PLATFORMS {
        let platform_dir = dist_dir.join(platform);
        if platform_dir.is_dir() {
            found.push((*platform, find_binary(&platform_dir)?));
        }
    }
    if found.is_empty() {
        let local = dir.join("sidecar").join("target").join("release");
        if local.is_dir() {
            if let Ok(binary) = find_binary(&local) {
                found.push((current_platform(), binary));
            }
        }
    }
    Ok(found)
}

// --- Helpers ---

/// Finish a publish. A public item is submitted for review when it is new or
/// not yet published; a published item stays listed when updated, so its new
/// version goes live without another submit (new binaries are still scanned
/// before they ship). Private and loop items are not listed and not reviewed.
async fn finalize(
    id: &str,
    name: &str,
    kind: &str,
    version: &str,
    visibility: &str,
    previous_status: Option<&str>,
) -> Result<()> {
    if visibility != "public" {
        println!("\nDone! {kind} '{name}' v{version} saved ({visibility}). It is not listed, so it is not reviewed.");
        return Ok(());
    }
    match previous_status {
        Some("active") => {
            println!("\nDone! {kind} '{name}' updated to v{version}. It stays published.");
        }
        Some("pending_review") | Some("review") => {
            println!("\nDone! {kind} '{name}' updated to v{version}. It is already in review; check with `neboai status {id}`.");
        }
        _ => {
            println!("Submitting v{version} for review...");
            api::submit(id, version).await?;
            println!("\nDone! {kind} '{name}' v{version} submitted. Item ID: {id}");
        }
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
    /// Optional human display name for the marketplace listing. When absent we
    /// Title Case the (lowercase) `name`.
    title: Option<String>,
}

/// Clean Title Case display name for the marketplace, from an explicit
/// frontmatter `title` or derived from the lowercase-hyphen runtime `name`
/// ("nebo-design" -> "Nebo Design", "x-manager" -> "X Manager").
fn clean_display_name(name: &str, title: Option<&str>) -> String {
    if let Some(t) = title {
        let t = t.trim();
        if !t.is_empty() {
            return t.to_string();
        }
    }
    name.split(['-', '_'])
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Read the marketplace long description from LISTING.md (the publisher's
/// human "What it does" body), stripping YAML frontmatter if present. Returns
/// None when there's no LISTING.md. This is separate from SKILL.md, which is
/// for the LLM at runtime.
fn read_listing(dir: &Path) -> Option<String> {
    let path = if dir.join("LISTING.md").exists() {
        dir.join("LISTING.md")
    } else if dir.join("listing.md").exists() {
        dir.join("listing.md")
    } else {
        return None;
    };
    let raw = std::fs::read_to_string(&path).ok()?;
    let body = strip_frontmatter(&raw).trim().to_string();
    (!body.is_empty()).then_some(body)
}

/// Drop a leading `---`-delimited YAML frontmatter block, returning the body.
fn strip_frontmatter(content: &str) -> &str {
    let trimmed = content.trim_start();
    if let Some(rest) = trimmed.strip_prefix("---") {
        if let Some(end) = rest.find("\n---") {
            // Skip past the closing delimiter line.
            if let Some(nl) = rest[end + 1..].find('\n') {
                return &rest[end + 1 + nl + 1..];
            }
        }
    }
    content
}

/// Set the human marketplace listing for a freshly created artifact: a clean
/// display name (slug stays the lowercase runtime id) and, if a LISTING.md is
/// present, the long "What it does" description. Both go through the publisher
/// update endpoint, which merges — omitted fields keep their current value.
async fn apply_listing(dir: &Path, id: &str, name: &str, title: Option<&str>) -> Result<()> {
    let display_name = clean_display_name(name, title);
    let long_description = read_listing(dir);
    api::update_listing(id, &display_name, long_description.as_deref()).await?;
    if long_description.is_some() {
        println!("  Listing: name \"{display_name}\" + LISTING.md long description");
    } else {
        println!("  Listing: name \"{display_name}\"");
    }
    Ok(())
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

    let title = yaml
        .get("title")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Ok(FrontmatterFields {
        name,
        version,
        description,
        category,
        title,
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

/// tar.gz `src` into `output`, rooted at `prefix/`.
fn build_tarball(src: &Path, prefix: &str, output: &Path) -> Result<()> {
    let file = std::fs::File::create(output)?;
    let enc = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut tar = tar::Builder::new(enc);
    tar.append_dir_all(prefix, src)?;
    tar.into_inner()?.finish()?;
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
    #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
    return "windows-arm64";
    #[cfg(not(any(
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "windows", target_arch = "x86_64"),
        all(target_os = "windows", target_arch = "aarch64"),
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
    fn display_name_title_cases_the_slug() {
        assert_eq!(clean_display_name("nebo-design", None), "Nebo Design");
        assert_eq!(clean_display_name("x-manager", None), "X Manager");
        assert_eq!(clean_display_name("cold-email", None), "Cold Email");
        assert_eq!(clean_display_name("seo", None), "Seo");
    }

    #[test]
    fn display_name_prefers_explicit_title() {
        assert_eq!(clean_display_name("nebo-design", Some("Nebo Design Studio")), "Nebo Design Studio");
        // Blank title falls back to derivation.
        assert_eq!(clean_display_name("nebo-design", Some("  ")), "Nebo Design");
    }

    #[test]
    fn strip_frontmatter_drops_yaml_block() {
        assert_eq!(strip_frontmatter("---\ntitle: X\n---\nBody here").trim(), "Body here");
        assert_eq!(strip_frontmatter("No frontmatter").trim(), "No frontmatter");
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
