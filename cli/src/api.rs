use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;

use crate::auth;

const DEFAULT_BASE_URL: &str = "https://neboai.com/api/v1";

/// Base API URL. Defaults to production; override with $NEBOAI_BASE_URL to point
/// the CLI at a local server (e.g. http://localhost:8080/api/v1) for testing.
pub fn base_url() -> String {
    std::env::var("NEBOAI_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_string())
}

pub fn client() -> Client {
    Client::new()
}

pub async fn authenticated_client() -> Result<(Client, String)> {
    let token = auth::get_token().await?;
    Ok((client(), token))
}

// --- Artifact Management ---

pub async fn list_artifacts() -> Result<()> {
    let (client, token) = authenticated_client().await?;
    let base = base_url();

    let resp = client
        .get(format!("{base}/developer/apps"))
        .bearer_auth(&token)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await?;
        anyhow::bail!("Failed to list artifacts ({status}): {body}");
    }

    let body: serde_json::Value = resp.json().await?;
    let empty = vec![];
    let products = body
        .get("products")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty);
    if products.is_empty() {
        println!("Nothing published yet. Run `neboai publish <dir>`.");
        return Ok(());
    }
    for p in products {
        let app = p.get("app").unwrap_or(p);
        let field = |k: &str| app.get(k).and_then(|v| v.as_str()).unwrap_or("");
        println!(
            "{:<10} {:<32} v{:<10} {:<10} {}",
            field("type"),
            field("name"),
            field("version"),
            status_label(field("status")),
            field("id")
        );
    }

    Ok(())
}

pub async fn get_status(id: &str) -> Result<()> {
    let (client, token) = authenticated_client().await?;
    let base = base_url();

    let resp = client
        .get(format!("{base}/developer/apps/{id}"))
        .bearer_auth(&token)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await?;
        anyhow::bail!("Failed to get status ({status}): {body}");
    }

    let json: serde_json::Value = resp.json().await?;
    let field = |k: &str| json.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
    println!("{} ({}) v{}", field("name"), field("type"), field("version"));
    println!("  Status:     {}", status_label(&field("status")));
    println!("  Visibility: {}", field("visibility"));
    if !field("code").is_empty() {
        println!("  Install code: {}", field("code"));
    }

    Ok(())
}

// --- Binary Management ---

pub async fn list_binaries(id: &str) -> Result<()> {
    let (client, token) = authenticated_client().await?;
    let base = base_url();

    let resp = client
        .get(format!("{base}/developer/apps/{id}/binaries"))
        .bearer_auth(&token)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await?;
        anyhow::bail!("Failed to list binaries ({status}): {body}");
    }

    let body = resp.text().await?;
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        println!("{body}");
    }

    Ok(())
}

pub async fn delete_binary(artifact_id: &str, binary_id: &str) -> Result<()> {
    let (client, token) = authenticated_client().await?;
    let base = base_url();

    let resp = client
        .delete(format!("{base}/developer/apps/{artifact_id}/binaries/{binary_id}"))
        .bearer_auth(&token)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await?;
        anyhow::bail!("Failed to delete binary ({status}): {body}");
    }

    println!("Binary deleted.");
    Ok(())
}

// --- Developer accounts ---

/// The publisher account (developer account) an item is published under.
/// Every NeboAI account can publish: listing accounts sets up the free personal
/// one on first use, so there is always at least one.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    pub slug: String,
    #[serde(default)]
    pub namespace_id: String,
}

/// Resolve the publisher account to publish under. `slug` picks a team account
/// you belong to (set $NEBOAI_ACCOUNT); otherwise your first (personal) account.
pub async fn resolve_account(slug: Option<&str>) -> Result<Account> {
    let (client, token) = authenticated_client().await?;
    let base = base_url();

    let resp = client
        .get(format!("{base}/developer/accounts"))
        .bearer_auth(&token)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await?;
        anyhow::bail!("Failed to list publisher accounts ({status}): {}", body.trim());
    }

    let val: serde_json::Value = resp.json().await?;
    let arr = val.get("accounts").cloned().unwrap_or(val);
    let accounts: Vec<Account> = serde_json::from_value(arr).unwrap_or_default();

    match slug {
        Some(s) => accounts
            .into_iter()
            .find(|a| a.slug == s)
            .with_context(|| format!("You are not a member of a publisher account '@{s}'")),
        None => accounts
            .into_iter()
            .next()
            .context("NeboAI did not return a publisher account. Try again, or sign in again with `neboai auth login`."),
    }
}

// --- Create / Update / Submit ---

/// An item you already published: its ID and current status
/// (draft, pending_review, review, active, revoked).
pub struct Existing {
    pub id: String,
    pub status: String,
}

/// Resolve an existing artifact's ID by slug/name + type. Publishing an
/// artifact that already exists is a version update, not a create — the
/// create endpoint rejects the slug+type unique constraint with a 400.
pub async fn find_artifact(name: &str, artifact_type: &str) -> Result<Option<Existing>> {
    let (client, token) = authenticated_client().await?;
    let base = base_url();

    let resp = client
        .get(format!("{base}/developer/apps"))
        .bearer_auth(&token)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await?;
        anyhow::bail!("Failed to list artifacts ({status}): {body}");
    }

    let body: serde_json::Value = resp.json().await?;
    let empty = vec![];
    let products = body
        .get("products")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty);
    for p in products {
        let app = p.get("app").unwrap_or(p);
        let slug = app.get("slug").and_then(|v| v.as_str());
        let app_name = app.get("name").and_then(|v| v.as_str());
        let ty = app.get("type").and_then(|v| v.as_str());
        if ty == Some(artifact_type) && (slug == Some(name) || app_name == Some(name)) {
            if let Some(id) = app.get("id").and_then(|v| v.as_str()) {
                let status = app.get("status").and_then(|v| v.as_str()).unwrap_or("");
                return Ok(Some(Existing {
                    id: id.to_string(),
                    status: status.to_string(),
                }));
            }
        }
    }
    Ok(None)
}

#[allow(clippy::too_many_arguments)]
pub async fn create_artifact(
    account_id: &str,
    name: &str,
    artifact_type: &str,
    category: &str,
    description: &str,
    version: &str,
    visibility: &str,
    manifest_content: &str,
) -> Result<String> {
    let (client, token) = authenticated_client().await?;
    let base = base_url();

    let body = serde_json::json!({
        "accountId": account_id,
        "name": name,
        "type": artifact_type,
        "category": category,
        "description": description,
        "version": version,
        "visibility": visibility,
        "manifestContent": manifest_content,
    });

    let resp = client
        .post(format!("{base}/developer/apps"))
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await?;
        anyhow::bail!("Failed to create artifact ({status}): {body}");
    }

    #[derive(Deserialize)]
    struct CreateResp {
        id: String,
    }

    let created: CreateResp = resp.json().await?;
    Ok(created.id)
}

/// Update an existing artifact's manifest and (when provided) its version.
/// The version MUST be updated before uploading a new version's binaries:
/// the binaries endpoint records uploads under the artifact row's CURRENT
/// version, so uploading first mislabels the new bytes as the old version.
pub async fn update_manifest(
    id: &str,
    manifest_content: &str,
    version: &str,
    description: &str,
) -> Result<()> {
    let (client, token) = authenticated_client().await?;
    let base = base_url();

    let body = serde_json::json!({
        "manifestContent": manifest_content,
        "version": version,
        "description": description,
    });

    let resp = client
        .put(format!("{base}/developer/apps/{id}"))
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await?;
        anyhow::bail!("Failed to update manifest ({status}): {body}");
    }

    Ok(())
}

/// Create a collection (a bundle of existing artifacts). Returns its ID.
pub async fn create_collection(
    namespace_id: &str,
    name: &str,
    description: &str,
    visibility: &str,
) -> Result<String> {
    let (client, token) = authenticated_client().await?;
    let base = base_url();

    let body = serde_json::json!({
        "namespaceId": namespace_id,
        "name": name,
        "description": description,
        "visibility": visibility,
    });

    let resp = client
        .post(format!("{base}/collections"))
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await?;
        anyhow::bail!("Failed to create collection ({status}): {body}");
    }

    let val: serde_json::Value = resp.json().await?;
    val.get("id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .context("collection create response missing id")
}

/// Add an existing artifact to a collection by its ID and type.
pub async fn add_collection_item(
    collection_id: &str,
    target_id: &str,
    target_type: &str,
    position: i64,
) -> Result<()> {
    let (client, token) = authenticated_client().await?;
    let base = base_url();

    let body = serde_json::json!({
        "targetId": target_id,
        "targetType": target_type,
        "position": position,
    });

    let resp = client
        .post(format!("{base}/collections/{collection_id}/items"))
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await?;
        anyhow::bail!("Failed to add collection item {target_id} ({status}): {body}");
    }
    Ok(())
}

/// Set the marketplace listing fields (display name + long "What it does"
/// description) on an artifact via the publisher update endpoint. The handler
/// merges, so empty fields are left untouched. `long_description` is the
/// human-facing body from LISTING.md, separate from the SKILL.md the LLM uses.
pub async fn update_listing(id: &str, name: &str, long_description: Option<&str>) -> Result<()> {
    let (client, token) = authenticated_client().await?;
    let base = base_url();

    let mut body = serde_json::Map::new();
    if !name.is_empty() {
        body.insert("name".into(), serde_json::Value::String(name.to_string()));
    }
    if let Some(ld) = long_description {
        body.insert("longDescription".into(), serde_json::Value::String(ld.to_string()));
    }
    if body.is_empty() {
        return Ok(());
    }

    let resp = client
        .put(format!("{base}/developer/apps/{id}"))
        .bearer_auth(&token)
        .json(&serde_json::Value::Object(body))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await?;
        anyhow::bail!("Failed to update listing ({status}): {body}");
    }

    Ok(())
}

/// Submit the item's current version for review and report the outcome.
/// Text-only items get a content scan and are approved at once when clean;
/// items with binaries are scanned in the background.
pub async fn submit(id: &str, version: &str) -> Result<()> {
    let (client, token) = authenticated_client().await?;
    let base = base_url();

    let body = serde_json::json!({
        "version": version,
    });

    let resp = client
        .post(format!("{base}/developer/apps/{id}/submit"))
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await?;
        anyhow::bail!("Failed to submit ({status}): {body}");
    }

    let val: serde_json::Value = resp.json().await.unwrap_or_default();
    let outcome = val
        .pointer("/submission/status")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    println!("  {}", submit_outcome(outcome));
    Ok(())
}

/// Plain words for a submission status.
pub fn submit_outcome(status: &str) -> &'static str {
    match status {
        "approved" | "auto_approved" => "Review: approved. It is published.",
        "flagged" | "manual_review" => {
            "Review: the automated scan flagged it, so a person on the NeboAI team will review it. Check with `neboai status <id>`."
        }
        _ => "Review: in progress (binaries are scanned in the background). Check with `neboai status <id>`.",
    }
}

/// Plain words for an item status.
pub fn status_label(status: &str) -> &'static str {
    match status {
        "active" => "Published",
        "pending_review" | "review" => "In review",
        "revoked" => "Removed",
        _ => "Draft",
    }
}

// --- Binary Upload ---

pub async fn upload_binary(
    id: &str,
    platform: &str,
    binary_path: Option<&std::path::Path>,
    manifest_path: &std::path::Path,
    config_path: Option<&std::path::Path>,
    skills_tarball: Option<&std::path::Path>,
    ui_tarball: Option<&std::path::Path>,
) -> Result<()> {
    let upload_token = auth::get_token().await?;
    let base = base_url();
    let url = format!("{base}/developer/apps/{id}/binaries");

    let mut form = reqwest::multipart::Form::new().text("platform", platform.to_string());

    // Manifest (SKILL.md / PLUGIN.md / AGENT.md)
    let manifest_bytes = std::fs::read(manifest_path)
        .with_context(|| format!("Failed to read {}", manifest_path.display()))?;
    form = form.part(
        "skill",
        reqwest::multipart::Part::bytes(manifest_bytes).file_name(
            manifest_path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string(),
        ),
    );

    // Binary file (optional for agents)
    if let Some(bin_path) = binary_path {
        let bin_bytes = std::fs::read(bin_path)
            .with_context(|| format!("Failed to read binary {}", bin_path.display()))?;
        form = form.part(
            "file",
            reqwest::multipart::Part::bytes(bin_bytes)
                .file_name(bin_path.file_name().unwrap().to_string_lossy().to_string()),
        );
    }

    // Config (plugin.json / agent.json)
    if let Some(cfg_path) = config_path {
        let cfg_bytes = std::fs::read(cfg_path)
            .with_context(|| format!("Failed to read config {}", cfg_path.display()))?;
        form = form.part(
            "config",
            reqwest::multipart::Part::bytes(cfg_bytes)
                .file_name(cfg_path.file_name().unwrap().to_string_lossy().to_string()),
        );
    }

    // Skills tarball
    if let Some(tarball_path) = skills_tarball {
        let tar_bytes = std::fs::read(tarball_path)
            .with_context(|| format!("Failed to read skills tarball {}", tarball_path.display()))?;
        form = form.part(
            "skills",
            reqwest::multipart::Part::bytes(tar_bytes).file_name("skills.tar.gz"),
        );
    }

    // App UI bundle (tar.gz of ui/), packed under ui/ in the installable package
    if let Some(ui_path) = ui_tarball {
        let ui_bytes = std::fs::read(ui_path)
            .with_context(|| format!("Failed to read UI bundle {}", ui_path.display()))?;
        form = form.part(
            "ui",
            reqwest::multipart::Part::bytes(ui_bytes).file_name("ui.tar.gz"),
        );
    }

    let client = reqwest::ClientBuilder::new().http1_only().build()?;

    let resp = client
        .post(&url)
        .bearer_auth(&upload_token)
        .multipart(form)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await?;
        anyhow::bail!("Upload failed for {platform} ({status}): {body}");
    }

    println!("  Uploaded: {platform}");
    Ok(())
}

// --- Skill bundle upload (multi-file skills) ---

/// Zip an entire skill directory in memory, preserving relative paths, then POST
/// it to /skills/{id}/bundle. The server extracts SKILL.md into the manifest and
/// stores the rest (references/, scripts/, assets/) as skill files. `.git/` is
/// skipped to keep the upload small; the server filters other noise itself.
pub async fn upload_bundle(id: &str, dir: &std::path::Path) -> Result<usize> {
    let (zip_bytes, file_count) = zip_dir(dir)?;

    let upload_token = auth::get_token().await?;
    let base = base_url();
    let url = format!("{base}/skills/{id}/bundle");

    let form = reqwest::multipart::Form::new().part(
        "file",
        reqwest::multipart::Part::bytes(zip_bytes).file_name("skill.zip"),
    );

    // HTTP/1.1 only — HTTP/2 causes stream errors on large multipart uploads.
    let client = reqwest::ClientBuilder::new().http1_only().build()?;

    let resp = client
        .post(&url)
        .bearer_auth(&upload_token)
        .multipart(form)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await?;
        anyhow::bail!("Bundle upload failed ({status}): {body}");
    }

    Ok(file_count)
}

/// Build a zip of `dir` in memory. Returns the zip bytes and the number of files
/// included. Entry paths are relative to `dir`. Skips `.git/` and common OS noise.
fn zip_dir(dir: &std::path::Path) -> Result<(Vec<u8>, usize)> {
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    let mut cursor = std::io::Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(&mut cursor);
    let opts = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let mut count = 0usize;
    for entry in walkdir::WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let rel = path
            .strip_prefix(dir)
            .with_context(|| format!("path escapes skill dir: {}", path.display()))?;
        // Normalize to forward slashes; the server matches path components.
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        // The runtime bundle is exactly the documented skill format: SKILL.md
        // plus the references/, scripts/, and assets/ trees. Everything else
        // in the directory is dev scaffolding (a cli/ crate with target/, CI
        // files, Formula, …) that must never ship — an unfiltered walk blows
        // the upload size cap and leaks non-runtime files.
        let top = rel_str.split('/').next().unwrap_or("");
        let allowed = rel_str == "SKILL.md"
            || top == "references"
            || top == "scripts"
            || top == "assets";
        let base = rel.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if !allowed || rel_str.split('/').any(|c| c == ".git") || base == ".DS_Store" {
            continue;
        }
        let bytes = std::fs::read(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        zip.start_file(rel_str, opts)?;
        zip.write_all(&bytes)?;
        count += 1;
    }
    zip.finish()?;
    Ok((cursor.into_inner(), count))
}
