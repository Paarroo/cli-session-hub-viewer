pub mod generators;

/// Common CLI traits and types for multi-provider support
pub mod traits;

/// Claude CLI interaction module
pub mod claude;

/// OpenCode CLI interaction module
pub mod opencode;

/// Gemini CLI interaction module
pub mod gemini;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "dx")]
#[command(about = "Dioxus CLI - Rails-like generators for fullstack Rust apps")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Generate new code (alias: g)
    Generate {
        #[command(subcommand)]
        generator: GeneratorCommands,
    },
    /// Alias for generate
    G {
        #[command(subcommand)]
        generator: GeneratorCommands,
    },
    /// Database operations
    Db {
        #[command(subcommand)]
        action: DbCommands,
    },
    /// Setup project interactively
    Setup,
}

#[derive(Subcommand)]
pub enum GeneratorCommands {
    /// Generate a full CRUD scaffold (model + migration + API + UI)
    Scaffold {
        /// Name of the resource (e.g., User, Post)
        name: String,
        /// Fields in format: name:type[:options]
        /// Examples: title:string, email:string:unique, age:integer
        fields: Vec<String>,
        /// Skip generating certain parts
        #[arg(long)]
        skip: Vec<String>,
        /// Force overwrite existing files
        #[arg(long)]
        force: bool,
    },
    /// Generate a SeaORM model
    Model {
        /// Name of the model (e.g., User, Post)
        name: String,
        /// Fields in format: name:type[:options]
        fields: Vec<String>,
        /// Add timestamps (created_at, updated_at)
        #[arg(long)]
        timestamps: bool,
        /// Use UUID instead of auto-incrementing integer
        #[arg(long)]
        uuid: bool,
        /// Add soft delete (deleted_at)
        #[arg(long)]
        soft_delete: bool,
    },
    /// Generate API controller/routes
    Controller {
        /// Name of the controller (e.g., api/v1/users)
        name: String,
        /// Actions to generate (index, show, create, update, destroy)
        actions: Vec<String>,
        /// Generate as resource (includes all CRUD actions)
        #[arg(long)]
        resource: bool,
    },
    /// Generate authentication system
    Auth {
        /// Model name for authentication (default: User)
        model: Option<String>,
        /// Authentication method (jwt, session)
        #[arg(long, default_value = "jwt")]
        method: String,
    },
    /// Generate a UI component
    Component {
        /// Name of the component
        name: String,
        /// Props in format: name:type
        props: Vec<String>,
    },
    /// Generate a page
    Page {
        /// Name of the page/route
        name: String,
        /// Require authentication
        #[arg(long)]
        auth_required: bool,
        /// Admin only access
        #[arg(long)]
        admin_only: bool,
    },
}

#[derive(Subcommand)]
pub enum DbCommands {
    /// Setup database and run migrations
    Setup,
    /// Create database
    Create,
    /// Drop database
    Drop,
    /// Seed database with sample data
    Seed,
    /// Reset database (drop + create + migrate + seed)
    Reset,
}

pub fn run_cli() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Generate { generator } | Commands::G { generator } => {
            handle_generator(generator)
        }
        Commands::Db { action } => {
            handle_db(action)
        }
        Commands::Setup => {
            handle_setup()
        }
    }
}

fn handle_generator(generator: GeneratorCommands) -> Result<(), Box<dyn std::error::Error>> {
    match generator {
        GeneratorCommands::Model { name, fields, timestamps, uuid, soft_delete } => {
            generators::model::generate_model(&name, &fields, timestamps, uuid, soft_delete)
        }
        GeneratorCommands::Scaffold { name, fields, skip, force } => {
            generators::scaffold::generate_scaffold(&name, &fields, &skip, force)
        }
        GeneratorCommands::Controller { name, actions, resource } => {
            generators::controller::generate_controller(&name, &actions, resource)
        }
        GeneratorCommands::Auth { model, method } => {
            let model_name = model.unwrap_or_else(|| "User".to_string());
            generators::auth::generate_auth(&model_name, &method)
        }
        GeneratorCommands::Component { name, props } => {
            generators::component::generate_component(&name, &props)
        }
        GeneratorCommands::Page { name, auth_required, admin_only } => {
            generators::page::generate_page(&name, auth_required, admin_only)
        }
    }
}

fn handle_db(action: DbCommands) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        DbCommands::Setup => {
            println!("Setting up database...");
            // TODO: Implement database setup
            Ok(())
        }
        DbCommands::Create => {
            println!("Creating database...");
            // TODO: Implement database creation
            Ok(())
        }
        DbCommands::Drop => {
            println!("Dropping database...");
            // TODO: Implement database drop
            Ok(())
        }
        DbCommands::Seed => {
            println!("Seeding database...");
            // TODO: Implement seeding
            Ok(())
        }
        DbCommands::Reset => {
            println!("Resetting database...");
            // TODO: Implement reset
            Ok(())
        }
    }
}

fn handle_setup() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Welcome to Dioxus CLI Setup!");
    println!("This will help you configure your project interactively.");
    // TODO: Implement interactive setup
    Ok(())
}