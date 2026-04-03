use anyhow::Result;
use colored::*;
use std::path::PathBuf;

pub fn create_web_project(_platforms_dir: &PathBuf, _name: &str) -> Result<()> {
    println!("  {} platforms/web/ (template coming soon)", "○".yellow());
    Ok(())
}
