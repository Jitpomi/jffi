use anyhow::{Context, Result};
use include_dir::{include_dir, Dir};
use std::path::PathBuf;

static EMBEDDED_TEMPLATES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates");

pub fn find_templates_dir() -> Result<PathBuf> {
    let development = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates");
    if std::env::var("JFFI_FORCE_EMBEDDED_TEMPLATES").as_deref() != Ok("1") && development.is_dir()
    {
        return Ok(development);
    }

    let destination =
        std::env::temp_dir().join(format!("jffi-{}-templates", env!("CARGO_PKG_VERSION")));
    if !destination.is_dir() {
        std::fs::create_dir_all(&destination).with_context(|| {
            format!(
                "Failed to create template cache at {}",
                destination.display()
            )
        })?;
        EMBEDDED_TEMPLATES.extract(&destination).with_context(|| {
            format!(
                "Failed to extract embedded templates to {}",
                destination.display()
            )
        })?;
    }
    Ok(destination)
}
