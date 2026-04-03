use anyhow::Result;
use colored::*;
use std::path::PathBuf;

pub fn create_macos_project(_platforms_dir: &PathBuf, _name: &str) -> Result<()> {
    println!("  {} platforms/macos/ (template coming soon)", "○".yellow());
    Ok(())
}
