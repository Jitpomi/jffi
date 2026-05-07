use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod commands;
mod config;
mod platform;
mod templating;

#[derive(Parser)]
#[command(name = "jffi")]
#[command(about = "JFFI - Cross-platform app framework with Rust + native UIs", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new cross-platform app
    New {
        /// Project name
        name: String,

        /// Target platforms (comma-separated: ios,android,macos,windows,linux,web,multi)
        #[arg(short, long)]
        platforms: Option<String>,

        /// Project template (todo, hello, counter)
        #[arg(short, long)]
        template: Option<String>,
        
        /// Project directory (defaults to current directory)
        #[arg(short = 'd', long)]
        path: Option<PathBuf>,
    },
    
    /// Build the app for specific platform(s)
    Build {
        /// Platform to build (ios, android, macos-arm64, macos-x64, windows-x64, windows-x86, linux, web)
        #[arg(short, long)]
        platform: Option<String>,
        
        /// Build all enabled platforms
        #[arg(short, long)]
        all: bool,
        
        /// Release build
        #[arg(short, long)]
        release: bool,
        
        /// Build for physical device (iOS only)
        #[arg(short, long)]
        device: bool,
        
        /// Build for deployment (all architectures for Windows, otherwise same as default)
        #[arg(long)]
        deploy: bool,
        
        /// Show verbose build output
        #[arg(short, long)]
        verbose: bool,
        
        /// Skip auto-generation of Android ndk-context JNI bridge
        #[arg(long)]
        no_android_bridge: bool,
    },
    
    /// Run the app on specific platform
    Run {
        /// Platform to run on
        #[arg(short, long, default_value = "ios")]
        platform: String,
        
        /// Run on physical device (iOS only)
        #[arg(short, long)]
        device: bool,
        
        /// Show verbose build output
        #[arg(short, long)]
        verbose: bool,
        
        /// Skip auto-generation of Android ndk-context JNI bridge
        #[arg(long)]
        no_android_bridge: bool,
    },
    
    /// Watch mode - auto-rebuild on changes
    Dev {
        /// Platform to develop for
        #[arg(short, long, default_value = "ios")]
        platform: String,
        
        /// Show verbose build output
        #[arg(short, long)]
        verbose: bool,
        
        /// Skip auto-generation of Android ndk-context JNI bridge
        #[arg(long)]
        no_android_bridge: bool,
    },
    
    /// Add a new platform to existing project
    Add {
        /// Platform to add
        platform: String,
    },
    
    /// Remove a platform from existing project
    Remove {
        /// Platform to remove
        platform: String,
    },
    
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::New { name, platforms, template, path } => {
            commands::new::create_project(&name, platforms.as_deref(), template.as_deref(), path)?;
        }
        Commands::Build { platform, all, release, device, deploy, verbose, no_android_bridge } => {
            if verbose {
                std::env::set_var("JFFI_VERBOSE", "1");
            }
            if no_android_bridge {
                std::env::set_var("JFFI_NO_ANDROID_BRIDGE", "1");
            }
            commands::build::build_project(platform, all, release, device, deploy)?;
        }
        Commands::Run { platform, device, verbose, no_android_bridge } => {
            if verbose {
                std::env::set_var("JFFI_VERBOSE", "1");
            }
            if no_android_bridge {
                std::env::set_var("JFFI_NO_ANDROID_BRIDGE", "1");
            }
            commands::run::run_project(&platform, device)?;
        }
        Commands::Dev { platform, verbose, no_android_bridge } => {
            if verbose {
                std::env::set_var("JFFI_VERBOSE", "1");
            }
            if no_android_bridge {
                std::env::set_var("JFFI_NO_ANDROID_BRIDGE", "1");
            }
            commands::dev::watch_project(&platform)?;
        }
        Commands::Add { platform } => {
            commands::add::add_platform(&platform)?;
        }
        Commands::Remove { platform } => {
            commands::remove::remove_platform(&platform)?;
        }
    }
    
    Ok(())
}
