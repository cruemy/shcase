use std::io::Write;
use std::path::Path;
use zip::{CompressionMethod, ZipWriter};
use zip::write::SimpleFileOptions;

use crate::error::ShcaseError;
use super::types::Template;

type Result<T> = std::result::Result<T, ShcaseError>;

impl Template {
    pub fn duplicate_slide(&mut self, source_idx: u32) -> Result<u32> {
        let new_num = self.next_slide_num;
        self.next_slide_num += 1;

        let new_r_id = self.next_r_id;
        self.next_r_id += 1;

        let new_sld_id = self.next_sld_id;
        self.next_sld_id += 1;

        let source_name = format!("ppt/slides/slide{}.xml", source_idx);
        let source_data = self.entries.get(&source_name)
            .ok_or_else(|| ShcaseError::SlideNotFound(source_idx))?;

        let new_name = format!("ppt/slides/slide{}.xml", new_num);
        self.entries.insert(new_name.clone(), source_data.clone());

        let source_rels = format!("ppt/slides/_rels/slide{}.xml.rels", source_idx);
        if let Some(rels_data) = self.entries.get(&source_rels) {
            let new_rels = format!("ppt/slides/_rels/slide{}.xml.rels", new_num);
            self.entries.insert(new_rels, rels_data.clone());
        }

        let source_notes = format!("ppt/notesSlides/notesSlide{}.xml", source_idx);
        if let Some(notes_data) = self.entries.get(&source_notes) {
            let new_notes = format!("ppt/notesSlides/notesSlide{}.xml", new_num);
            self.entries.insert(new_notes.clone(), notes_data.clone());

            let source_notes_rels = format!("ppt/notesSlides/_rels/notesSlide{}.xml.rels", source_idx);
            if let Some(notes_rels_data) = self.entries.get(&source_notes_rels) {
                let new_notes_rels = format!("ppt/notesSlides/_rels/notesSlide{}.xml.rels", new_num);
                self.entries.insert(new_notes_rels, notes_rels_data.clone());
            }

            self.add_content_type_override(&new_notes,
                "application/vnd.openxmlformats-officedocument.presentationml.notesSlide+xml")?;
        }

        self.update_copied_rels(source_idx, new_num)?;

        let rel_target = new_name.strip_prefix("ppt/").unwrap_or(&new_name);
        self.add_slide_to_presentation_xml(new_r_id, new_sld_id)?;
        self.add_slide_relationship(new_r_id, rel_target)?;
        self.add_content_type_override(&new_name,
            "application/vnd.openxmlformats-officedocument.presentationml.slide+xml")?;

        Ok(new_num)
    }

    fn update_copied_rels(&mut self, source_idx: u32, new_num: u32) -> Result<()> {
        let rels_key = format!("ppt/slides/_rels/slide{}.xml.rels", new_num);
        if let Some(data) = self.entries.get(&rels_key) {
            let xml = String::from_utf8_lossy(data).to_string();
            let old_ref = format!("notesSlide{}.xml", source_idx);
            let new_ref = format!("notesSlide{}.xml", new_num);
            let updated = xml.replace(&old_ref, &new_ref);
            self.entries.insert(rels_key, updated.into_bytes());
        }

        let notes_rels_key = format!("ppt/notesSlides/_rels/notesSlide{}.xml.rels", new_num);
        if let Some(data) = self.entries.get(&notes_rels_key) {
            let xml = String::from_utf8_lossy(data).to_string();
            let old_ref = format!("slides/slide{}.xml", source_idx);
            let new_ref = format!("slides/slide{}.xml", new_num);
            let updated = xml.replace(&old_ref, &new_ref);
            self.entries.insert(notes_rels_key, updated.into_bytes());
        }

        Ok(())
    }

    fn add_slide_to_presentation_xml(&mut self, r_id: u32, sld_id: u32) -> Result<()> {
        let key = "ppt/presentation.xml";
        let data = self.entries.get(key)
            .ok_or_else(|| ShcaseError::Template("presentation.xml not found".into()))?;
        let mut xml = String::from_utf8_lossy(data).to_string();

        let new_entry = format!("<p:sldId id=\"{}\" r:id=\"rId{}\"/>", sld_id, r_id);

        if let Some(pos) = xml.rfind("</p:sldIdLst>") {
            xml.insert_str(pos, &format!("{}\n        ", new_entry));
        } else if let Some(pos) = xml.rfind("</p:sldIdLst>") {
            xml.insert_str(pos, &format!("{}\n        ", new_entry));
        } else {
            return Err(ShcaseError::Template("Cannot find </p:sldIdLst> in presentation.xml".into()));
        }

        self.entries.insert(key.into(), xml.into_bytes());
        Ok(())
    }

    fn add_slide_relationship(&mut self, r_id: u32, target: &str) -> Result<()> {
        let key = "ppt/_rels/presentation.xml.rels";
        let data = self.entries.get(key)
            .ok_or_else(|| ShcaseError::Template("presentation.xml.rels not found".into()))?;
        let mut xml = String::from_utf8_lossy(data).to_string();

        let rel_type = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide";
        let new_rel = format!(
            "<Relationship Id=\"rId{}\" Type=\"{}\" Target=\"{}\"/>",
            r_id, rel_type, target
        );

        if let Some(pos) = xml.rfind("</Relationships>") {
            xml.insert_str(pos, &format!("{}\n    ", new_rel));
        } else {
            return Err(ShcaseError::Template("Cannot find </Relationships> in presentation.xml.rels".into()));
        }

        self.entries.insert(key.into(), xml.into_bytes());
        Ok(())
    }

    fn add_content_type_override(&mut self, part_name: &str, content_type: &str) -> Result<()> {
        let key = "[Content_Types].xml";
        let data = self.entries.get(key)
            .ok_or_else(|| ShcaseError::Template("[Content_Types].xml not found".into()))?;
        let mut xml = String::from_utf8_lossy(data).to_string();

        let part_path = format!("/{}", part_name);
        let new_override = format!(
            "<Override PartName=\"{}\" ContentType=\"{}\"/>",
            part_path, content_type
        );

        if let Some(pos) = xml.rfind("</Types>") {
            xml.insert_str(pos, &format!("{}\n  ", new_override));
        } else {
            return Err(ShcaseError::Template("Cannot find </Types> in [Content_Types].xml".into()));
        }

        self.entries.insert(key.into(), xml.into_bytes());
        Ok(())
    }

    pub fn replace_text(&mut self, slide_num: u32, placeholder: &str, text: &str) -> Result<()> {
        let key = format!("ppt/slides/slide{}.xml", slide_num);
        let data = self.entries.get(&key)
            .ok_or_else(|| ShcaseError::SlideNotFound(slide_num))?;

        if !data.windows(placeholder.len()).any(|w| w == placeholder.as_bytes()) {
            eprintln!("Warning: placeholder '{}' no encontrado en slide {}. El template necesita tener este texto exacto.", placeholder, slide_num);
        }

        let escaped = xml_escape(text);
        let old_bytes = placeholder.as_bytes();
        let new_data = replace_all_bytes(data, old_bytes, escaped.as_bytes());

        self.entries.insert(key, new_data);
        Ok(())
    }

    pub fn set_notes(&mut self, slide_num: u32, text: &str) -> Result<()> {
        let notes_key = format!("ppt/notesSlides/notesSlide{}.xml", slide_num);
        let rels_key = format!("ppt/slides/_rels/slide{}.xml.rels", slide_num);

        let escaped = xml_escape(text);

        if let Some(data) = self.entries.get(&notes_key) {
            let xml = String::from_utf8_lossy(data).to_string();
            let new_xml = xml.replace("{{NOTES_TEXT}}", &escaped);
            self.entries.insert(notes_key, new_xml.into_bytes());
            return Ok(());
        }

        let notes_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:notes xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
         xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
         xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr>
        <p:cNvPr id="1" name=""/>
        <p:cNvGrpSpPr/>
        <p:nvPr/>
      </p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="2" name="Notes Content"/>
          <p:cNvSpPr txBox="1"/>
          <p:nvPr>
            <p:ph type="body" idx="1"/>
          </p:nvPr>
        </p:nvSpPr>
        <p:spPr/>
        <p:txBody>
          <a:bodyPr/>
          <a:lstStyle/>
          <a:p>
            <a:r>
              <a:rPr lang="en-US"/>
              <a:t>__NOTES_TEXT__</a:t>
            </a:r>
          </a:p>
        </p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:notes>
"#.replace("__NOTES_TEXT__", &escaped);

        self.entries.insert(notes_key.clone(), notes_xml.into_bytes());

        if let Some(data) = self.entries.get(&rels_key) {
            let mut xml = String::from_utf8_lossy(data).to_string();

            if !xml.contains("notesSlide") {
                let notes_r_id = self.next_r_id;
                self.next_r_id += 1;

                let notes_rel = format!(
                    r#"<Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/notesSlide" Target="../notesSlides/notesSlide{}.xml"/>"#,
                    notes_r_id, slide_num
                );

                if let Some(pos) = xml.rfind("</Relationships>") {
                    xml.insert_str(pos, &format!("\n    {}", notes_rel));
                }

                self.entries.insert(rels_key.clone(), xml.into_bytes());
            }
        }

        self.add_content_type_override(&notes_key,
            "application/vnd.openxmlformats-officedocument.presentationml.notesSlide+xml")?;

        Ok(())
    }

    pub fn remove_slide(&mut self, slide_num: u32) -> Result<()> {
        self.entries.remove(&format!("ppt/slides/slide{}.xml", slide_num));
        self.entries.remove(&format!("ppt/slides/_rels/slide{}.xml.rels", slide_num));
        self.entries.remove(&format!("ppt/notesSlides/notesSlide{}.xml", slide_num));
        self.entries.remove(&format!("ppt/notesSlides/_rels/notesSlide{}.xml.rels", slide_num));

        let slide_r_id = self.find_slide_r_id(slide_num);

        let rels_key = "ppt/_rels/presentation.xml.rels";
        if let Some(data) = self.entries.get(rels_key).cloned() {
            let xml = String::from_utf8_lossy(&data).to_string();
            let target = format!("slides/slide{}.xml", slide_num);
            let mut updated = String::new();
            let mut in_removed = false;
            for line in xml.split_inclusive('>') {
                if line.contains(&target) && line.contains("relationships/slide") {
                    in_removed = true;
                    continue;
                }
                updated.push_str(line);
            }
            if in_removed {
                self.entries.insert(rels_key.into(), updated.into_bytes());
            }
        }

        if slide_r_id > 0 {
            let pres_key = "ppt/presentation.xml";
            if let Some(data) = self.entries.get(pres_key).cloned() {
                let xml = String::from_utf8_lossy(&data).to_string();
                let sld_id_pattern = format!("rId{}\"", slide_r_id);
                let mut updated = String::new();
                let mut in_removed = false;
                for line in xml.split_inclusive('>') {
                    if line.contains("<p:sldId") && line.contains(&sld_id_pattern) {
                        in_removed = true;
                        continue;
                    }
                    updated.push_str(line);
                }
                if in_removed {
                    self.entries.insert(pres_key.into(), updated.into_bytes());
                }
            }
        }

        let ct_key = "[Content_Types].xml";
        if let Some(data) = self.entries.get(ct_key).cloned() {
            let xml = String::from_utf8_lossy(&data).to_string();
            let slide_path = format!("/ppt/slides/slide{}.xml", slide_num);
            let notes_path = format!("/ppt/notesSlides/notesSlide{}.xml", slide_num);
            let mut updated = String::new();
            for line in xml.split_inclusive('>') {
                if line.contains("Override") && (line.contains(&slide_path) || line.contains(&notes_path)) {
                    continue;
                }
                updated.push_str(line);
            }
            self.entries.insert(ct_key.into(), updated.into_bytes());
        }

        Ok(())
    }

    fn find_slide_r_id(&self, slide_num: u32) -> u32 {
        let key = "ppt/_rels/presentation.xml.rels";
        if let Some(data) = self.entries.get(key) {
            let xml = String::from_utf8_lossy(data);
            let target = format!("Target=\"slides/slide{}.xml\"", slide_num);
            if let Some(pos) = xml.find(&target) {
                let before = &xml[..pos];
                if let Some(id_start) = before.rfind("Id=\"rId") {
                    let id_str = &before[id_start + 7..];
                    let num_str: String = id_str.chars().take_while(|c| c.is_ascii_digit()).collect();
                    if let Ok(n) = num_str.parse::<u32>() {
                        return n;
                    }
                }
            }
        }
        0
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let file = std::fs::File::create(path.as_ref())?;
        let mut writer = ZipWriter::new(file);

        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated);

        let mut entry_names: Vec<&String> = self.entries.keys().collect();
        entry_names.sort();

        let slide_count = self.count_slides();

        for name in entry_names {
            let data = if *name == "docProps/app.xml" {
                let xml = String::from_utf8_lossy(self.entries.get(name).unwrap()).to_string();
                update_slide_count_in_app_xml(&xml, slide_count).into_bytes()
            } else {
                self.entries.get(name).unwrap().clone()
            };
            writer.start_file(name, options)?;
            writer.write_all(&data)?;
        }

        writer.finish()?;
        Ok(())
    }

    fn count_slides(&self) -> u32 {
        self.entries.keys()
            .filter(|k| k.starts_with("ppt/slides/slide") && k.ends_with(".xml"))
            .count() as u32
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn replace_all_bytes(data: &[u8], from: &[u8], to: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(data.len());
    let mut i = 0;
    while i < data.len() {
        if data[i..].starts_with(from) {
            result.extend_from_slice(to);
            i += from.len();
        } else {
            result.push(data[i]);
            i += 1;
        }
    }
    result
}

fn update_slide_count_in_app_xml(xml: &str, count: u32) -> String {
    let re_slides = regex::Regex::new(r"<Slides>\d+</Slides>").unwrap();
    let re_notes = regex::Regex::new(r"<Notes>\d+</Notes>").unwrap();
    let xml = re_slides.replace(xml, format!("<Slides>{}</Slides>", count));
    re_notes.replace(&xml, format!("<Notes>{}</Notes>", count)).to_string()
}
