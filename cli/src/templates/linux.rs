use anyhow::Result;
use colored::*;
use std::path::PathBuf;

pub fn create_linux_project(_platforms_dir: &PathBuf, _name: &str) -> Result<()> {
    println!("  {} platforms/linux/ (template coming soon)", "○".yellow());
    Ok(())
}
