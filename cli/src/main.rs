use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod commands;
mod config;
mod platform;
mod setup;
mod templates;
mod templating;

#[derive(Parser)]
#[command(name = "jffi")]
#[command(about = "JFFI - Cross-platform app framework with Rust + native UIs", long_about = None)]
#[command(version)]
struct Cli {
    /// Do not automatically install or reconcile managed build tools
    #[arg(long, global = true)]
    no_setup: bool,

    /// Disable network access and automatic tool setup
    #[arg(long, global = true)]
    offline: bool,

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

        /// Scaffold files without installing tools or running an initial build
        #[arg(long)]
        no_build: bool,
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

    /// Install missing tools and Rust targets for one platform
    Setup {
        /// Platform to prepare
        #[arg(short, long)]
        platform: String,
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

        /// Release build
        #[arg(short, long)]
        release: bool,
    },

    /// Build and run an app with debug diagnostics enabled
    Debug {
        /// Platform to debug
        #[arg(short, long, default_value = "ios")]
        platform: String,

        /// Debug on a physical device (iOS only)
        #[arg(short, long)]
        device: bool,

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

    /// Bundle the app for distribution
    Bundle {
        /// Platform to bundle (ios, android, macos, windows, linux, web, all)
        #[arg(short, long)]
        platform: String,

        /// Output format (e.g. aab, apk, dmg, app, ipa, msixbundle, flatpak)
        #[arg(short, long)]
        format: Option<String>,

        /// Signing profile to use (e.g. release, debug, ad_hoc, app_store)
        #[arg(long, default_value = "release")]
        profile: String,

        /// Disable code signing
        #[arg(long)]
        no_sign: bool,

        /// Enable notarization (macOS only)
        #[arg(long)]
        notarize: bool,

        /// Dry run: print what would be done without doing it
        #[arg(long)]
        dry_run: bool,

        /// Print the bundle plan
        #[arg(long)]
        print_plan: bool,

        /// Print exact native commands with secrets redacted
        #[arg(long)]
        print_commands: bool,
    },

    /// Diagnostics and health checks
    Doctor {
        /// Subsystem to check (e.g. bundle)
        subcommand: Option<String>,

        /// Platform to check
        #[arg(short, long)]
        platform: Option<String>,

        /// Check for release readiness
        #[arg(long)]
        release: bool,

        /// Signing profile to validate for release readiness
        #[arg(long, default_value = "release")]
        profile: String,
    },

    /// Generate icons for all platforms
    Icons {
        /// Platform to generate icons for (or all)
        #[arg(short, long, default_value = "all")]
        platform: String,
    },

    /// Generate Apple App Store and Google Play screenshots
    Screenshots,
}

fn main() -> anyhow::Result<()> {
    // Automatically unset CC_wasm32_unknown_unknown to prevent it from breaking the web build.
    // If it's in your shell profile, you can temporarily override it inline:
    // CC_wasm32_unknown_unknown=clang jffi dev --platform web
    std::env::remove_var("CC_wasm32_unknown_unknown");

    let cli = Cli::parse();

    if cli.no_setup || cli.offline {
        std::env::set_var("JFFI_NO_SETUP", "1");
    }
    if cli.offline {
        std::env::set_var("CARGO_NET_OFFLINE", "true");
    }

    match cli.command {
        Commands::New {
            name,
            platforms,
            template,
            path,
            no_build,
        } => {
            commands::new::create_project(
                &name,
                platforms.as_deref(),
                template.as_deref(),
                path,
                no_build,
            )?;
        }
        Commands::Build {
            platform,
            all,
            release,
            device,
            deploy,
            verbose,
            no_android_bridge,
        } => {
            if verbose {
                std::env::set_var("JFFI_VERBOSE", "1");
            }
            if no_android_bridge {
                std::env::set_var("JFFI_NO_ANDROID_BRIDGE", "1");
            }
            commands::build::build_project(platform, all, release, device, deploy)?;
        }
        Commands::Setup { platform } => {
            let platform = platform::Platform::from_str(&platform)
                .ok_or_else(|| anyhow::anyhow!("Unknown platform: {}", platform))?;
            setup::install_platform(&platform)?;
        }
        Commands::Run {
            platform,
            device,
            verbose,
            no_android_bridge,
            release,
        } => {
            if verbose {
                std::env::set_var("JFFI_VERBOSE", "1");
            }
            if no_android_bridge {
                std::env::set_var("JFFI_NO_ANDROID_BRIDGE", "1");
            }
            commands::run::run_project(&platform, device, release)?;
        }
        Commands::Debug {
            platform,
            device,
            no_android_bridge,
        } => {
            std::env::set_var("JFFI_VERBOSE", "1");
            std::env::set_var("RUST_BACKTRACE", "1");
            if no_android_bridge {
                std::env::set_var("JFFI_NO_ANDROID_BRIDGE", "1");
            }
            commands::run::run_project(&platform, device, false)?;
        }
        Commands::Dev {
            platform,
            verbose,
            no_android_bridge,
        } => {
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
        Commands::Bundle {
            platform,
            format,
            profile,
            no_sign,
            notarize,
            dry_run,
            print_plan,
            print_commands,
        } => {
            commands::bundle::bundle_project(commands::bundle::BundleOptions {
                platform: &platform,
                format: format.as_deref(),
                profile: &profile,
                no_sign,
                notarize,
                dry_run,
                print_plan,
                print_commands,
            })?;
        }
        Commands::Doctor {
            subcommand,
            platform,
            release,
            profile,
        } => {
            commands::doctor::run_doctor(
                subcommand.as_deref(),
                platform.as_deref(),
                release,
                &profile,
            )?;
        }
        Commands::Icons { platform } => {
            let config = crate::config::load_config()?;
            if platform == "all" {
                let platforms = vec!["ios", "android", "macos", "windows", "linux", "web"];
                for p in platforms {
                    crate::commands::icons::generate_icons(&config, p)?;
                }
            } else {
                crate::commands::icons::generate_icons(&config, &platform)?;
            }
        }
        Commands::Screenshots => {
            crate::commands::screenshots::generate_screenshots()?;
        }
    }

    Ok(())
}
