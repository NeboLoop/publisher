use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const AUTH_URL: &str = "https://neboai.com/oauth/authorize";
const TOKEN_URL: &str = "https://neboai.com/oauth/token";
// Dedicated first-party public client for the CLI (PKCE, no secret). Its access
// token is an owner token that the NeboAI API accepts directly. Uses its own
// port 19847 + /auth/neboai/callback redirect so it never collides with the Nebo
// desktop app (which owns port 27895).
const CLIENT_ID: &str = "nbl_neboai_cli";
const REDIRECT_PORT: u16 = 19847;

#[derive(Debug, Serialize, Deserialize)]
pub struct Credentials {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<i64>,
}

impl Credentials {
    pub fn is_expired(&self) -> bool {
        match self.expires_at {
            Some(exp) => chrono::Utc::now().timestamp() >= exp - 60,
            None => false,
        }
    }
}

fn credentials_path() -> PathBuf {
    let dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("neboai");
    std::fs::create_dir_all(&dir).ok();
    dir.join("credentials.json")
}

pub fn load_credentials() -> Result<Option<Credentials>> {
    let path = credentials_path();
    if !path.exists() {
        return Ok(None);
    }
    let data = std::fs::read_to_string(&path)?;
    let creds: Credentials = serde_json::from_str(&data)?;
    Ok(Some(creds))
}

fn save_credentials(creds: &Credentials) -> Result<()> {
    let path = credentials_path();
    let data = serde_json::to_string_pretty(creds)?;
    std::fs::write(&path, data)?;
    // The token is an account credential: keep it readable by this user only.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

pub async fn login() -> Result<()> {
    // PKCE verifier/challenge plus a state value tying the callback to this run.
    let verifier = random_urlsafe(32);
    let challenge = generate_pkce_challenge(&verifier);
    let state = random_urlsafe(16);

    let redirect_uri = format!("http://localhost:{REDIRECT_PORT}/auth/neboai/callback");
    let auth_url = format!(
        "{AUTH_URL}?client_id={CLIENT_ID}&redirect_uri={}&response_type=code&scope={}&state={state}&code_challenge={challenge}&code_challenge_method=S256",
        urlencoding::encode(&redirect_uri),
        urlencoding::encode("openid profile email")
    );

    // Bind before opening the browser so the redirect always has a listener.
    let server = tiny_http::Server::http(format!("127.0.0.1:{REDIRECT_PORT}")).map_err(|e| {
        anyhow::anyhow!("Could not listen on port {REDIRECT_PORT} for the sign-in callback: {e}")
    })?;

    println!("Opening your browser to sign in to NeboAI...");
    println!("If it doesn't open, visit:\n{auth_url}\n");
    open::that(&auth_url).ok();
    println!("Waiting for sign-in...");

    let code = loop {
        let request = server
            .recv_timeout(std::time::Duration::from_secs(600))
            .context("Failed to receive the sign-in callback")?
            .context("Timed out waiting for sign-in. Run `neboai auth login` again.")?;
        let url = url::Url::parse(&format!("http://localhost{}", request.url()))?;
        if url.path() != "/auth/neboai/callback" {
            // Browsers also ask for /favicon.ico and the like.
            request.respond(tiny_http::Response::empty(404)).ok();
            continue;
        }
        let param = |name: &str| {
            url.query_pairs()
                .find(|(k, _)| k == name)
                .map(|(_, v)| v.to_string())
        };
        let (page, result) = if let Some(err) = param("error") {
            (
                "Sign-in was cancelled. You can close this tab.",
                Err(anyhow::anyhow!("Sign-in failed: {err}")),
            )
        } else if param("state").as_deref() != Some(state.as_str()) {
            (
                "Sign-in could not be verified. You can close this tab.",
                Err(anyhow::anyhow!(
                    "Sign-in failed: the callback did not match this request"
                )),
            )
        } else if let Some(code) = param("code") {
            ("Signed in to NeboAI. You can close this tab.", Ok(code))
        } else {
            (
                "Sign-in failed. You can close this tab.",
                Err(anyhow::anyhow!(
                    "Sign-in failed: no authorization code received"
                )),
            )
        };
        let response =
            tiny_http::Response::from_string(format!("<html><body><h1>{page}</h1></body></html>"))
                .with_header(
                    "Content-Type: text/html"
                        .parse::<tiny_http::Header>()
                        .unwrap(),
                );
        request.respond(response).ok();
        break result?;
    };

    // Exchange code for token
    let client = reqwest::Client::new();
    let resp = client
        .post(TOKEN_URL)
        .form(&[
            ("grant_type", "authorization_code"),
            ("client_id", CLIENT_ID),
            ("code", &code),
            ("redirect_uri", &redirect_uri),
            ("code_verifier", &verifier),
        ])
        .send()
        .await?;

    if !resp.status().is_success() {
        let body = resp.text().await?;
        anyhow::bail!("Token exchange failed: {body}");
    }

    #[derive(Deserialize)]
    struct TokenResponse {
        access_token: String,
        refresh_token: Option<String>,
        expires_in: Option<i64>,
    }

    let token: TokenResponse = resp.json().await?;
    let creds = Credentials {
        access_token: token.access_token,
        refresh_token: token.refresh_token,
        expires_at: token
            .expires_in
            .map(|exp| chrono::Utc::now().timestamp() + exp),
    };

    save_credentials(&creds)?;
    // Listing accounts also sets up the free publisher account on first use.
    match crate::api::resolve_account(None).await {
        Ok(account) => println!("Signed in. Publishing as @{}.", account.slug),
        Err(_) => println!("Signed in."),
    }
    Ok(())
}

pub async fn status() -> Result<()> {
    match load_credentials()? {
        None => println!("Not signed in. Run `neboai auth login`."),
        Some(creds) if creds.is_expired() => {
            println!("Session expired. Run `neboai auth login`.")
        }
        // Ask the server, so a revoked or deleted session isn't reported as signed in.
        Some(_) => match crate::api::resolve_account(None).await {
            Ok(account) => println!("Signed in to NeboAI. Publishing as @{}.", account.slug),
            Err(e) => println!("Signed in locally, but NeboAI did not accept the session ({e}). Run `neboai auth login`."),
        },
    }
    Ok(())
}

pub async fn logout() -> Result<()> {
    let path = credentials_path();
    if path.exists() {
        std::fs::remove_file(&path)?;
        println!("Signed out.");
    } else {
        println!("Not signed in.");
    }
    Ok(())
}

pub async fn get_token() -> Result<String> {
    let creds = load_credentials()?.context("Not signed in. Run `neboai auth login` first.")?;

    if creds.is_expired() {
        anyhow::bail!("Session expired. Run `neboai auth login`.");
    }

    Ok(creds.access_token)
}

fn random_urlsafe(len: usize) -> String {
    use base64::Engine;
    let mut bytes = vec![0u8; len];
    getrandom::getrandom(&mut bytes).expect("OS random number generator unavailable");
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn generate_pkce_challenge(verifier: &str) -> String {
    use base64::Engine;
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(verifier.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hash)
}

mod urlencoding {
    pub fn encode(s: &str) -> String {
        url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
    }
}
