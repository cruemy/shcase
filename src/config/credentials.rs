use std::io::Write;
use std::path::PathBuf;

use crate::error::ShcaseError;
use super::settings::get_shcase_dir;

type Result<T> = std::result::Result<T, ShcaseError>;

pub fn resolve_api_key() -> Result<String> {
    if let Ok(key) = std::env::var("GEMINI_API_KEY") {
        if !key.is_empty() {
            return Ok(key);
        }
    }

    let env_path = get_shcase_env_path();
    if env_path.exists() {
        dotenvy::from_path(&env_path).ok();
        if let Ok(key) = std::env::var("GEMINI_API_KEY") {
            if !key.is_empty() {
                return Ok(key);
            }
        }
    }

    prompt_and_save_key(&env_path)
}

fn get_shcase_env_path() -> PathBuf {
    get_shcase_dir().unwrap_or_else(|_| PathBuf::from(".")).join("env")
}

fn prompt_and_save_key(env_path: &PathBuf) -> Result<String> {
    println!("No se encontró GEMINI_API_KEY.");
    print!("Ingresá tu API key de Google AI Studio: ");
    std::io::stdout().flush()?;

    let mut key = String::new();
    std::io::stdin().read_line(&mut key)?;
    let key = key.trim().to_string();

    if key.is_empty() {
        return Err(ShcaseError::ApiKeyNotFound(
            "No se ingresó ninguna API key".into(),
        ));
    }

    if let Some(parent) = env_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let content = format!("GEMINI_API_KEY={}\n", key);
    std::fs::write(env_path, content)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = std::fs::metadata(env_path) {
            let mut perms = metadata.permissions();
            perms.set_mode(0o600);
            std::fs::set_permissions(env_path, perms).ok();
        }
    }

    println!("API key guardada en {:?}", env_path);
    Ok(key)
}
