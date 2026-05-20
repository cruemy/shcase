# Plan de Implementación — shcase

Generador de presentaciones PowerPoint desde Markdown, usando un template `.pptx` como base.

## CLI: Dual Mode

```
MODO SIMPLE:
  shcase input.md
    → detecta template.pptx en el directorio actual
    → parsea input.md
    → genera input.pptx automáticamente

MODO INTERACTIVO:
  shcase                        # sin args
  shcase --interactive          # explícito
    → escanea el directorio
    → menú interactivo con dialoguer:
      1. Mostrar .md disponibles y seleccionar (o auto-detecta si hay 1)
      2. Confirmar template (detecta template.pptx auto, o preguntar)
      3. Preguntar: "¿Mejorar contenido con AI?"
      4. Procesar y mostrar resultado

FLAGS:
  -t, --template <PATH>   template específico (default: busca template.pptx)
  -o, --output <PATH>     sobreescribe auto-naming (default: input.md → input.pptx)
  --codex                 habilita mejora con AI vía Responses API
  --model <NAME>          modelo (default: gpt-4o)
  --interactive           fuerza modo interactivo
  --dry-run               muestra estructura sin generar archivo
```

## Arquitectura

```
src/
├── main.rs                 # Entry point: parsea args, decide modo simple vs interactivo
├── cli.rs                  # Definición de CLI con clap
├── interactive.rs          # Menú interactivo con dialoguer
├── error.rs                # Tipos de error unificados
│
├── template/
│   ├── mod.rs
│   ├── engine.rs           # Core: open, duplicate_slide, replace_text, save
│   └── types.rs            # Template, SlideId, TemplateError
│
├── markdown/
│   ├── mod.rs
│   ├── parser.rs           # Frontmatter + ## slides → PresentationData
│   └── types.rs            # PresentationData, SlideData, CoverData
│
├── codex/
│   ├── mod.rs
│   └── client.rs           # LLM client (OpenAI Responses API)
│
└── config/
    ├── mod.rs
    └── settings.rs          # Lectura de .env, variables de entorno
```

## Fase 1: Template Engine (ZIP + XML)

El archivo `.pptx` es un ZIP con XML adentro. El engine opera directamente sobre los archivos internos.

### Formato del template

- **Slide 1**: Portada con placeholders `{{main_title}}` y `{{secondary_title}}`
- **Slide 2**: Plantilla de contenido con `{{slide_title}}` y `{{content}}`

Ambas slides pueden tener backgrounds, imágenes, logos, formas — todo se preserva porque se duplica el XML exacto.

### API del engine

```rust
let mut template = Template::open("template.pptx")?;

// Slide 0 = portada
template.replace_text(0, "{{main_title}}", &data.cover.title)?;
template.replace_text(0, "{{secondary_title}}", &data.cover.subtitle)?;

// Slide 1 = template de contenido → duplicar N veces
for slide in &data.slides {
    let new_id = template.duplicate_slide(1)?;
    template.replace_text(new_id, "{{slide_title}}", &slide.title)?;
    template.replace_text(new_id, "{{content}}", &slide.body)?;
}

template.save("output.pptx")?;
```

### duplicate_slide internals

1. Copiar `ppt/slides/slide2.xml` → `ppt/slides/slide{N}.xml`
2. Copiar `ppt/slides/_rels/slide2.xml.rels` → `ppt/slides/_rels/slide{N}.xml.rels`
3. Generar nuevo `rId` único secuencial
4. Agregar `<p:sldId>` en `ppt/presentation.xml`
5. Agregar `Relationship` en `ppt/_rels/presentation.xml.rels`
6. Agregar `Override` en `[Content_Types].xml`

### replace_text internals

- Reemplazo de texto plano sobre el XML de la slide (`Vec<u8>`)
- No necesita parser XML porque `{{placeholder}}` es único y no conflictúa con namespaces

## Fase 2: Parser de Markdown

### Formato input.md

```markdown
---
main_title: "Mi Presentación"
secondary_title: "Hecha con shcase"
---

## Introducción
- Bullet point uno
- Bullet point dos
- Bullet point tres

## Contexto
Este es un párrafo normal
que puede ocupar varias líneas.

## Conclusión
Contenido final.
```

### Estructuras

```rust
struct PresentationData {
    cover: CoverData,
    slides: Vec<SlideData>,
}

struct CoverData {
    main_title: String,
    secondary_title: String,
}

struct SlideData {
    title: String,
    body: String,
}
```

### Parseo

1. Split por `---`: primero es frontmatter (YAML), segundo es body
2. Frontmatter se parsea con `serde_yaml`
3. Body: split por regex `^## ` → bloques
4. Primer línea de cada bloque = título, resto = body

## Fase 3: CLI (clap + dialoguer)

### cli.rs

```rust
#[derive(Parser)]
struct Cli {
    input: Option<String>,

    #[arg(short, long)]
    template: Option<String>,

    #[arg(short, long)]
    output: Option<String>,

    #[arg(long)]
    codex: bool,

    #[arg(long, default_value = "gpt-4o")]
    model: String,

    #[arg(long)]
    interactive: bool,

    #[arg(long)]
    dry_run: bool,
}
```

### interactive.rs (dialoguer)

```rust
fn run_interactive() -> Result<ProcessConfig> {
    // 1. Detectar archivos .md
    let md_files: Vec<_> = glob("*.md")?.collect();
    // Si hay 1 solo → lo usa directo
    // Si hay varios → Select::new() para elegir

    // 2. Detectar template.pptx
    // Si existe → lo usa automático
    // Si no → Input::new() preguntando path

    // 3. Preguntar Codex
    Confirm::new()
        .with_prompt("¿Mejorar contenido con AI?")

    // 4. Mostrar resumen y confirmar
}
```

## Fase 4: Codex Integration

### Modos de autenticación

| Modo | Variable | Source |
|------|----------|--------|
| API Key | `OPENAI_API_KEY` | `.env` o variable de entorno |
| Codex Subscription | `CODEX_ACCESS_TOKEN` | Token de `codex login` |

### Llamada a Responses API

```rust
POST https://api.openai.com/v1/responses
Authorization: Bearer {token}

{
    "model": "gpt-4o",
    "input": "Improve this presentation markdown...\n\n{raw_md}",
    "instructions": "You are a presentation expert. Improve structure and content.",
    "store": false
}
```

### Flujo

```
input.md ──> [Codex] ──> markdown mejorado ──> parser ──> template engine ──> output.pptx
```

Si no hay token disponible, el flag `--codex` muestra un warning y sigue sin AI.

## Dependencias

```toml
[dependencies]
zip = { version = "2", features = ["deflate"] }
quick-xml = "0.37"
clap = { version = "4", features = ["derive"] }
dialoguer = "0.12"
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
reqwest = { version = "0.12", features = ["json"] }
tokio = { version = "1", features = ["full"] }
thiserror = "2"
dotenvy = "0.15"
glob = "0.3"
```

## Decisiones Técnicas

| Decisión | Elección | Motivo |
|----------|----------|--------|
| Template engine | `zip` + `quick-xml` | Única forma de duplicar slides preservando backgrounds, imágenes y relaciones. Crates de alto nivel (ppt-rs, pptx) no soportan `duplicate_slide`. |
| CLI framework | `clap` | Estándar Rust, derive API, manejo de subcomandos |
| Interactive prompts | `dialoguer` | Liviano (2 deps), del equipo console-rs, cubre select/confirm/input |
| Frontmatter | `serde_yaml` | YAML es más legible que TOML para metadata |
| Codex | `reqwest` + Responses API | Llamada HTTP directa, sin dependencias pesadas adicionales |
| Output naming | Auto: `input.md` → `input.pptx` | Simple, predecible, sin preguntar |
