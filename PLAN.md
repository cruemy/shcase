# Plan de Implementación — shcase

Generador de presentaciones PowerPoint desde Markdown, usando un template `.pptx` como base.
Con opción de mejora de contenido via Google Gemini AI y generación automática de exámenes.

---

## CLI: Dual Mode

```
MODO SIMPLE:
  shcase input.md
    → detecta template.pptx en el directorio actual
    → parsea input.md
    → [--ai] mejora contenido con Gemini AI
    → genera input.pptx + input-examen.md

MODO INTERACTIVO:
  shcase                        # sin args
  shcase --interactive          # explícito
    → escanea el directorio
    → menú interactivo con dialoguer:
      1. Elegir modo: generar PPTX o analizar PPTX existente
      2. Seleccionar archivo (MD o PPTX según el modo)
      3. Confirmar template (detecta template.pptx auto si aplica)
      4. Preguntar: "¿Mejorar contenido con IA?"
      5. Procesar y mostrar resultado

MODO STANDALONE (examen desde PPTX externo):
  shcase --from-pptx presentacion.pptx
    → extrae texto de todas las slides
    → genera presentacion-examen.md (7 preguntas + respuestas)

FLAGS:
  -t, --template <PATH>   template específico (default: busca template.pptx)
  -o, --output <PATH>     sobreescribe auto-naming (default: input.md → input.pptx)
  --ai                    habilita mejora con Google Gemini AI
  --from-pptx <PATH>      modo standalone: extrae texto de PPTX y genera examen
  --no-examen             saltea generación del examen (default: siempre genera)
  --model <NAME>          modelo Gemini (default: gemini-3.1-flash-lite)
  --interactive           fuerza modo interactivo
  --dry-run               muestra estructura sin generar archivo
```

---

## Arquitectura

```
src/
├── main.rs                 # Entry point: parsea args, decide modo
├── cli.rs                  # Definición de CLI con clap
├── interactive.rs          # Menú interactivo con dialoguer
├── error.rs                # Tipos de error unificados
│
├── template/
│   ├── mod.rs
│   ├── engine.rs           # Core: open, duplicate_slide, replace_text, set_notes, save
│   └── types.rs            # Template, SlideId, TemplateError
│
├── markdown/
│   ├── mod.rs
│   ├── parser.rs           # Frontmatter + ## slides → PresentationData
│   └── types.rs            # PresentationData, SlideData, CoverData
│
├── ai/
│   ├── mod.rs
│   ├── client.rs           # Gemini API client
│   └── prompts.rs          # System prompts para resumen y examen
│
├── examen/
│   ├── mod.rs
│   └── generator.rs        # Genera examen.md con preguntas + respuestas
│
├── pptx/
│   ├── mod.rs
│   └── extractor.rs        # Extrae texto de slides de un PPTX existente
│
└── config/
    ├── mod.rs
    ├── settings.rs          # Lectura de .env, variables de entorno
    └── credentials.rs       # Resolución y guardado de API key
```

---

## Fase 1: Template Engine (ZIP + XML)

El archivo `.pptx` es un ZIP con XML adentro. El engine opera directamente sobre los archivos internos usando `zip` + `quick-xml`.

### Formato del template

- **Slide 1**: Portada con placeholders `{{main_title}}` y `{{secondary_title}}`
- **Slide 2**: Plantilla de contenido con `{{slide_title}}` y `{{content}}`
- **Slide 2 debe tener** una slide de notas (notes slide) asociada, aunque esté vacía

Ambas slides pueden tener backgrounds, imágenes, logos, formas — todo se preserva porque se duplica el XML exacto junto con sus relaciones.

### API del engine

```rust
let mut template = Template::open("template.pptx")?;

// Slide 0 = portada
template.replace_text(0, "{{main_title}}", &data.cover.title)?;
template.replace_text(0, "{{secondary_title}}", &data.cover.subtitle)?;

// Slide 1 = template de contenido → duplicar N veces
for (i, slide) in data.slides.iter().enumerate() {
    let new_id = template.duplicate_slide(1)?;

    // Body con versión ultra explicativa (generada por Gemini)
    template.replace_text(new_id, "{{slide_title}}", &enhanced[i].title)?;
    template.replace_text(new_id, "{{content}}", &enhanced[i].body)?;

    // Speaker notes con el contenido técnico original
    template.set_notes(new_id, &slide.body)?;
}

template.save("output.pptx")?;
```

### duplicate_slide internals

1. Copiar `ppt/slides/slide2.xml` → `ppt/slides/slide{N}.xml`
2. Copiar `ppt/slides/_rels/slide2.xml.rels` → `ppt/slides/_rels/slide{N}.xml.rels`
3. Si existe `ppt/notesSlides/notesSlide2.xml`, copiar a `notesSlide{N}.xml`
4. Si existe `ppt/notesSlides/_rels/notesSlide2.xml.rels`, copiar también
5. Generar nuevo `rId` único secuencial
6. Agregar `<p:sldId>` en `ppt/presentation.xml`
7. Agregar `Relationship` en `ppt/_rels/presentation.xml.rels`
8. Agregar `Override` en `[Content_Types].xml`

### replace_text internals

Reemplazo de texto plano sobre el XML de la slide (`Vec<u8>`). No necesita parser XML porque `{{placeholder}}` es único y no conflictúa con namespaces de OpenXML.

### set_notes internals

1. Revisar si existe `ppt/notesSlides/notesSlide{N}.xml`
2. Si no existe: crear XML de notes slide desde template hardcodeado
3. Agregar relación en `ppt/slides/_rels/slide{N}.xml.rels` apuntando al notes slide
4. Agregar `Override` en `[Content_Types].xml` si no existe ya
5. Reemplazar el contenido textual del `<a:t>` en el XML de notes

---

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

1. Normalizar `\r\n` → `\n` (cross-platform)
2. Split por `---`: primero es frontmatter (YAML), segundo es body
3. Frontmatter se parsea con `serde_yaml`
4. Body: split por regex `^## ` → bloques
5. Primera línea de cada bloque = título, resto = body

---

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
    ai: bool,

    #[arg(long)]
    from_pptx: Option<String>,

    #[arg(long)]
    no_examen: bool,

    #[arg(long, default_value = "gemini-3.1-flash-lite")]
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
    // 1. Elegir modo
    let mode = Select::new()
        .with_prompt("¿Qué querés hacer?")
        .item("Generar PPTX desde un archivo MD")
        .item("Generar examen desde un PPTX existente")
        .interact()?;

    match mode {
        0 => {
            // a) Modo PPTX desde MD
            //    - Seleccionar .md (auto-detecta si hay 1)
            //    - Confirmar template (detecta template.pptx auto)
            //    - Confirmar mejora con AI
            //    - Mostrar resumen y procesar
        }
        1 => {
            // b) Modo standalone: examen desde PPTX
            //    - Seleccionar .pptx
            //    - Procesar
        }
    }
}
```

---

## Fase 4: Gemini AI Integration

### Autenticación y Key Management

Orden de resolución:

```
1. Variable de entorno GEMINI_API_KEY
   └─ ¿existe? → la usa

2. Archivo ~/.shcase/env
   └─ ¿existe GEMINI_API_KEY ahí? → la usa

3. ¿No encontró nada? → prompt interactivo
   └─ "No se encontró GEMINI_API_KEY.
       Ingresá tu API key de Google AI Studio: [______]"
   └─ Se guarda automáticamente en ~/.shcase/env (con permisos 600 en Unix)
   └─ La usa
```

Storage: `$HOME/.shcase/env` con formato `clave=valor` estándar, leído con `dotenvy`.

### API Call a Gemini

```rust
POST https://generativelanguage.googleapis.com/v1beta/models/gemini-3.1-flash-lite:generateContent?key={GEMINI_API_KEY}

{
    "system_instruction": {
        "parts": [{ "text": "<SYSTEM_PROMPT>" }]
    },
    "contents": [{
        "parts": [{ "text": "Mejorá este markdown de presentación:\n\n{raw_md}" }]
    }]
}
```

### Prompts del sistema (ai/prompts.rs)

**SYSTEM_PROMPT_RESUMEN** — Transforma contenido técnico a ultra explicativo:

```
Eres un experto en comunicación clara. Tu tarea es transformar contenido
técnico en explicaciones que cualquier persona sin conocimientos previos
pueda entender.

Reglas:
1. Cada concepto debe explicarse COMPLETAMENTE, sin dar nada por sentado.
2. No uses "esto", "ello", "lo mismo" — siempre repetí el sujeto.
3. Si mencionás un término, explicá QUÉ significa inmediatamente después.
4. Usá analogías de la vida cotidiana cuando sea posible.
5. Cada slide debe ser auto-contenida.
6. Devolvé la misma estructura de slides (mismos títulos) pero con contenido
   transformado a versión ultra explicativa.
```

**SYSTEM_PROMPT_EXAMEN** — Genera 7 preguntas + respuestas:

```
Eres un evaluador educativo. Generás exámenes para personas sin conocimiento
técnico previo.

Las preguntas deben ser VARIADAS en tipo. NO todas iguales. La prioridad es:

1. Preguntas de "Sí o No": la persona responde si algo se hace o no se hace,
   si algo existe o no existe. La respuesta debe explicar BREVEMENTE por qué
   es sí o por qué es no. Estas son las MÁS IMPORTANTES, deben ser la mayoría.
2. Preguntas de "Explicame cómo": la persona debe describir los pasos para
   hacer algo (ej: "¿Cómo se hace tal cosa en la plataforma?").
   La respuesta debe detallar el PASO A PASO. Estas son las segunda prioridad.
3. Preguntas de escenario: planteá una situación y preguntá qué haría
   la persona, o qué significa algo. Máximo UNA de este tipo por examen,
   si es que corresponde. No es obligatoria.

REGLAS GENERALES (obligatorias para TODAS las preguntas):
0. El examen debe tener EXACTAMENTE 7 preguntas. Ni una más, ni una menos.
1. Cada pregunta debe ser AUTO-CONTENIDA: explicá el contexto completo.
2. NO uses: "según el texto", "como se mencionó", "de acuerdo a lo visto",
   "anteriormente", "como vimos". NADA de inferencia.
3. En cambio: describí brevemente el escenario ANTES de preguntar.
   Cada pregunta debe leerse sola y entenderse sin haber leído las otras.
4. NO se necesita explicar "por qué" en todas. Solo si la pregunta lo amerita.
5. El lenguaje debe ser CLARO y SENCILLO, como si hablaras con alguien que
   nunca usó la herramienta ni conoce el tema.

Formato de salida: JSON array con objetos { pregunta: string, respuesta: string }
```

---

## Fase 5: Examen Generator

### Flujo

Cuando se genera PPTX (modo normal):

```
input.md
  → [Gemini AI] resumen ultra explicativo → slides del PPTX
  → [Gemini AI] examen 7 preguntas + respuestas
  → examen/generator.rs escribe input-examen.md
```

Con `--from-pptx` (modo standalone):

```
externo.pptx
  → pptx/extractor.rs extrae texto de cada slide
  → [Gemini AI] examen 7 preguntas + respuestas
  → examen/generator.rs escribe externo-examen.md
```

### Formato del examen.md

Con ejemplos de cada tipo de pregunta:

```markdown
# Examen: Gestión de Flota

## Pregunta 1 (Sí o No)
En una empresa de logística, los conductores usan una aplicación en su
teléfono para registrar cuando empiezan y terminan su jornada laboral.

¿El conductor necesita tener internet en el teléfono para poder marcar
el inicio de su jornada?

**Respuesta:** Sí. La aplicación necesita internet para enviar el registro
al sistema central. Si el conductor no tiene internet, el sistema no va a
saber a qué hora empezó a trabajar.

## Pregunta 2 (Sí o No)
Cuando un conductor termina un viaje, la plataforma le muestra un botón
que dice "Finalizar Viaje". Si el conductor toca ese botón antes de
llegar al destino, el sistema registra el viaje como completado igual.

¿Es correcto esto? ¿Se puede finalizar un viaje antes de llegar?

**Respuesta:** No. Si el conductor toca "Finalizar Viaje" antes de llegar,
el sistema va a registrar mal la información. El viaje solo debe finalizarse
cuando el camión llegó al destino indicado.

## Pregunta 3 (Explicame cómo)
Imaginá que sos un transportista y tenés que llevar un pedido desde la
ciudad de Córdoba hasta la ciudad de Rosario. En la plataforma aparece
un botón que dice "Iniciar Viaje".

Explicá paso a paso cómo hacés para que el sistema sepa que arrancaste
el viaje.

**Respuesta:**
1. Abrí la aplicación en tu teléfono.
2. Buscá el viaje asignado en la lista de pendientes.
3. Tocá el botón que dice "Iniciar Viaje".
4. Esperá a que la aplicación confirme que el viaje quedó registrado
   (vas a ver un mensaje verde que dice "Viaje iniciado").
5. Recién ahí podés arrancar el camión.

## Pregunta 4 (Explicame cómo)
En la plataforma hay una sección que se llama "Historial de Viajes".
Ahí se ven todos los viajes que ya hiciste.

Explicá cómo harías para encontrar un viaje específico que hiciste
la semana pasada.

**Respuesta:**
1. Abrí la aplicación y tocá el menú principal (las tres líneas en la
   esquina superior izquierda).
2. Seleccioná "Historial de Viajes".
3. En la barra de búsqueda, escribí la fecha del viaje o el nombre
   del cliente.
4. Tocá el viaje en la lista para ver los detalles.

## Pregunta 5 (Escenario)
En un centro de distribución, los operarios usan escáneres para registrar
los paquetes que cargan en cada camión. Si un operario escanea un paquete
y la pantalla muestra un mensaje en rojo que dice "Paquete no asignado a
este viaje", ¿qué significa ese mensaje?

**Respuesta:** Significa que ese paquete no corresponde a ese camión.
Alguien puede haber cargado el paquete en el lugar equivocado. Lo correcto
es dejar ese paquete aparte y verificar a qué camión pertenece realmente.
```

### Output naming

- `input.md` → `input.pptx` + `input-examen.md`
- `--from-pptx externo.pptx` → `externo-examen.md`
- La ubicación es el mismo directorio del archivo de entrada

---

## Fase 6: PPTX Extractor (modo standalone)

```rust
fn extract_text_from_pptx(path: &str) -> Result<String> {
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;

    let mut slides_text = Vec::new();

    // Leer todas las slides en orden
    for i in 1.. {
        let slide_path = format!("ppt/slides/slide{}.xml", i);
        let Ok(mut slide) = archive.by_name(&slide_path) else { break };
        let mut xml = String::new();
        slide.read_to_string(&mut xml)?;

        // Extraer texto de todos los <a:t> tags
        let document = quick_xml::Reader::from_str(&xml);
        // ... recolectar texto ...
        slides_text.push(text);
    }

    Ok(slides_text.join("\n\n"))
}
```

Se usa el mismo stack `zip` + `quick-xml` que el template engine, cero dependencias extra.

---

## Fase 7: Cross-Platform

| Aspecto | Linux | Windows 11 |
|---------|-------|------------|
| Config path | `~/.shcase/env` | `C:\Users\user\.shcase\env` |
| Resolución home | `dirs::home_dir()` | `dirs::home_dir()` |
| Permisos env | `chmod 600` | No aplica (skip con `#[cfg(unix)]`) |
| Line endings MD | `\n` | `\r\n` → normalizar a `\n` |
| ZIP paths | `/` siempre | `/` siempre |
| Terminal interactivo | `console` | `console` + `crossterm` (transparente) |

---

## Dependencias

```toml
[dependencies]
# Template engine
zip = { version = "2", features = ["deflate"] }
quick-xml = "0.37"

# CLI
clap = { version = "4", features = ["derive"] }
dialoguer = "0.12"

# Parseo
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"

# Networking (Gemini API)
reqwest = { version = "0.12", features = ["json"] }
tokio = { version = "1", features = ["full"] }

# Utilidades
thiserror = "2"
dotenvy = "0.15"
glob = "0.3"
dirs = "6"
```

---

## Flujo Completo

```
┌─────────────────────────────────────────────────────────────────────┐
│ shcase input.md                                                      │
│                                                                      │
│  1. Parsear input.md → PresentationData                              │
│     (cover + slides con contenido técnico original)                  │
│                                                                      │
│  2. [--ai] Resolver API key (env → file → prompt interactivo)       │
│                                                                      │
│  3. [--ai] Gemini → generar versión ultra explicativa               │
│     → enhanced_slides: Vec<SlideData>                               │
│                                                                      │
│  4. [--ai] Gemini → generar 7 preguntas + respuestas                │
│     → exam: Vec<Pregunta>                                           │
│                                                                      │
│  5. Template Engine:                                                 │
│     - Portada: {{main_title}} + {{secondary_title}}                  │
│     - Por cada slide:                                                │
│       a) duplicar slide 2 (con su notes slide)                       │
│       b) body ← enhanced_slides[i] (ultra explicativo)               │
│       c) speaker notes ← slides[i].body (técnico original)           │
│     - Guardar output.pptx                                            │
│                                                                      │
│  6. Generar examen.md (7 preguntas + respuestas)                     │
│                                                                      │
│  Output: input.pptx + input-examen.md                                │
└─────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────┐
│ shcase --from-pptx externo.pptx                                      │
│                                                                      │
│  1. Extractor: abrir PPTX como ZIP → parsear XML de cada slide      │
│  2. [Gemini AI] Generar 7 preguntas + respuestas                    │
│  3. Generar externo-examen.md                                        │
│                                                                      │
│  Output: externo-examen.md                                           │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Decisiones Técnicas

| Decisión | Elección | Motivo |
|----------|----------|--------|
| Template engine | `zip` + `quick-xml` | Única forma de duplicar slides preservando backgrounds, imágenes y relaciones. Crates de alto nivel (ppt-rs, pptx) no soportan `duplicate_slide`. |
| CLI framework | `clap` | Estándar Rust, derive API, manejo de subcomandos |
| Interactive prompts | `dialoguer` | Liviano (2 deps), del equipo console-rs, cubre select/confirm/input |
| Frontmatter | `serde_yaml` | YAML es más legible que TOML para metadata |
| AI provider | Google Gemini API | Elegido por el usuario (no Codex). `reqwest` directo sin SDKs pesados. |
| Key storage | `~/.shcase/env` | Mismo formato que `.env`, fuera del proyecto, reusable |
| Key resolution | env var → file → prompt | Convention over configuration |
| Speaker notes | `set_notes()` en engine | Crea XML de notes slide desde template o desde cero |
| Examen siempre-on | Flujo obligatorio al generar PPTX | Requisito del usuario |
| Extracción PPTX | `zip` + `quick-xml` parsear `<a:t>` | Mismo stack que el template engine, sin dependencias extra |
| Examen en MD | Preguntas + respuestas en markdown | Legible, versionable, fácil de compartir |
| Output naming | Auto: `input.md` → `input.pptx` + `input-examen.md` | Simple, predecible, sin preguntar |
| Cross-platform | `dirs` + `cfg(unix)` + normalizar `\r\n` | Funciona en Linux y Windows 11 |
