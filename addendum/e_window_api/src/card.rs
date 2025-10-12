use crate::{EWindow, Result, WindowConfig};
use std::collections::HashMap;

/// Represents an anchor link in a card
#[derive(Debug, Clone)]
pub struct Anchor {
    pub text: String,
    pub href: Option<String>,
}

impl Anchor {
    /// Create a new anchor with text and href
    pub fn new<T: Into<String>, H: Into<String>>(text: T, href: H) -> Self {
        Self {
            text: text.into(),
            href: Some(href.into()),
        }
    }

    /// Create a new text-only anchor (no clickable action)
    pub fn text_only<T: Into<String>>(text: T) -> Self {
        Self {
            text: text.into(),
            href: None,
        }
    }
}

/// Represents a card/document to be displayed in an e_window
#[derive(Debug, Clone)]
pub struct Card {
    /// Window configuration that should be applied when showing this card
    pub window_title: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub x: Option<i32>,
    pub y: Option<i32>,

    /// Key-value fields to display
    pub fields: HashMap<String, CardField>,

    /// Content structure (after the blank line separator)
    /// First line after blank line - the card title
    pub title: Option<String>,
    /// Second line after blank line - the header
    pub header: Option<String>,
    /// Third line after blank line - the caption  
    pub caption: Option<String>,
    /// Remaining lines - the body content
    pub body: Vec<String>,

    /// Clickable anchor links
    pub anchors: Vec<Anchor>,

    /// Legacy content field - deprecated, use structured fields above
    #[deprecated(note = "Use title, header, caption, and body fields instead")]
    pub content: String,

    /// Additional content lines - deprecated
    #[deprecated(note = "Use body field instead")]
    pub additional_content: Vec<String>,
}

/// A field in a card with a value and type
#[derive(Debug, Clone)]
pub struct CardField {
    pub value: String,
    pub field_type: String,
}

impl Default for Card {
    fn default() -> Self {
        Self {
            window_title: None,
            width: None,
            height: None,
            x: None,
            y: None,
            fields: HashMap::new(),
            title: None,
            header: None,
            caption: None,
            body: Vec::new(),
            anchors: Vec::new(),
            #[allow(deprecated)]
            content: String::new(),
            #[allow(deprecated)]
            additional_content: Vec::new(),
        }
    }
}

impl Card {
    /// Create a new empty card
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the window title for this card  
    pub fn window_title<S: Into<String>>(mut self, title: S) -> Self {
        self.window_title = Some(title.into());
        self
    }

    /// Set the window title for this card (deprecated - use window_title instead)
    #[deprecated(note = "Use window_title() instead to avoid confusion with content title")]
    pub fn title<S: Into<String>>(mut self, title: S) -> Self {
        self.window_title = Some(title.into());
        self
    }

    /// Set the window size for this card
    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.width = Some(width);
        self.height = Some(height);
        self
    }

    /// Set the window position for this card
    pub fn position(mut self, x: i32, y: i32) -> Self {
        self.x = Some(x);
        self.y = Some(y);
        self
    }

    /// Add a field to the card
    pub fn field<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.fields.insert(
            key.into(),
            CardField {
                value: value.into(),
                field_type: "string".to_string(),
            },
        );
        self
    }

    /// Add a typed field to the card
    pub fn typed_field<K, V, T>(mut self, key: K, value: V, field_type: T) -> Self
    where
        K: Into<String>,
        V: Into<String>,
        T: Into<String>,
    {
        self.fields.insert(
            key.into(),
            CardField {
                value: value.into(),
                field_type: field_type.into(),
            },
        );
        self
    }

    /// Add an expiration field - shows countdown timer until auto-close
    /// Duration should be specified in seconds
    pub fn expiration<K>(mut self, key: K, duration_seconds: u64) -> Self
    where
        K: Into<String>,
    {
        self.fields.insert(
            key.into(),
            CardField {
                value: duration_seconds.to_string(),
                field_type: "expiration".to_string(),
            },
        );
        self
    }

    /// Add a duration field - shows elapsed time since start
    pub fn duration_field<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.fields.insert(
            key.into(),
            CardField {
                value: value.into(),
                field_type: "duration".to_string(),
            },
        );
        self
    }

    /// Add a datetime field
    pub fn datetime_field<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.fields.insert(
            key.into(),
            CardField {
                value: value.into(),
                field_type: "datetime".to_string(),
            },
        );
        self
    }

    /// Set the main content
    #[deprecated(note = "Use card_title(), header(), caption(), and body() methods instead")]
    pub fn content<S: Into<String>>(mut self, content: S) -> Self {
        #[allow(deprecated)]
        {
            self.content = content.into();
        }
        self
    }

    /// Add additional content line
    #[deprecated(note = "Use body() method instead")]
    pub fn add_content<S: Into<String>>(mut self, content: S) -> Self {
        #[allow(deprecated)]
        {
            self.additional_content.push(content.into());
        }
        self
    }

    /// Set the card title (first line after blank line separator)
    pub fn card_title<S: Into<String>>(mut self, title: S) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the header (second line after blank line separator)
    pub fn header<S: Into<String>>(mut self, header: S) -> Self {
        self.header = Some(header.into());
        self
    }

    /// Set the caption (third line after blank line separator)
    pub fn caption<S: Into<String>>(mut self, caption: S) -> Self {
        self.caption = Some(caption.into());
        self
    }

    /// Add a line to the body content
    pub fn body_line<S: Into<String>>(mut self, line: S) -> Self {
        self.body.push(line.into());
        self
    }

    /// Set the body content from multiple lines
    pub fn body_lines<I, S>(mut self, lines: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.body = lines.into_iter().map(|s| s.into()).collect();
        self
    }

    /// Set the body content from a single string (splits on newlines)
    pub fn body<S: Into<String>>(mut self, content: S) -> Self {
        let content = content.into();
        if content.is_empty() {
            self.body = Vec::new();
        } else {
            self.body = content.lines().map(|s| s.to_string()).collect();
        }
        self
    }

    /// Add a clickable anchor link with text and href
    pub fn anchor<T: Into<String>, H: Into<String>>(mut self, text: T, href: H) -> Self {
        self.anchors.push(Anchor::new(text, href));
        self
    }

    /// Add a text-only anchor (no clickable action)
    pub fn anchor_text<T: Into<String>>(mut self, text: T) -> Self {
        self.anchors.push(Anchor::text_only(text));
        self
    }

    /// Add multiple anchors at once
    pub fn anchors<I>(mut self, anchors: I) -> Self
    where
        I: IntoIterator<Item = Anchor>,
    {
        self.anchors.extend(anchors);
        self
    }

    /// Convert the card to the e_window document format
    pub fn to_document(&self) -> String {
        let mut doc = String::new();

        // Add window configuration if specified
        if let Some(ref title) = self.window_title {
            doc.push_str(&format!("--title \"{}\"", title));
        }
        if let Some(width) = self.width {
            doc.push_str(&format!(" --width {}", width));
        }
        if let Some(height) = self.height {
            doc.push_str(&format!(" --height {}", height));
        }
        if let Some(x) = self.x {
            doc.push_str(&format!(" --x {}", x));
        }
        if let Some(y) = self.y {
            doc.push_str(&format!(" --y {}", y));
        }

        // If we added any window config, add a newline
        if !doc.is_empty() {
            doc.push('\n');
        }

        // Add fields
        for (key, field) in &self.fields {
            doc.push_str(&format!(
                "{} | {} | {}\n",
                key, field.value, field.field_type
            ));
        }

        // Check if we have structured content
        let has_structured_content = self.title.is_some()
            || self.header.is_some()
            || self.caption.is_some()
            || !self.body.is_empty();

        // Check if we have legacy content (for backward compatibility)
        #[allow(deprecated)]
        let has_legacy_content = !self.content.is_empty() || !self.additional_content.is_empty();

        let has_content = has_structured_content || has_legacy_content;

        // Add separator line if we have CLI arguments or fields and content
        let has_cli_args = !doc.is_empty();
        if has_cli_args && has_content {
            doc.push('\n');
        }

        // Add structured content according to e_window protocol:
        // Line 1: Title
        // Line 2: Header
        // Line 3: Caption
        // Lines 4+: Body
        if has_structured_content {
            // Add title (or empty line if none)
            if let Some(ref title) = self.title {
                doc.push_str(title);
            }
            doc.push('\n');

            // Add header (or empty line if none)
            if let Some(ref header) = self.header {
                doc.push_str(header);
            }
            doc.push('\n');

            // Add caption (or empty line if none)
            if let Some(ref caption) = self.caption {
                doc.push_str(caption);
            }
            doc.push('\n');

            // Add body lines
            for line in &self.body {
                doc.push_str(line);
                doc.push('\n');
            }
        } else if has_legacy_content {
            // Fallback to legacy content for backward compatibility
            #[allow(deprecated)]
            {
                if !self.content.is_empty() {
                    doc.push_str(&self.content);
                    doc.push('\n');
                }

                for content in &self.additional_content {
                    doc.push_str(content);
                    doc.push('\n');
                }
            }
        }

        // Add anchors
        for anchor in &self.anchors {
            if let Some(ref href) = anchor.href {
                doc.push_str(&format!("anchor: {} | {}\n", anchor.text, href));
            } else {
                doc.push_str(&format!("anchor: {}\n", anchor.text));
            }
        }

        doc
    }

    /// Show this card in a new e_window
    pub async fn show(&self) -> Result<()> {
        // Create window config from card settings
        let mut config = WindowConfig::new();

        if let Some(ref title) = self.window_title {
            config = config.title(title);
        }
        if let Some(width) = self.width {
            if let Some(height) = self.height {
                config = config.size(width, height);
            }
        }
        if let Some(x) = self.x {
            config = config.position(x, self.y.unwrap_or(0));
        } else if let Some(y) = self.y {
            config = config.position(0, y);
        }

        // Launch window and show card
        let mut window = EWindow::launch(config).await?;
        window.show_card(self.clone()).await?;
        window.wait().await?;

        Ok(())
    }

    /// Show this card in a new e_window (blocking version)
    pub fn show_blocking(&self) -> Result<()> {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(self.show())
    }
}

impl std::fmt::Display for Card {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_document())
    }
}
