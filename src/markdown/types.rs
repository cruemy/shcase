#[derive(Debug, Clone)]
pub struct PresentationData {
    pub cover: CoverData,
    pub slides: Vec<SlideData>,
}

#[derive(Debug, Clone)]
pub struct CoverData {
    pub main_title: String,
    pub secondary_title: String,
}

#[derive(Debug, Clone)]
pub struct SlideData {
    pub title: String,
    pub body: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Frontmatter {
    pub main_title: String,
    pub secondary_title: String,
}
