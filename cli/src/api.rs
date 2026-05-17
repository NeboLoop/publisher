use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;

use crate::auth;

const BASE_URL: &str = "https://neboloop.com/api/v1";

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

    let resp = client
        .get(format!("{BASE_URL}/developer/apps"))
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

    let resp = client
        .get(format!("{BASE_URL}/developer/apps/{id}"))
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

    let resp = client
        .get(format!("{BASE_URL}/developer/apps/{id}/binaries"))
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

    let resp = client
        .delete(format!("{BASE_URL}/developer/binaries/{id}"))
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

// --- Upload Token ---

pub async fn get_upload_token(id: &str) -> Result<String> {
    let (client, token) = authenticated_client().await?;

    let resp = client
        .post(format!("{BASE_URL}/developer/apps/{id}/upload-token"))
        .bearer_auth(&token)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await?;
        anyhow::bail!("Failed to get upload token ({status}): {body}");
    }

    #[derive(Deserialize)]
    struct TokenResp {
        token: String,
    }

    let token_resp: TokenResp = resp.json().await?;
    Ok(token_resp.token)
}

// --- Create / Update / Submit ---

pub async fn create_artifact(name: &str, category: &str, manifest_content: &str) -> Result<String> {
    let (client, token) = authenticated_client().await?;

    let body = serde_json::json!({
        "name": name,
        "category": category,
        "manifestContent": manifest_content,
    });

    let resp = client
        .post(format!("{BASE_URL}/developer/apps"))
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

    let body = serde_json::json!({
        "manifestContent": manifest_content,
    });

    let resp = client
        .put(format!("{BASE_URL}/developer/apps/{id}"))
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

pub async fn submit(id: &str, version: &str) -> Result<()> {
    let (client, token) = authenticated_client().await?;

    let body = serde_json::json!({
        "version": version,
    });

    let resp = client
        .post(format!("{BASE_URL}/developer/apps/{id}/submit"))
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
    upload_token: &str,
    platform: &str,
    binary_path: Option<&std::path::Path>,
    manifest_path: &std::path::Path,
    config_path: Option<&std::path::Path>,
    skills_tarball: Option<&std::path::Path>,
) -> Result<()> {
    let url = format!("{BASE_URL}/developer/apps/{id}/binaries");

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
        .bearer_auth(upload_token)
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
