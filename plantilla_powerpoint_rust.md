# Generar presentaciones PowerPoint desde una plantilla en Rust

## Contexto

Tengo un archivo `.pptx` que ya contiene una estructura base:

- La primera slide es la portada.
  - Tiene un título principal.
  - Tiene un título secundario.
  - Usa una imagen de background.

- La segunda slide funciona como plantilla de contenido.
  - Tiene un título de slide, opcional.
  - Tiene un campo de contenido.
  - También usa una imagen de background.

Actualmente, el proceso manual consiste en duplicar la segunda slide dentro de PowerPoint y cambiar el contenido de cada copia.

La idea es automatizar este proceso desde Rust.

---

## Objetivo

Usar el `.pptx` existente como una plantilla y generar una nueva presentación automáticamente, conservando:

- La estructura visual de las slides.
- Las imágenes de background.
- Los estilos.
- Las posiciones de los textos.
- Logos, formas, colores y cualquier elemento ya definido en la plantilla.

---

## Enfoque recomendado

La mejor estrategia es tratar el archivo `.pptx` como una **presentación template**.

En lugar de crear slides desde cero, se debe:

1. Abrir el `.pptx` base.
2. Modificar la slide 1 para reemplazar los textos de portada.
3. Usar la slide 2 como molde de contenido.
4. Duplicar la slide 2 tantas veces como sea necesario.
5. Reemplazar los textos de cada copia.
6. Guardar el resultado como un nuevo `.pptx`.

Este enfoque es importante porque permite conservar automáticamente las imágenes de background y el diseño original.

---

## Placeholders sugeridos

En PowerPoint, conviene reemplazar los textos de ejemplo por placeholders fáciles de detectar desde código.

Por ejemplo, en la slide 1:

```text
{{main_title}}
{{secondary_title}}
```

Y en la slide 2:

```text
{{slide_title}}
{{content}}
```

Luego, desde Rust, el programa puede buscar esos textos dentro del XML de cada slide y reemplazarlos por contenido real.

---

## Modelo de uso esperado

Conceptualmente, el flujo sería algo así:

```rust
let template = open_pptx("base.pptx")?;

// Slide 1 = portada
// Slide 2 = template de contenido
let cover_slide = template.slide(0);
let content_template = template.slide(1);

cover_slide.replace_text("{{main_title}}", "Mi título principal");
cover_slide.replace_text("{{secondary_title}}", "Subtítulo de la presentación");

for item in slides {
    let new_slide = template.duplicate_slide(content_template)?;
    new_slide.replace_text("{{slide_title}}", item.title);
    new_slide.replace_text("{{content}}", item.content);
}

template.save("output.pptx")?;
```

Este código es conceptual. La implementación exacta depende de la librería usada o de si se manipula directamente el `.pptx` como XML.

---

## Librerías posibles en Rust

### `pptx`

Existe un crate llamado `pptx` orientado a leer y escribir archivos PowerPoint `.pptx`.

Puede ser útil si permite:

- Abrir una presentación existente.
- Leer slides.
- Modificar textos.
- Guardar el archivo final.
- Preservar relaciones internas e imágenes.

Es una opción interesante para trabajar con presentaciones existentes.

---

### `ppt-rs`

También existe `ppt-rs`, que permite crear y trabajar con presentaciones `.pptx`.

Puede ser útil para generar presentaciones, pero habría que validar si soporta bien este caso específico:

- Duplicar una slide existente.
- Mantener imágenes de background.
- Mantener relaciones internas del archivo.
- Editar textos de slides copiadas.

Muchas librerías permiten crear slides nuevas, pero no necesariamente duplicar slides reales con todos sus recursos asociados.

---

## Alternativa robusta: manipular el `.pptx` como ZIP + XML

Un archivo `.pptx` internamente es un archivo ZIP con archivos XML y recursos multimedia.

La estructura interna suele tener archivos como estos:

```text
ppt/slides/slide1.xml
ppt/slides/slide2.xml
ppt/slides/_rels/slide1.xml.rels
ppt/slides/_rels/slide2.xml.rels
ppt/media/image1.png
ppt/media/image2.png
ppt/presentation.xml
ppt/_rels/presentation.xml.rels
[Content_Types].xml
```

Esto es importante porque una imagen de background normalmente no está embebida directamente en `slide2.xml`.

La slide suele tener una referencia como `rId1`, `rId2`, etc. Esa referencia apunta a una imagen mediante el archivo `.rels` correspondiente.

Por ejemplo:

```text
ppt/slides/slide2.xml
```

puede referenciar una imagen usando un identificador como:

```text
rId3
```

Y ese `rId3` se resuelve dentro de:

```text
ppt/slides/_rels/slide2.xml.rels
```

Por eso, para duplicar correctamente una slide con background, no basta con copiar solo el XML de la slide.

También hay que copiar su archivo de relaciones.

---

## Qué se debe copiar al duplicar la slide 2

Si la slide 2 es la plantilla de contenido, para crear una nueva slide se debe copiar:

```text
ppt/slides/slide2.xml
ppt/slides/_rels/slide2.xml.rels
```

A nuevos archivos, por ejemplo:

```text
ppt/slides/slide3.xml
ppt/slides/_rels/slide3.xml.rels
```

Luego, también hay que actualizar los archivos globales de la presentación:

```text
ppt/presentation.xml
ppt/_rels/presentation.xml.rels
[Content_Types].xml
```

Estos archivos indican que existe una nueva slide dentro de la presentación.

---

## Crates útiles para el enfoque ZIP + XML

Una implementación manual en Rust podría usar crates generales como estos:

```toml
[dependencies]
zip = "2"
quick-xml = "0.37"
uuid = { version = "1", features = ["v4"] }
```

También podrían ser útiles:

```toml
[dependencies]
roxmltree = "0.20"
xmltree = "0.11"
```

Dependiendo de si se quiere parsear XML de forma estructurada o hacer reemplazos simples de texto.

---

## Ventajas de duplicar la slide completa

Duplicar la slide completa es mejor que crear una slide nueva desde cero porque conserva:

- Backgrounds.
- Imágenes.
- Logos.
- Shapes.
- Tipografías.
- Posiciones.
- Tamaños.
- Layouts.
- Estilos definidos en PowerPoint.
- Relaciones internas del archivo.

Esto es especialmente importante cuando las slides tienen diseño visual complejo.

---

## Recomendación práctica

La recomendación más segura es:

1. Crear un `.pptx` base en PowerPoint.
2. Dejar la slide 1 como portada.
3. Dejar la slide 2 como plantilla de contenido.
4. Usar placeholders claros:

```text
{{main_title}}
{{secondary_title}}
{{slide_title}}
{{content}}
```

5. Desde Rust, abrir el `.pptx` como ZIP.
6. Duplicar `slide2.xml` y `slide2.xml.rels` para cada nueva slide.
7. Reemplazar los placeholders en cada copia.
8. Actualizar `presentation.xml`, `presentation.xml.rels` y `[Content_Types].xml`.
9. Guardar el resultado como un nuevo `.pptx`.

---

## Punto clave sobre los backgrounds

Para conservar las imágenes de background, no se debe reconstruir la slide desde cero.

Lo correcto es duplicar la slide completa junto con su archivo `.rels`.

De esta forma, las referencias a imágenes se mantienen y el diseño visual debería verse igual que en la plantilla original.

---

## Resumen

Sí, se puede usar exactamente la slide 2 como molde para generar nuevas slides.

La solución más confiable consiste en trabajar con el `.pptx` como una plantilla, duplicar la slide de contenido y reemplazar los placeholders.

En Rust, se puede intentar primero con una librería como `pptx` o `ppt-rs`, pero si no soportan bien la duplicación de slides con imágenes y relaciones internas, la alternativa más robusta es manipular directamente el `.pptx` como ZIP + XML.

La parte más importante es no olvidar que las imágenes de background dependen de los archivos `.rels`, no solo del XML principal de la slide.
