# Obsidian — Markdown → PowerPoint Native Renderer

## Objetivo

Reemplazar el `replace_text()` plano actual por un sistema completo que parsea
markdown y genera elementos XML nativos de PowerPoint: `<a:p>` por párrafo,
`<a:r>` con `<a:rPr>` para formato inline, `<a:buChar>` / `<a:buAutoNum>`
para listas reales, `<a:lvl>` para indentación, y `<a:br>` para saltos de línea.

---

## Arquitectura

```
src/
├── md2pptx/                     # Motor independiente
│   ├── mod.rs
│   ├── lexer.rs                 # Char stream → tokens
│   ├── ast.rs                   # Tipos: Block, Inline, ListItem, etc.
│   ├── parser.rs                # Token stream → AST (sintáctico)
│   ├── semantic.rs              # Validación + sanitización + lints
│   ├── xmlgen.rs                # AST validado → fragmentos XML de PowerPoint
│   └── render.rs                # Integración: reemplaza {{content}}
│
└── template/
    └── engine.rs                # MODIFICADO: usa md2pptx::render_content()
```

---

## Fase 0: Lexer (`lexer.rs`)

Convierte el string de markdown en un stream de `Token` con posición
(línea, columna). Es char-by-char, sin regex. Caracteres no reconocidos
producen `Token::Error`.

```rust
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
    pub span: SourceSpan,
}

pub enum TokenKind {
    // Bloques
    Hash(u8),                    // #, ##, ###, etc.
    Dash, Star,
    NumberDot,                   // 1.
    Backtick(u8),                // `, ``, ```
    Angle,                       // >
    Newline, BlankLine,
    // Inline
    DoubleStar, Underscore, DoubleUnderscore,
    Tilde, DoubleTilde,
    OpenBracket, CloseBracket,
    OpenParen, CloseParen,
    Backslash,
    // Contenido
    Text(String), Whitespace,
    // Meta
    Eof, Error(String),
}
```

---

## Fase 1: AST (`ast.rs`)

```rust
pub enum Block {
    Paragraph(Vec<Inline>),
    Heading { level: u8, children: Vec<Inline> },
    UnorderedList { items: Vec<ListItem> },
    OrderedList { items: Vec<ListItem> },
    CodeBlock { language: Option<String>, text: String },
    Blockquote(Vec<Block>),
}

pub struct ListItem {
    pub children: Vec<Block>,
    pub level: u8,
}

pub enum Inline {
    Text(String),
    Bold(Vec<Inline>),
    Italic(Vec<Inline>),
    Code(String),
    Strikethrough(Vec<Inline>),
    Link { text: Vec<Inline>, url: String },
    HardBreak,
}
```

---

## Fase 2: Parser Sintáctico (`parser.rs`)

Toma `Vec<Token>` y produce el AST. Gramática:

```
markdown   → block+
block      → heading | ulist | olist | codeblock | blockquote | paragraph
heading    → HASH(n) whitespace? inline NEWLINE
ulist      → (DASH whitespace inline NEWLINE)+ BLANKLINE?
olist      → (NUMBER DOT whitespace inline NEWLINE)+ BLANKLINE?
codeblock  → BACKTICK(3) language? NEWLINE text BACKTICK(3)
blockquote → ANGLE? inline NEWLINE
paragraph  → inline (NEWLINE inline)* BLANKLINE
inline     → (bold | italic | code | strike | link | text)+
bold       → DOUBLE_STAR inline DOUBLE_STAR
italic     → STAR inline STAR
code       → BACKTICK(1) text BACKTICK(1)
strike     → DOUBLE_TILDE inline DOUBLE_TILDE
```

Errores detectados: `**bold sin cerrar`, `[texto](url` sin `)`,
listas con indentación inconsistente.

---

## Fase 3: Analizador Semántico (`semantic.rs`)

Recibe el AST, lo valida y produce **AST validado** o **lista de errores**.

```rust
pub enum SemanticIssue {
    // Errores (bloquean generación)
    UnclosedMarker { kind: &'static str, span: SourceSpan },
    CodeBlockNotClosed { start: SourceSpan },
    UnescapedXmlChar { char: char, span: SourceSpan },
    // Warnings (no bloquean, se muestran en consola)
    EmptyBold(SourceSpan),
    EmptyItalic(SourceSpan),
    DeepNesting { level: u8, span: SourceSpan },
    EmojiDetected { span: SourceSpan },
}

pub fn validate(ast: &[Block]) -> Result<Vec<Block>, Vec<SemanticIssue>>;
```

### Validaciones

| Validación | Tipo | Acción |
|------------|------|--------|
| Marcadores inline sin cerrar | Error | Reporta span exacto, bloquea |
| Código sin cerrar | Error | Reporta, bloquea |
| `<`, `>`, `&` en texto | Error | Marca para `xml_escape()` obligatorio |
| `****` / `**  **` vacío | Warning | Omite el marker |
| Listas con >6 niveles | Warning | Trunca a 6 (límite PowerPoint) |
| Emojis detectados | Warning | "Slide N contiene emojis — verificá que la fuente del template los soporte" |
| Párrafo > 5000 chars | Warning | Sugiere dividir |

### Emoji Lint

```rust
// Sin dependencias nuevas (regex ya está en Cargo.toml)
// Renderizado en < 0.1ms
fn lint_emoji(blocks: &[Block]) -> Vec<SemanticIssue> {
    let re = Regex::new(
        r"[\u{2600}-\u{27BF}\u{1F300}-\u{1FAFF}\u{FE00}-\u{FE0F}\u{1F3FB}-\u{1F3FF}\u{200D}]"
    ).unwrap();
    // recorre AST, busca Inline::Text que matchee → EmojiDetected con span
}
```

No bloquea la generación — solo informa en consola.

---

## Fase 4: XML Generator (`xmlgen.rs`)

Toma el AST validado y produce strings XML. **Todo texto pasa por
`xml_escape()`**.

```rust
pub fn generate_body(blocks: &[Block]) -> Result<String> {
    let mut out = String::with_capacity(estimate_capacity(blocks));
    for block in blocks {
        out.push_str(&block_to_paragraph(block, 0)?);
    }
    Ok(out)
}
```

| Markdown | XML generado |
|----------|-------------|
| Párrafo | `<a:p><a:r><a:t>texto</a:t></a:r></a:p>` |
| **bold** | `<a:r><a:rPr b="1"/><a:t>bold</a:t></a:r>` |
| *italic* | `<a:r><a:rPr i="1"/><a:t>italic</a:t></a:r>` |
| `code` | `<a:r><a:rPr sz="1600"/><a:t>code</a:t></a:r>` |
| - item | `<a:p><a:pPr><a:buChar char="•"/></a:pPr><a:r><a:t>item</a:t></a:r></a:p>` |
| 1. item | `<a:p><a:pPr><a:buAutoNum/></a:pPr><a:r><a:t>item</a:t></a:r></a:p>` |
| ## title | `<a:p><a:pPr><a:rPr sz="2800" b="1"/></a:pPr><a:r><a:t>title</a:t></a:r></a:p>` |

---

## Fase 5: Render (`render.rs`)

Conexión con el engine actual:

```rust
pub fn render_content(slide_xml: &str, markdown: &str) -> Result<String> {
    let tokens = lexer::tokenize(markdown)?;
    let ast = parser::parse(&tokens)?;
    let ast = semantic::validate(&ast)?;
    let body_xml = xmlgen::generate_body(&ast)?;
    replace_txbody_content(slide_xml, "{{content}}", &body_xml)
}
```

En `engine.rs`:

```rust
impl Template {
    pub fn render_slide_content(&mut self, slide_num: u32, markdown: &str) -> Result<()> {
        let key = format!("ppt/slides/slide{}.xml", slide_num);
        let data = self.entries.get(&key)
            .ok_or_else(|| ShcaseError::SlideNotFound(slide_num))?;
        let xml = String::from_utf8_lossy(data);
        let new_xml = md2pptx::render::render_content(&xml, markdown)?;
        self.entries.insert(key, new_xml.into_bytes());
        Ok(())
    }
}
```

---

## Performance

| Riesgo | Probabilidad | Mitigación |
|--------|-------------|------------|
| Markdown > 1MB | Baja | `estimate_capacity()` pre-asigna buffer; pipeline streaming por bloques; `MAX_INPUT_SIZE` configurable (512KB default) |
| AST enorme | Baja | Strings con ownership (no clones); Vec acotado |
| Regex catastrófica | Media | Lexer char-by-char sin regex; parser con límite de nesting (10 niveles) |

Para el caso típico (10–50 slides, < 100KB), corre en < 1ms.

---

## Plan de Implementación por Fases

### Fase 1 — Foundation
- `ast.rs`: `Block::Paragraph` + `Inline::Text`
- `lexer.rs`: solo texto plano, newlines, blanklines
- `parser.rs`: solo paragraphs
- `semantic.rs`: solo verificación de XML chars no escapados
- `xmlgen.rs`: `<a:p><a:r><a:t>`
- **Output**: párrafos separados con XML nativo

### Fase 2 — Inline Formatting
- `lexer.rs`: `**`, `*`, `` ` ``, `~~`
- `parser.rs`: nesting bold, italic, code, strikethrough
- `semantic.rs`: marcadores sin cerrar, énfasis vacío, + emoji lint
- `xmlgen.rs`: `<a:rPr b="1" i="1" sz="1600" strike="sngStrike">`

### Fase 3 — Lists
- `lexer.rs`: `-`, `*`, `1.`
- `parser.rs`: listas anidadas por indentación
- `semantic.rs`: consistencia indentación, máximo 6 niveles
- `xmlgen.rs`: `<a:buChar>`, `<a:buAutoNum>`, `<a:lvl>`

### Fase 4 — Headings + Code Blocks
- `lexer.rs`: `#`+, `` ``` ``
- `parser.rs`: headings con nivel, code blocks multilinea
- `semantic.rs`: code blocks sin cerrar, headings sin contenido
- `xmlgen.rs`: `<a:rPr sz="2800" b="1">`, monospace para código

### Fase 5 — Blockquotes + Extras
- `lexer.rs`: `>`
- `parser.rs`: blockquotes anidados
- Links como texto plano
- `<a:br/>` para hard breaks

---

## Riesgos Finales

| # | Riesgo | Impacto | Mitigación |
|---|--------|---------|------------|
| 1 | **XML mal formado por caracteres especiales** | Alto — archivo corrupto | `semantic.rs` marca caracteres XML-sensitive; `xmlgen.rs` SIEMPRE pasa por `xml_escape()`. No hay ruta donde texto crudo llegue al XML. |
| 2 | **Inline markers sin cerrar** | Medio — parseo parcial | `semantic.rs` detecta y bloquea con span exacto. Error se propaga a `render.rs` que retorna `Err`. |
| 3 | **Performance en markdown grande** | Bajo | Pre-asignación de buffers, parser por bloques, límite de nesting, `MAX_INPUT_SIZE` configurable. |
| 4 | **Nesting excesivo** | Bajo — PowerPoint limita a 9 niveles | `semantic.rs` trunca a 6 con warning. |
| 5 | **Regresión en templates existentes** | Medio | `render_slide_content()` es opt-in. `replace_text()` sigue existiendo para portada y título. |
| 6 | **Emojis no renderizados por la fuente del template** | Bajo — se ven como cuadrados | Lint en `semantic.rs` advierte al usuario sin bloquear la generación. |
