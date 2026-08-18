use url::Url;

use crate::about::PrefToggle;

/// A loaded document. HTML arrives in Phase 3.
#[derive(Clone, Debug)]
pub enum Document {
    Blank,
    Internal(InternalPage),
    Unavailable { url: Url, reason: String },
}

impl Document {
    pub fn internal(page: InternalPage) -> Self {
        Self::Internal(page)
    }

    pub fn unavailable(url: Url, reason: impl Into<String>) -> Self {
        Self::Unavailable {
            url,
            reason: reason.into(),
        }
    }

    pub fn title(&self) -> &str {
        match self {
            Self::Blank => "Blank",
            Self::Internal(page) => &page.title,
            Self::Unavailable { .. } => "Unavailable",
        }
    }

    pub fn url(&self) -> Option<&Url> {
        match self {
            Self::Blank => None,
            Self::Internal(page) => Some(&page.url),
            Self::Unavailable { url, .. } => Some(url),
        }
    }
}

/// Structured internal page. Chrome renders this; there is no HTML here.
#[derive(Clone, Debug)]
pub struct InternalPage {
    pub title: String,
    pub url: Url,
    pub blocks: Vec<Block>,
}

/// Building blocks for `about:` pages.
#[derive(Clone, Debug)]
pub enum Block {
    Hero {
        title: String,
        subtitle: String,
    },
    Heading(String),
    Paragraph(String),
    Note(String),
    Divider,
    KeyValue {
        key: String,
        value: String,
    },
    Toggle {
        id: PrefToggle,
        label: String,
        description: String,
        value: bool,
    },
    Link {
        label: String,
        href: String,
    },
    List(Vec<String>),
}
