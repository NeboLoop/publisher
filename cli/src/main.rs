mod api;
mod auth;
mod detect;
mod publish;
mod validate;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "neboai",
    version,
    about = "Build, validate and publish to the NeboAI marketplace"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Sign in to NeboAI
    Auth {
        #[command(subcommand)]
        action: AuthAction,
    },
    /// Validate an artifact directory
    Validate {
        /// Path to the artifact directory
        path: String,
        /// Override type detection
        #[arg(long, value_parser = ["skill", "plugin", "agent", "app", "connector", "collection"])]
        r#type: Option<String>,
    },
    /// Publish an artifact to the NeboAI marketplace
    Publish {
        /// Path to the artifact directory
        path: String,
        /// Override type detection
        #[arg(long, value_parser = ["skill", "plugin", "agent", "app", "connector", "collection"])]
        r#type: Option<String>,
        /// Marketplace visibility. "public" submits the version for review and
        /// lists it once approved; "private" and "loop" are not listed (no review).
        #[arg(long, value_parser = ["public", "private", "loop"], default_value = "public")]
        visibility: String,
        /// Accepted for compatibility; publishing is re-runnable, so running
        /// the same command again picks up where it left off.
        #[arg(long, hide = true)]
        resume: bool,
    },
    /// List the items you have published
    List,
    /// Show an item's review status
    Status {
        /// Artifact ID
        id: String,
    },
    /// Manage uploaded binaries
    Binaries {
        #[command(subcommand)]
        action: BinariesAction,
    },
}

#[derive(Subcommand)]
enum AuthAction {
    /// Sign in with your NeboAI account (opens your browser)
    Login,
    /// Show whether you are signed in, and your publisher handle
    Status,
    /// Sign out and remove the saved credentials
    Logout,
}

#[derive(Subcommand)]
enum BinariesAction {
    /// List binaries for an artifact
    List {
        /// Artifact ID
        id: String,
    },
    /// Delete a binary
    Delete {
        /// Artifact ID
        artifact_id: String,
        /// Binary ID
        binary_id: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Auth { action } => match action {
            AuthAction::Login => auth::login().await?,
            AuthAction::Status => auth::status().await?,
            AuthAction::Logout => auth::logout().await?,
        },
        Commands::Validate { path, r#type } => {
            validate::run(&path, r#type.as_deref())?;
        }
        Commands::Publish {
            path,
            r#type,
            visibility,
            resume: _,
        } => {
            publish::run(&path, r#type.as_deref(), &visibility).await?;
        }
        Commands::List => {
            api::list_artifacts().await?;
        }
        Commands::Status { id } => {
            api::get_status(&id).await?;
        }
        Commands::Binaries { action } => match action {
            BinariesAction::List { id } => {
                api::list_binaries(&id).await?;
            }
            BinariesAction::Delete { artifact_id, binary_id } => {
                api::delete_binary(&artifact_id, &binary_id).await?;
            }
        },
    }

    Ok(())
}
