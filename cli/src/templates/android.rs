use anyhow::Result;
use colored::*;
use std::path::PathBuf;

pub fn create_android_project(_platforms_dir: &PathBuf, _name: &str) -> Result<()> {
    println!("  {} platforms/android/ (template coming soon)", "○".yellow());
    Ok(())
}
