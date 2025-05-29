// src/parser.rs


pub struct ParsedText {
    pub triples: Vec<(String, String, String)>,
    pub title: Option<String>,
    pub header: Option<String>,
    pub caption: Option<String>,
    pub body: Option<String>,
}

pub fn parse_text(input: &str) -> ParsedText {
    let mut triples = Vec::new();
    let mut lines = input.lines().peekable();

    // Parse key | value | type lines until an empty line
    while let Some(line) = lines.peek() {
        if line.trim().is_empty() {
            lines.next(); // consume the empty line
            break;
        }
        let parts: Vec<&str> = line.split('|').map(|s| s.trim()).collect();
        if parts.len() == 3 {
            triples.push((
                parts[0].to_string(),
                parts[1].to_string(),
                parts[2].to_string(),
            ));
        }
        lines.next();
    }

    // Parse title, header, caption, body
    let title = lines.next().map(|s| s.to_string());
    let header = lines.next().map(|s| s.to_string());
    let caption = lines.next().map(|s| s.to_string());
    let body = {
        let body_lines: Vec<String> = lines.map(|s| s.to_string()).collect();
        if body_lines.is_empty() {
            None
        } else {
            Some(body_lines.join("\n"))
        }
    };

    ParsedText {
        triples,
        title,
        header,
        caption,
        body,
    }
}
