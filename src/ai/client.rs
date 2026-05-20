use serde::{Deserialize, Serialize};

use crate::error::ShcaseError;
use super::prompts::{SYSTEM_PROMPT_EXAMEN, SYSTEM_PROMPT_RESUMEN};

type Result<T> = std::result::Result<T, ShcaseError>;

#[derive(Serialize)]
struct GeminiRequest {
    system_instruction: SystemInstruction,
    contents: Vec<Content>,
}

#[derive(Serialize)]
struct SystemInstruction {
    parts: Vec<Part>,
}

#[derive(Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Serialize, Deserialize)]
struct Part {
    text: String,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
}

#[derive(Deserialize)]
struct Candidate {
    content: Option<ResponseContent>,
}

#[derive(Deserialize)]
struct ResponseContent {
    parts: Vec<Part>,
}

pub fn call_gemini(api_key: &str, model: &str, system_prompt: &str, user_text: &str) -> Result<String> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );

    let client = reqwest::blocking::Client::new();
    let request = GeminiRequest {
        system_instruction: SystemInstruction {
            parts: vec![Part { text: system_prompt.to_string() }],
        },
        contents: vec![Content {
            parts: vec![Part { text: user_text.to_string() }],
        }],
    };

    let response = client.post(&url)
        .json(&request)
        .send()?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(ShcaseError::Gemini(format!("HTTP {}: {}", status, body)));
    }

    let data: GeminiResponse = response.json()?;

    let text = data.candidates
        .first()
        .and_then(|c| c.content.as_ref())
        .and_then(|c| c.parts.first())
        .map(|p| p.text.clone())
        .ok_or_else(|| ShcaseError::Gemini("Respuesta vacía de Gemini".into()))?;

    Ok(text)
}

pub fn enhance_content(api_key: &str, model: &str, raw_md: &str) -> Result<String> {
    let user_text = format!("Mejorá este markdown de presentación:\n\n{}", raw_md);
    call_gemini(api_key, model, SYSTEM_PROMPT_RESUMEN, &user_text)
}

pub fn generate_exam(api_key: &str, model: &str, content: &str) -> Result<Vec<ExamQuestion>> {
    let user_text = format!("Generá un examen de 7 preguntas basado en este contenido:\n\n{}", content);
    let response = call_gemini(api_key, model, SYSTEM_PROMPT_EXAMEN, &user_text)?;

    let cleaned = response.trim()
        .strip_prefix("```json")
        .or_else(|| response.strip_prefix("```"))
        .unwrap_or(&response)
        .trim()
        .strip_suffix("```")
        .unwrap_or(&response)
        .trim()
        .to_string();

    let questions: Vec<ExamQuestion> = serde_json::from_str(&cleaned)
        .map_err(|e| ShcaseError::Gemini(format!("Error parseando JSON del examen: {}", e)))?;

    Ok(questions)
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ExamQuestion {
    pub pregunta: String,
    pub respuesta: String,
}
