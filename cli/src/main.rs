mod auth;
mod api;
mod detect;
mod publish;
mod validate;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "neboai", version, about = "Publish to the NeboLoop marketplace")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Authenticate with NeboLoop
    Auth {
        #[command(subcommand)]
        action: AuthAction,
    },
    /// Validate an artifact directory
    Validate {
        /// Path to the artifact directory
        path: String,
        /// Override type detection
        #[arg(long, value_parser = ["skill", "plugin", "agent", "app"])]
        r#type: Option<String>,
    },
    /// Publish an artifact to NeboLoop
    Publish {
        /// Path to the artifact directory
        path: String,
        /// Override type detection
        #[arg(long, value_parser = ["skill", "plugin", "agent", "app"])]
        r#type: Option<String>,
        /// Resume a failed publish
        #[arg(long)]
        resume: bool,
    },
    /// List your published artifacts
    List,
    /// Check submission status
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
    /// Log in via OAuth (opens browser)
    Login,
    /// Check authentication status
    Status,
    /// Log out and clear credentials
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
        /// Binary ID
        id: String,
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
            resume,
        } => {
            publish::run(&path, r#type.as_deref(), resume).await?;
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
            BinariesAction::Delete { id } => {
                api::delete_binary(&id).await?;
            }
        },
    }

    Ok(())
}
