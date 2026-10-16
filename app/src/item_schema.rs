//! Validation for optional, standalone TF2 items_game.txt files.
use std::{fs, path::Path};

pub fn validate(path: &Path) -> Result<(), String> {
    if !path.is_file() { return Err("File not found. Choose items_game.txt, use How to find, or continue without a schema.".into()); }
    if !path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("txt")) {
        return Err("Incorrect file type. Select a standalone items_game.txt file.".into());
    }
    let text = fs::read_to_string(path).map_err(|_| "Cannot read this file as text. Select items_game.txt.".to_owned())?;
    validate_text(&text).map_err(|_| "Invalid item schema. Select a complete TF2 items_game.txt file.".to_owned())
}

#[derive(Debug, PartialEq)]
enum Token { Text(String), Open, Close }

fn validate_text(text: &str) -> Result<(), ()> {
    let mut chars = text.trim_start_matches('\u{feff}').chars().peekable();
    let mut tokens = Vec::new();
    while let Some(c) = chars.next() {
        match c {
            c if c.is_whitespace() => {},
            '/' if chars.peek() == Some(&'/') => { for c in chars.by_ref() { if c == '\n' { break; } } },
            '{' => tokens.push(Token::Open),
            '}' => tokens.push(Token::Close),
            '"' => {
                let mut value = String::new();
                loop {
                    match chars.next().ok_or(())? {
                        '"' => break,
                        '\\' => { value.push(chars.next().ok_or(())?); },
                        c => value.push(c),
                    }
                }
                tokens.push(Token::Text(value));
            },
            c => {
                let mut value = String::from(c);
                while chars.peek().is_some_and(|c| !c.is_whitespace() && !matches!(c, '{' | '}' | '"')) {
                    value.push(chars.next().unwrap());
                }
                tokens.push(Token::Text(value));
            },
        }
    }
    // Parse key/value pairs iteratively, validating all braces and values.
    let mut stack: Vec<String> = Vec::new();
    let mut i = 0;
    let mut root = false;
    let mut item = false;
    while i < tokens.len() {
        if tokens[i] == Token::Close {
            stack.pop().ok_or(())?;
            i += 1;
            continue;
        }
        let Token::Text(key) = &tokens[i] else { return Err(()); };
        match tokens.get(i + 1).ok_or(())? {
            Token::Open => {
                if stack.is_empty() {
                    if root || key != "items_game" { return Err(()); }
                    root = true;
                }
                stack.push(key.clone());
            },
            Token::Text(value) => {
                if stack.is_empty() { return Err(()); }
                if stack.len() == 3 && stack[0] == "items_game" && stack[1] == "items"
                    && stack[2].parse::<u32>().is_ok() && key == "name" && !value.trim().is_empty() {
                    item = true;
                }
            },
            Token::Close => return Err(()),
        }
        i += 2;
    }
    if root && item && stack.is_empty() { Ok(()) } else { Err(()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepts_schema_with_comments_bom_and_escaped_quotes() {
        assert!(validate_text("\u{feff}// TF2\n\"items_game\" { \"items\" { \"1\" { \"name\" \"weapon \\\"test\\\"\" } } }").is_ok());
    }
    #[test]
    fn rejects_unrelated_empty_truncated_and_malformed_files() {
        for text in ["", "hello", "\"other\" {}", "items_game { items {} }", "items_game { items { 1 { name weapon } }", "items_game { items { 1 { name } } }", "items_game { items { 1 { name weapon } } } }", "items_game { items { 1 { name \"unterminated } } }"] {
            assert!(validate_text(text).is_err(), "{text}");
        }
    }
    #[test]
    fn checks_file_type_existence_and_contents() {
        let dir = std::env::temp_dir().join(format!("schema-validation-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("items_game.txt");
        assert!(validate(&path).is_err());
        fs::write(&path, "unrelated text").unwrap();
        assert!(validate(&path).is_err());
        fs::write(&path, "items_game { items { 1 { name weapon } } }").unwrap();
        assert!(validate(&path).is_ok());
        let wrong = dir.join("schema.json");
        fs::copy(&path, &wrong).unwrap();
        assert!(validate(&wrong).is_err());
        fs::remove_dir_all(dir).unwrap();
    }
}
