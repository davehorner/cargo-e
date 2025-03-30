pub mod cargo_utils;
pub mod cli;
pub mod clipboard;
pub mod crate_recreator_py;
pub mod crate_recreator_rs;
pub mod file_gatherer;
pub mod summarizer;
//raw input → preprocess → sanitize → literal encode
//[filesystem traversal] --> [per-file processor] --> [per-line or per-comment processor] --> [emit events]

/// Sanitizes the input string by removing non-printable control characters,
/// except for commonly used whitespace characters: newline (`\n`), carriage return (`\r`), and tab (`\t`).
///
/// This function preserves valid Unicode characters, including emojis and accented characters.
///
/// Example:
/// ```
/// use e_ai_summarize::sanitize;
/// let cleaned = sanitize("Hello\u{0000} World!\nNext line.");
/// assert_eq!(cleaned, "Hello World!\nNext line.");
/// ```
pub fn sanitize(s: impl AsRef<str>) -> String {
    s.as_ref()
        .chars()
        .filter(|&c| {
            // Filter out the replacement character explicitly
            if c == '\u{FFFD}' {
                return false;
            }
            !(c.is_control() && c != '\n' && c != '\r' && c != '\t')
        })
        .collect()
}

/// Generates a raw string literal using enough `#` to safely surround the input
pub fn make_raw_string_literal(input: &str) -> String {
    let mut max_hashes = 0;
    let mut current = 0;

    for ch in input.chars() {
        if ch == '#' {
            current += 1;
            max_hashes = max_hashes.max(current);
        } else {
            current = 0;
        }
    }

    let num_hashes = max_hashes + 1;
    let hashes = "#".repeat(num_hashes);
    format!("r{hashes}\"{}\"{hashes}", input)
}

/// Decides if it should emit as raw string or escaped string
pub fn emit_literal(input: &str) -> String {
    if input.contains('\r') {
        // fallback if we have a bare \r
        format!("{:?}", input)
    } else {
        make_raw_string_literal(input)
    }
}

pub struct PreprocessConfig {
    pub trim_whitespace: bool,
    pub collapse_blank_lines: bool,
    pub max_comment_block_lines: Option<usize>, // e.g., Some(4)
    pub remove_long_comments: bool,
}

pub fn preprocess_text(input: &str, config: &PreprocessConfig) -> String {
    let mut output = String::new();
    let mut comment_block = Vec::new();
    let mut in_comment = false;

    for line in input.lines() {
        let is_comment = line.trim_start().starts_with("//");

        if is_comment {
            comment_block.push(line);
            in_comment = true;
        } else {
            if in_comment {
                // End of comment block
                if config.remove_long_comments
                    && comment_block.len() > config.max_comment_block_lines.unwrap_or(usize::MAX)
                {
                    // Skip comment block
                } else {
                    for comment_line in &comment_block {
                        output.push_str(comment_line);
                        output.push('\n');
                    }
                }
                comment_block.clear();
                in_comment = false;
            }

            let mut processed_line = line.to_string();

            if config.trim_whitespace {
                processed_line = processed_line.trim().to_string();
            }

            output.push_str(&processed_line);
            output.push('\n');
        }
    }

    // Flush any remaining comment block
    if in_comment {
        if !config.remove_long_comments
            || comment_block.len() <= config.max_comment_block_lines.unwrap_or(usize::MAX)
        {
            for comment_line in &comment_block {
                output.push_str(comment_line);
                output.push('\n');
            }
        }
    }

    if config.collapse_blank_lines {
        // Optional: collapse multiple blank lines into one
        output = output
            .lines()
            .fold((String::new(), false), |(mut acc, mut last_blank), line| {
                if line.trim().is_empty() {
                    if !last_blank {
                        acc.push_str("\n");
                        last_blank = true;
                    }
                } else {
                    acc.push_str(line);
                    acc.push('\n');
                    last_blank = false;
                }
                (acc, last_blank)
            })
            .0;
    }

    output
}
