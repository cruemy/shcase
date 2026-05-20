use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

use crate::error::ShcaseError;

type Result<T> = std::result::Result<T, ShcaseError>;

pub fn extract_text_from_pptx<P: AsRef<Path>>(path: P) -> Result<String> {
    let file = std::fs::File::open(path.as_ref())?;
    let mut archive = ZipArchive::new(file)?;

    let mut all_text = Vec::new();

    for i in 1..=1000 {
        let slide_path = format!("ppt/slides/slide{}.xml", i);
        let mut slide_file = match archive.by_name(&slide_path) {
            Ok(f) => f,
            Err(_) => break,
        };

        let mut xml = String::new();
        slide_file.read_to_string(&mut xml)?;

        let text = extract_text_from_slide_xml(&xml);
        all_text.push(text);
    }

    if all_text.is_empty() {
        return Err(ShcaseError::Template("No se encontraron slides en el PPTX".into()));
    }

    Ok(all_text.join("\n\n"))
}

fn extract_text_from_slide_xml(xml: &str) -> String {
    let mut texts = Vec::new();
    let mut in_at = false;
    let mut current_text = String::new();

    let bytes = xml.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i..].starts_with(b"<a:t") && !bytes[i..].starts_with(b"<a:t>)") {
            in_at = true;
            current_text.clear();
            // Find closing >
            while i < bytes.len() && bytes[i] != b'>' {
                i += 1;
            }
            i += 1;
        } else if in_at && bytes[i..].starts_with(b"</a:t>") {
            in_at = false;
            texts.push(std::mem::take(&mut current_text));
            i += 6;
        } else if in_at {
            current_text.push(bytes[i] as char);
            i += 1;
        } else {
            i += 1;
        }
    }

    texts.join(" ")
}
