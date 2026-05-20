use std::path::Path;

use crate::ai::client::ExamQuestion;
use crate::error::ShcaseError;

type Result<T> = std::result::Result<T, ShcaseError>;

pub fn write_exam_file(base_path: &Path, questions: &[ExamQuestion], main_title: &str) -> Result<String> {
    let stem = base_path.file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "output".to_string());

    let parent = base_path.parent().unwrap_or(Path::new("."));
    let exam_path = parent.join(format!("{}-examen.md", stem));

    let mut md = String::new();
    md.push_str(&format!("# Examen: {}\n\n", main_title));

    for (i, q) in questions.iter().enumerate() {
        md.push_str(&format!("{}. {}\nR={}\n\n\n", i + 1, q.pregunta, q.respuesta));
    }

    std::fs::write(&exam_path, md)?;
    Ok(exam_path.to_string_lossy().to_string())
}

