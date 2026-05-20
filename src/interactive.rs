use std::path::{Path, PathBuf};

use dialoguer::{Confirm, Select};

use crate::cli::Cli;
use crate::error::ShcaseError;

type Result<T> = std::result::Result<T, ShcaseError>;

#[derive(Debug)]
pub struct ProcessConfig {
    pub mode: ProcessMode,
    pub input_path: PathBuf,
    pub template_path: Option<PathBuf>,
    pub output_path: Option<PathBuf>,
    pub use_ai: bool,
    pub model: String,
    pub no_examen: bool,
    pub dry_run: bool,
}

#[derive(Debug, PartialEq)]
pub enum ProcessMode {
    MdToPptx,
    PptxToExam,
}

pub fn run_interactive(cli: &Cli) -> Result<ProcessConfig> {
    let mode = Select::new()
        .with_prompt("¿Qué querés hacer?")
        .item("Generar PPTX desde un archivo MD")
        .item("Generar examen desde un PPTX existente")
        .default(0)
        .interact()
        .map_err(|_| ShcaseError::Canceled)?;

    match mode {
        0 => handle_md_to_pptx(cli),
        1 => handle_pptx_to_exam(cli),
        _ => Err(ShcaseError::Canceled),
    }
}

fn find_files(dir: &Path, ext: &str) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let dir_str = dir.to_string_lossy();
    #[cfg(windows)]
    let dir_str = dir_str.replace('\\', "/");
    let pattern = format!("{}/*{}", dir_str, ext);
    for entry in glob::glob(&pattern).map_err(|e| ShcaseError::Io(
        std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
    ))? {
        match entry {
            Ok(path) => files.push(path),
            Err(_) => continue,
        }
    }
    files.sort();
    Ok(files)
}

fn select_file(files: &[PathBuf], prompt: &str) -> Result<PathBuf> {
    if files.is_empty() {
        return Err(ShcaseError::NoFiles(format!("No se encontraron archivos")));
    }
    if files.len() == 1 {
        println!("Usando: {}", files[0].display());
        return Ok(files[0].clone());
    }

    let items: Vec<String> = files.iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect();
    let refs: Vec<&str> = items.iter().map(|s| s.as_str()).collect();

    let idx = Select::new()
        .with_prompt(prompt)
        .items(&refs)
        .default(0)
        .interact()
        .map_err(|_| ShcaseError::Canceled)?;

    Ok(files[idx].clone())
}

fn handle_md_to_pptx(cli: &Cli) -> Result<ProcessConfig> {
    let cwd = std::env::current_dir()?;

    let md_files = find_files(&cwd, ".md")?;
    let md_files: Vec<PathBuf> = md_files.into_iter()
        .filter(|p| !p.to_string_lossy().contains("-examen"))
        .collect();

    let input_path = if let Some(ref input) = cli.input {
        PathBuf::from(input)
    } else {
        select_file(&md_files, "Seleccioná el archivo MD")?
    };

    let template_path = if let Some(ref t) = cli.template {
        Some(PathBuf::from(t))
    } else {
        let template_candidate = cwd.join("template.pptx");
        if template_candidate.exists() {
            println!("Template detectado: template.pptx");
            Some(template_candidate)
        } else {
            println!("No se encontró template.pptx en el directorio actual.");
            let pptx_files = find_files(&cwd, ".pptx")?;
            let pptx_files: Vec<PathBuf> = pptx_files.into_iter()
                .filter(|p| !p.to_string_lossy().contains("-examen"))
                .collect();
            Some(select_file(&pptx_files, "Seleccioná el archivo template PPTX")?)
        }
    };

    let use_ai = if cli.ai {
        true
    } else {
        Confirm::new()
            .with_prompt("¿Mejorar contenido con IA?")
            .default(false)
            .interact()
            .map_err(|_| ShcaseError::Canceled)?
    };

    let no_examen = if cli.no_examen {
        true
    } else {
        !Confirm::new()
            .with_prompt("¿Generar examen?")
            .default(true)
            .interact()
            .map_err(|_| ShcaseError::Canceled)?
    };

    Ok(ProcessConfig {
        mode: ProcessMode::MdToPptx,
        input_path,
        template_path,
        output_path: cli.output.as_ref().map(PathBuf::from),
        use_ai,
        model: cli.model.clone(),
        no_examen,
        dry_run: cli.dry_run,
    })
}

fn handle_pptx_to_exam(cli: &Cli) -> Result<ProcessConfig> {
    let cwd = std::env::current_dir()?;

    let input_path = if let Some(ref from) = cli.from_pptx {
        PathBuf::from(from)
    } else {
        let pptx_files = find_files(&cwd, ".pptx")?;
        select_file(&pptx_files, "Seleccioná el archivo PPTX")?
    };

    Ok(ProcessConfig {
        mode: ProcessMode::PptxToExam,
        input_path,
        template_path: None,
        output_path: None,
        use_ai: true,
        model: cli.model.clone(),
        no_examen: false,
        dry_run: cli.dry_run,
    })
}
