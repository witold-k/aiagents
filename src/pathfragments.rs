use std::path::{Path, PathBuf};

/// Expand path placeholders as text, without canonicalizing the output.
pub fn expand_path_fragments(input: &str, project_dir: &Path, home_dir: &Path) -> Result<String, String> {
    let project = project_dir.canonicalize().map_err(|e| e.to_string())?;
    let home = home_dir.canonicalize().map_err(|e| e.to_string())?;
    let relative = if input.contains("{{relprojectdir}}") {
        Some(project.strip_prefix(&home).map_err(|_| "project directory is outside HOME")?)
    } else { None };
    let mut result = input.replace("{{projectdir}}", &project.to_string_lossy())
        .replace("{{home}}", &home.to_string_lossy());
    if let Some(relative) = relative {
        result = result.replace("{{relprojectdir}}", &relative.to_string_lossy());
    }
    if result.contains("{{") || result.contains("}}") {
        return Err(format!("unknown or incomplete placeholder in '{input}'"));
    }
    Ok(result)
}

pub fn expand_current_path_fragments(input: &str) -> Result<String, String> {
    let home = std::env::var_os("HOME").ok_or("HOME is not set")?;
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    expand_path_fragments(input, &cwd, &PathBuf::from(home))
}
