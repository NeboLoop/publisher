use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const AUTH_URL: &str = "https://neboai.com/oauth/authorize";
const TOKEN_URL: &str = "https://neboai.com/oauth/token";
// Dedicated first-party public client for the CLI (PKCE, no secret), registered
// in the backend oauth_apps table under the nebo-official account. Uses its own
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
    Ok(())
}

pub async fn login() -> Result<()> {
    // Generate PKCE verifier and challenge
    let verifier = generate_pkce_verifier();
    let challenge = generate_pkce_challenge(&verifier);

    let redirect_uri = format!("http://localhost:{REDIRECT_PORT}/auth/neboai/callback");
    let auth_url = format!(
        "{AUTH_URL}?client_id={CLIENT_ID}&redirect_uri={}&response_type=code&scope={}&code_challenge={challenge}&code_challenge_method=S256",
        urlencoding::encode(&redirect_uri),
        urlencoding::encode("openid profile email")
    );

    println!("Opening browser for authentication...");
    println!("If it doesn't open, visit:\n{auth_url}\n");

    open::that(&auth_url).ok();

    // Start local server to receive callback
    let server = tiny_http::Server::http(format!("127.0.0.1:{REDIRECT_PORT}"))
        .map_err(|e| anyhow::anyhow!("Failed to start local callback server: {e}"))?;

    println!("Waiting for authorization...");

    let request = server.recv().context("Failed to receive callback")?;

    let url = url::Url::parse(&format!("http://localhost{}", request.url()))?;
    let code = url
        .query_pairs()
        .find(|(k, _)| k == "code")
        .map(|(_, v)| v.to_string())
        .context("No authorization code received")?;

    // Respond to browser
    let response = tiny_http::Response::from_string(
        "<html><body><h1>Authenticated!</h1><p>You can close this tab.</p></body></html>",
    )
    .with_header(
        "Content-Type: text/html"
            .parse::<tiny_http::Header>()
            .unwrap(),
    );
    request.respond(response).ok();

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
    println!("Authenticated successfully.");
    Ok(())
}

pub async fn status() -> Result<()> {
    match load_credentials()? {
        Some(creds) => {
            if creds.is_expired() {
                println!("Status: expired (run `neboai auth login` to re-authenticate)");
            } else {
                println!("Status: authenticated");
            }
        }
        None => {
            println!("Status: not authenticated (run `neboai auth login`)");
        }
    }
    Ok(())
}

pub async fn logout() -> Result<()> {
    let path = credentials_path();
    if path.exists() {
        std::fs::remove_file(&path)?;
        println!("Logged out.");
    } else {
        println!("Not currently authenticated.");
    }
    Ok(())
}

pub async fn get_token() -> Result<String> {
    let creds = load_credentials()?.context("Not authenticated. Run `neboai auth login` first.")?;

    if creds.is_expired() {
        // TODO: implement refresh token flow
        anyhow::bail!("Token expired. Run `neboai auth login` to re-authenticate.");
    }

    Ok(creds.access_token)
}

fn generate_pkce_verifier() -> String {
    use base64::Engine;
    let mut bytes = [0u8; 32];
    getrandom(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn generate_pkce_challenge(verifier: &str) -> String {
    use base64::Engine;
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(verifier.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hash)
}

fn getrandom(buf: &mut [u8]) {
    use std::fs::File;
    use std::io::Read;
    File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(buf))
        .expect("Failed to read /dev/urandom");
}

mod urlencoding {
    pub fn encode(s: &str) -> String {
        url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
    }
}
