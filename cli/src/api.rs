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

    let body = resp.text().await?;
    // Pretty print
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        println!("{body}");
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

    let body = resp.text().await?;
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        println!("{body}");
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

pub async fn delete_binary(id: &str) -> Result<()> {
    let (client, token) = authenticated_client().await?;
    let base = base_url();

    let resp = client
        .delete(format!("{base}/developer/binaries/{id}"))
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

/// Resolve the developer account id to publish under. If `slug` is given, picks
/// the account with that slug; otherwise returns the first account. The account
/// id is required by the create endpoint (publishing is namespace-scoped).
pub async fn resolve_account(slug: Option<&str>) -> Result<String> {
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
        anyhow::bail!("Failed to list developer accounts ({status}): {body}");
    }

    #[derive(Deserialize)]
    struct Account {
        id: String,
        slug: String,
    }
    let val: serde_json::Value = resp.json().await?;
    let arr = val.get("accounts").cloned().unwrap_or(val);
    let accounts: Vec<Account> = serde_json::from_value(arr).unwrap_or_default();

    if accounts.is_empty() {
        anyhow::bail!("No developer accounts found. Create one at neboai.com first.");
    }
    match slug {
        Some(s) => accounts
            .iter()
            .find(|a| a.slug == s)
            .map(|a| a.id.clone())
            .with_context(|| format!("No developer account with slug '{s}'")),
        None => Ok(accounts[0].id.clone()),
    }
}

// --- Create / Update / Submit ---

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

#[allow(dead_code)]
pub async fn update_manifest(id: &str, manifest_content: &str) -> Result<()> {
    let (client, token) = authenticated_client().await?;
    let base = base_url();

    let body = serde_json::json!({
        "manifestContent": manifest_content,
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
    name: &str,
    description: &str,
    visibility: &str,
) -> Result<String> {
    let (client, token) = authenticated_client().await?;
    let base = base_url();

    let body = serde_json::json!({
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

    println!("Submitted for review.");
    Ok(())
}

// --- Binary Upload ---

pub async fn upload_binary(
    id: &str,
    platform: &str,
    binary_path: Option<&std::path::Path>,
    manifest_path: &std::path::Path,
    config_path: Option<&std::path::Path>,
    skills_tarball: Option<&std::path::Path>,
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
        // Skip VCS metadata, OS noise, and LISTING.md (uploaded separately as
        // the marketplace long description, not a runtime skill file).
        let base = rel.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if rel_str.split('/').any(|c| c == ".git")
            || base == ".DS_Store"
            || base == "LISTING.md"
            || base == "listing.md"
        {
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
