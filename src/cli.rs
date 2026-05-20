use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "shcase", version, about = "Generador de presentaciones PowerPoint desde Markdown")]
pub struct Cli {
    pub input: Option<String>,

    #[arg(short, long)]
    pub template: Option<String>,

    #[arg(short, long)]
    pub output: Option<String>,

    #[arg(long)]
    pub ai: bool,

    #[arg(long)]
    pub from_pptx: Option<String>,

    #[arg(long)]
    pub no_examen: bool,

    #[arg(long, default_value = "gemini-3.1-flash-lite")]
    pub model: String,

    #[arg(short, long)]
    pub interactive: bool,

    #[arg(long)]
    pub dry_run: bool,
}
