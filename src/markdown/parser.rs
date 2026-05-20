use regex::Regex;

use crate::error::ShcaseError;
use super::types::{CoverData, Frontmatter, PresentationData, SlideData};

type Result<T> = std::result::Result<T, ShcaseError>;

pub fn parse_markdown(input: &str) -> Result<PresentationData> {
    let normalized = input.replace("\r\n", "\n");
    let normalized = normalized.trim();

    let (frontmatter_str, body) = split_frontmatter(normalized)?;
    let frontmatter: Frontmatter = serde_yaml::from_str(frontmatter_str)?;

    let cover = CoverData {
        main_title: frontmatter.main_title,
        secondary_title: frontmatter.secondary_title,
    };

    let slides = parse_slides(body)?;

    Ok(PresentationData { cover, slides })
}

fn split_frontmatter(input: &str) -> Result<(&str, &str)> {
    let input = input.trim();
    if !input.starts_with("---") {
        return Err(ShcaseError::Markdown(
            "Formato inválido: se requiere frontmatter entre --- y ---".into(),
        ));
    }

    let rest = &input[3..].trim_start();
    if let Some(end) = rest.find("\n---") {
        let frontmatter = rest[..end].trim();
        let body = rest[end + 4..].trim();
        Ok((frontmatter, body))
    } else if let Some(end) = rest.find("---") {
        let frontmatter = rest[..end].trim();
        let body = &rest[end + 3..].trim();
        Ok((frontmatter, body))
    } else {
        Err(ShcaseError::Markdown(
            "Formato inválido: no se encontró el cierre del frontmatter".into(),
        ))
    }
}

fn parse_slides(body: &str) -> Result<Vec<SlideData>> {
    let re = Regex::new(r"(?m)^## (.+?)$")?;
    let mut slides = Vec::new();
    let mut titles: Vec<(usize, String)> = Vec::new();

    for cap in re.captures_iter(body) {
        let match_start = cap.get(0).unwrap().start();
        titles.push((match_start, cap[1].trim().to_string()));
    }

    for i in 0..titles.len() {
        let title = &titles[i].1;
        let start = titles[i].0;
        let end = if i + 1 < titles.len() { titles[i + 1].0 } else { body.len() };

        let section_start = start + body[start..].find('\n').unwrap_or(0) + 1;
        let body_text = body[section_start..end].trim().to_string();

        slides.push(SlideData {
            title: title.clone(),
            body: body_text,
        });
    }

    Ok(slides)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic() {
        let md = r#"---
main_title: "Test"
secondary_title: "Sub"
---

## Slide One
Content here

## Slide Two
More content
"#;
        let data = parse_markdown(md).unwrap();
        assert_eq!(data.cover.main_title, "Test");
        assert_eq!(data.slides.len(), 2);
        assert_eq!(data.slides[0].title, "Slide One");
        assert_eq!(data.slides[0].body, "Content here");
    }

    #[test]
    fn test_parse_no_body() {
        let md = r#"---
main_title: "Test"
secondary_title: "Sub"
---
"#;
        let data = parse_markdown(md).unwrap();
        assert_eq!(data.slides.len(), 0);
    }
}
