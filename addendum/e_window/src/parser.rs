use serde::{Deserialize, Serialize};
use snailquote::unescape;

fn unescape_debug_string(debug_string: &str) -> Result<String, snailquote::UnescapeError> {
    let debug_string = debug_string.replace(r"\n", "\n");
    unescape(&debug_string)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Anchor {
    pub text: String,
    pub href: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ParsedText {
    pub triples: Vec<(String, String, String)>,
    pub title: Option<String>,
    pub header: Option<String>,
    pub caption: Option<String>,
    pub body: Option<String>,
    pub anchors: Vec<Anchor>,
}

pub fn parse_text(input: &str, decode_debug: bool) -> ParsedText {
    let mut triples = Vec::new();

    let mut anchors = Vec::new();
    let mut filtered_lines = Vec::new();
    for line in input.lines() {
        if line.starts_with("anchor:") {
            let anchor_line = line.trim_start_matches("anchor:").trim();
            if let Some((text, href)) = anchor_line.split_once("|") {
                anchors.push(Anchor {
                    text: text.trim().to_string(),
                    href: href.trim().to_string(),
                });
                println!(
                    "Debug: Parsed anchor - text: {}, href: {}",
                    text.trim(),
                    href.trim()
                );
            } else {
                anchors.push(Anchor {
                    text: anchor_line.to_string(),
                    href: String::new(),
                });
                println!(
                    "Debug: Parsed anchor - text: {}, href: <empty>",
                    anchor_line
                );
            }
        } else {
            filtered_lines.push(line);
        }
    }

    let mut lines = filtered_lines.into_iter().peekable();

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
    let mut title = lines.next().map(|s| s.to_string());
    println!("Debug: Parsed title: {:?}", title);
    let mut header = lines.next().map(|s| s.to_string());
    println!("Debug: Parsed header: {:?}", header);
    let mut caption = lines.next().map(|s| s.to_string());
    println!("Debug: Parsed caption: {:?}", caption);
    let mut body = {
        let body_lines: Vec<String> = lines.map(|s| s.to_string()).collect();
        if body_lines.is_empty() {
            None
        } else {
            Some(body_lines.join("\n"))
        }
    };
    println!("Debug: Parsed body: {:?}", body);

    if decode_debug {
        println!("Debug: Decoding debug strings...");
        title = title.map(|s| unescape_debug_string(&s).unwrap_or(s));
        header = header.map(|s| unescape_debug_string(&s).unwrap_or(s));
        caption = caption.map(|s| unescape_debug_string(&s).unwrap_or(s));
        body = body.map(|s| unescape_debug_string(&s).unwrap_or(s));
        println!("{:#?}", anchors);
        anchors = anchors
            .into_iter()
            .map(|mut a| {
                a.text = match unescape_debug_string(&a.text) {
                    Ok(decoded) => decoded,
                    Err(e) => {
                        println!("Error decoding anchor text '{}': {:?}", a.text, e);
                        a.text
                    }
                };
                println!("Debug: Decoded anchor text: {}", a.text);
                a
            })
            .collect();
        println!("{:#?}", anchors);
    } else {
        println!("Debug: Skipping decoding of debug strings.");
    }

    // Debugging: Print input lines and parsed anchors
    println!("Debug: Input lines:");
    for line in input.lines() {
        println!("Line: {}", line);
    }

    let ret = ParsedText {
        triples,
        title,
        header,
        caption,
        body,
        anchors,
    };
    println!("Debug: Parsed text: {:?}", ret);
    ret
}
