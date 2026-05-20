use thiserror::Error;

#[derive(Error, Debug)]
pub enum ShcaseError {
    #[error("Error de E/S: {0}")]
    Io(#[from] std::io::Error),

    #[error("Error de ZIP: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("Error de HTTP: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("Error de YAML: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("Error de JSON: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Error de regex: {0}")]
    Regex(#[from] regex::Error),

    #[error("Template inválido: {0}")]
    Template(String),

    #[error("Slide no encontrado: {0}")]
    SlideNotFound(u32),

    #[error("Markdown inválido: {0}")]
    Markdown(String),

    #[error("API key no encontrada: {0}")]
    ApiKeyNotFound(String),

    #[error("Error de Gemini API: {0}")]
    Gemini(String),

    #[error("Entrada cancelada por el usuario")]
    Canceled,

    #[error("No se encontraron archivos: {0}")]
    NoFiles(String),
}
