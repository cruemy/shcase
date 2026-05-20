use crate::error::ShcaseError;

type Result<T> = std::result::Result<T, ShcaseError>;

pub fn get_shcase_dir() -> Result<std::path::PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| ShcaseError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound, "Cannot determine home directory")))?;
    Ok(home.join(".shcase"))
}
