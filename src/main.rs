mod cli;
mod config;
mod error;
mod examen;
mod interactive;
mod markdown;
mod pptx;
mod template;
mod ai;

use std::path::{Path, PathBuf};

use clap::Parser;
use cli::Cli;
use error::ShcaseError;
use interactive::{ProcessConfig, ProcessMode};
use markdown::parser::parse_markdown;
use markdown::types::PresentationData;

type Result<T> = std::result::Result<T, ShcaseError>;

fn main() {
    let cli = Cli::parse();

    let result = run(cli);

    match result {
        Ok(msg) => println!("{}", msg),
        Err(ShcaseError::Canceled) => {
            println!("Operación cancelada.");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn run(cli: Cli) -> Result<String> {
    if cli.from_pptx.is_some() {
        return run_standalone_exam(&cli);
    }

    if cli.interactive || cli.input.is_none() {
        let config = interactive::run_interactive(&cli)?;
        return run_with_config(&cli, config);
    }

    run_simple_mode(&cli)
}

fn run_simple_mode(cli: &Cli) -> Result<String> {
    let input_path = cli.input.as_ref().unwrap();
    let input_path = Path::new(input_path);

    let md = std::fs::read_to_string(input_path)?;
    let data = parse_markdown(&md)?;

    let output_path = resolve_output(cli, input_path, "pptx");

    if cli.dry_run {
        return print_dry_run(&data, Path::new("N/A (dry-run)"), &output_path, cli.ai);
    }

    let template_path = resolve_template(cli, input_path)?;

    let enhanced = if cli.ai {
        let api_key = config::credentials::resolve_api_key()?;
        println!("Mejorando contenido con IA...");
        let response = ai::client::enhance_content(&api_key, &cli.model, &md)?;
        parse_enhanced_response(&response, &data.slides.len())?
    } else {
        data.slides.clone()
    };

    println!("Generando presentación...");
    let mut template = template::types::Template::open(&template_path)?;

    template.replace_text(1, "main_title", &data.cover.main_title)?;
    template.replace_text(1, "secondary_title", &data.cover.secondary_title)?;

    for (i, slide) in data.slides.iter().enumerate() {
        let new_id = template.duplicate_slide(2)?;
        template.replace_text(new_id, "slide_title", &enhanced[i].title)?;
        template.replace_text(new_id, "content", &enhanced[i].body)?;
        template.set_notes(new_id, &slide.body)?;
    }

    template.remove_slide(2)?;
    template.save(&output_path)?;
    println!("Presentación generada: {}", output_path);

    let mut result = format!("PPTX generado: {}", output_path);

    if !cli.no_examen {
        let exam_path = generate_exam_output(cli, input_path, &data, &md)?;
        result.push_str(&format!("\n✅ Examen generado: {}", exam_path));
    }

    Ok(result)
}

fn run_standalone_exam(cli: &Cli) -> Result<String> {
    let pptx_path = cli.from_pptx.as_ref().unwrap();
    let pptx_path = Path::new(pptx_path);

    println!("Extrayendo texto del PPTX...");
    let text = pptx::extractor::extract_text_from_pptx(pptx_path)?;

    let api_key = config::credentials::resolve_api_key()?;
    println!("Generando examen con IA...");
    let questions = ai::client::generate_exam(&api_key, &cli.model, &text)?;

    let stem = pptx_path.file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "presentacion".to_string());
    let exam_path = examen::generator::write_exam_file(pptx_path, &questions, &stem)?;
    Ok(format!("✅ Examen generado: {}", exam_path))
}

fn run_with_config(cli: &Cli, config: ProcessConfig) -> Result<String> {
    match config.mode {
        ProcessMode::MdToPptx => run_interactive_md(cli, config),
        ProcessMode::PptxToExam => run_interactive_pptx(cli, config),
    }
}

fn run_interactive_md(cli: &Cli, config: ProcessConfig) -> Result<String> {
    let md = std::fs::read_to_string(&config.input_path)?;
    let data = parse_markdown(&md)?;

    let template_path = config.template_path.as_ref().unwrap();
    let output_path = config.output_path.clone()
        .unwrap_or_else(|| PathBuf::from(resolve_output(cli, &config.input_path, "pptx")));

    if config.dry_run {
        return print_dry_run(&data, template_path, output_path.to_str().unwrap(), config.use_ai);
    }

    let enhanced = if config.use_ai {
        let api_key = config::credentials::resolve_api_key()?;
        println!("Mejorando contenido con IA...");
        let response = ai::client::enhance_content(&api_key, &config.model, &md)?;
        parse_enhanced_response(&response, &data.slides.len())?
    } else {
        data.slides.clone()
    };

    println!("Generando presentación...");
    let mut template = template::types::Template::open(template_path)?;

    template.replace_text(1, "main_title", &data.cover.main_title)?;
    template.replace_text(1, "secondary_title", &data.cover.secondary_title)?;

    for (i, slide) in data.slides.iter().enumerate() {
        let new_id = template.duplicate_slide(2)?;
        template.replace_text(new_id, "slide_title", &enhanced[i].title)?;
        template.replace_text(new_id, "content", &enhanced[i].body)?;
        template.set_notes(new_id, &slide.body)?;
    }

    template.remove_slide(2)?;
    template.save(&output_path)?;

    let mut result = format!("PPTX generado: {}", output_path.display());

    if !config.no_examen {
        let exam_path = generate_exam_output(cli, &config.input_path, &data, &md)?;
        result.push_str(&format!("\n✅ Examen generado: {}", exam_path));
    }

    Ok(result)
}

fn run_interactive_pptx(_cli: &Cli, config: ProcessConfig) -> Result<String> {
    println!("Extrayendo texto del PPTX...");
    let text = pptx::extractor::extract_text_from_pptx(&config.input_path)?;

    let api_key = config::credentials::resolve_api_key()?;
    println!("Generando examen con IA...");
    let questions = ai::client::generate_exam(&api_key, &config.model, &text)?;

    let stem = config.input_path.file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "presentacion".to_string());
    let exam_path = examen::generator::write_exam_file(&config.input_path, &questions, &stem)?;
    Ok(format!("✅ Examen generado: {}", exam_path))
}

fn resolve_template(cli: &Cli, input_path: &Path) -> Result<PathBuf> {
    if let Some(ref t) = cli.template {
        return Ok(PathBuf::from(t));
    }

    let cwd = std::env::current_dir()?;
    let local_template = cwd.join("template.pptx");
    if local_template.exists() {
        return Ok(local_template);
    }

    let input_dir = input_path.parent().unwrap_or(Path::new("."));
    let dir_template = input_dir.join("template.pptx");
    if dir_template.exists() {
        return Ok(dir_template);
    }

    Err(ShcaseError::Template(
        "No se encontró template.pptx en el directorio actual ni en el del archivo de entrada".into(),
    ))
}

fn resolve_output(cli: &Cli, input_path: &Path, ext: &str) -> String {
    if let Some(ref o) = cli.output {
        return o.clone();
    }

    let stem = input_path.file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "output".to_string());

    let parent = input_path.parent().unwrap_or(Path::new("."));
    parent.join(format!("{}.{}", stem, ext)).to_string_lossy().to_string()
}

fn print_dry_run(
    data: &PresentationData,
    template_path: &Path,
    output_path: &str,
    ai_enabled: bool,
) -> Result<String> {
    println!("=== DRY RUN ===");
    println!("Template: {}", template_path.display());
    println!("Output: {}", output_path);
    println!("AI: {}", if ai_enabled { "habilitado" } else { "deshabilitado" });
    println!("Portada: '{}' / '{}'", data.cover.main_title, data.cover.secondary_title);
    println!("Slides: {}", data.slides.len());
    for (i, slide) in data.slides.iter().enumerate() {
        println!("  {}. {} ({} caracteres)", i + 1, slide.title, slide.body.len());
    }
    println!("================");
    Ok("Dry run completado. No se generaron archivos.".into())
}

fn parse_enhanced_response(
    response: &str,
    expected_count: &usize,
) -> Result<Vec<markdown::types::SlideData>> {
    let response = response.trim();

    let slides = if response.starts_with('[') || response.starts_with('{') {
        parse_json_slides(response)?
    } else {
        parse_markdown(response)?.slides
    };

    if slides.len() != *expected_count {
        return Err(ShcaseError::Gemini(
            format!("Se esperaban {} slides pero Gemini devolvió {}", expected_count, slides.len())
        ));
    }

    Ok(slides)
}

fn parse_json_slides(json: &str) -> Result<Vec<markdown::types::SlideData>> {
    #[derive(serde::Deserialize)]
    struct JsonSlide {
        title: String,
        #[serde(default)]
        body: String,
        #[serde(default)]
        content: String,
    }

    let slides: Vec<JsonSlide> = serde_json::from_str(json)?;
    Ok(slides.into_iter().map(|s| markdown::types::SlideData {
        title: s.title,
        body: if s.body.is_empty() { s.content } else { s.body },
    }).collect())
}

fn generate_exam_output(
    cli: &Cli,
    input_path: &Path,
    data: &PresentationData,
    raw_md: &str,
) -> Result<String> {
    let api_key = config::credentials::resolve_api_key()?;

    let main_title = &data.cover.main_title;

    if cli.ai {
        println!("Generando examen con IA...");
        let questions = ai::client::generate_exam(&api_key, &cli.model, raw_md)?;
        examen::generator::write_exam_file(input_path, &questions, main_title)
    } else {
        let content = format_presentation_for_exam(data);
        println!("Generando examen con IA (contenido sin mejorar)...");
        let questions = ai::client::generate_exam(&api_key, &cli.model, &content)?;
        examen::generator::write_exam_file(input_path, &questions, main_title)
    }
}

fn format_presentation_for_exam(data: &PresentationData) -> String {
    let mut md = String::new();
    md.push_str(&format!("# {}\n", data.cover.main_title));
    md.push_str(&format!("{}\n\n", data.cover.secondary_title));
    for slide in &data.slides {
        md.push_str(&format!("## {}\n{}\n\n", slide.title, slide.body));
    }
    md
}
