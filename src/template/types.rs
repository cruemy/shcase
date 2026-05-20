use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

use crate::error::ShcaseError;

type Result<T> = std::result::Result<T, ShcaseError>;

pub struct Template {
    pub entries: HashMap<String, Vec<u8>>,
    pub next_slide_num: u32,
    pub next_r_id: u32,
    pub next_sld_id: u32,
}

impl Template {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = std::fs::File::open(path.as_ref())?;
        let mut archive = ZipArchive::new(file)?;

        let mut entries = HashMap::new();
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            let name = entry.name().to_string();
            let mut data = Vec::new();
            entry.read_to_end(&mut data)?;
            entries.insert(name, data);
        }

        let next_slide_num = Self::find_max_slide_num(&entries) + 1;
        let next_r_id = Self::find_max_r_id(&entries) + 1;
        let next_sld_id = Self::find_max_sld_id(&entries) + 1;

        Ok(Template { entries, next_slide_num, next_r_id, next_sld_id })
    }

    fn find_max_slide_num(entries: &HashMap<String, Vec<u8>>) -> u32 {
        let mut max = 0u32;
        for key in entries.keys() {
            if let Some(n) = Self::parse_num_after_str(key, "ppt/slides/slide") {
                if n > max { max = n; }
            }
        }
        max
    }

    fn find_max_r_id(entries: &HashMap<String, Vec<u8>>) -> u32 {
        let mut max = 0u32;
        let re = regex::Regex::new(r#""rId(\d+)""#).unwrap();
        for data in entries.values() {
            let s = String::from_utf8_lossy(data);
            for cap in re.captures_iter(&s) {
                if let Ok(n) = cap[1].parse::<u32>() {
                    if n > max { max = n; }
                }
            }
        }
        max
    }

    fn find_max_sld_id(entries: &HashMap<String, Vec<u8>>) -> u32 {
        let mut max = 255u32;
        if let Some(data) = entries.get("ppt/presentation.xml") {
            let s = String::from_utf8_lossy(data);
            let re = regex::Regex::new(r#"<p:sldId[^>]*?id="(\d+)""#).unwrap();
            for cap in re.captures_iter(&s) {
                if let Ok(n) = cap[1].parse::<u32>() {
                    if n > max { max = n; }
                }
            }
        }
        max
    }

    fn parse_num_after_str(s: &str, prefix: &str) -> Option<u32> {
        if let Some(idx) = s.find(prefix) {
            let rest = &s[idx + prefix.len()..];
            let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            return num_str.parse().ok();
        }
        None
    }
}
