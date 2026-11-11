//! Shared filesystem boundaries for command inputs and outputs.
use anyhow::Result;
use cm_exchange_plan::Plan;
use std::{fs, path::Path};

/// Write an artifact, creating parent directories when necessary.
pub fn write(path: &Path, text: &str) -> Result<()> {
    if let Some(parent) = path.parent().filter(|x| !x.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, text)?;
    Ok(())
}
/// Load and validate a saved plan before any mutating operation.
pub fn load_plan(path: &Path) -> Result<Plan> {
    let plan: Plan = cm_catalog::records::decode(&fs::read(path)?)?;
    plan.validate()?;
    Ok(plan)
}

/// Save an editable record while preserving comments from its previous contents.
pub fn write_record<T: serde::Serialize>(path: &Path, value: &T) -> Result<()> {
    let next = cm_catalog::records::encode(value)?;
    let text = match fs::read_to_string(path) {
        Ok(previous) => cm_catalog::records::preserve(&previous, &next)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => next,
        Err(error) => return Err(error.into()),
    };
    write(path, &text)
}
