//! Validation and weapon metadata for standalone TF2 items_game.txt files.
use anyhow::{Context, Result};
use std::{collections::HashMap, fs, path::Path};

#[derive(Clone, Debug)]
pub(crate) struct ItemInfo {
    pub(crate) slot: String,
    pub(crate) log_name: String,
}

pub(crate) fn load(path: &Path) -> Result<HashMap<i64, ItemInfo>> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("could not read item schema {}", path.display()))?;
    parse_items(&text).map_err(|()| anyhow::anyhow!("invalid item schema {}", path.display()))
}

pub fn validate(path: &Path) -> Result<(), String> {
    if !path.is_file() { return Err("File not found. Choose items_game.txt, use How to find, or continue without a schema.".into()); }
    if !path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("txt")) {
        return Err("Incorrect file type. Select a standalone items_game.txt file.".into());
    }
    let text = fs::read_to_string(path).map_err(|_| "Cannot read this file as text. Select items_game.txt.".to_owned())?;
    parse_items(&text)
        .map(|_| ())
        .map_err(|()| "Invalid item schema. Select a complete TF2 items_game.txt file.".to_owned())
}

#[derive(Debug, PartialEq)]
enum Token { Text(String), Open, Close }

fn tokenize(text: &str) -> Result<Vec<Token>, ()> {
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
    Ok(tokens)
}

#[derive(Default)]
struct ItemFields<'a> {
    name: Option<&'a str>,
    log_name: Option<&'a str>,
    slot: &'a str,
}

fn parse_items(text: &str) -> Result<HashMap<i64, ItemInfo>, ()> {
    let tokens = tokenize(text)?;
    // Only direct fields of numeric definitions under items_game/items
    // belong to this lookup. Other sections reuse the same numeric keys.
    let mut stack: Vec<&str> = Vec::new();
    let mut current_item: Option<(i64, ItemFields<'_>)> = None;
    let mut items = HashMap::new();
    let mut i = 0;
    let mut found_root = false;
    let mut found_named_item = false;
    while i < tokens.len() {
        if tokens[i] == Token::Close {
            if stack.len() == 3 {
                if let Some((index, fields)) = current_item.take() {
                    found_named_item |= fields.name.is_some();
                    items.insert(index, ItemInfo {
                        slot: fields.slot.to_ascii_lowercase(),
                        log_name: fields.log_name.or(fields.name).unwrap_or_default()
                            .trim_start_matches('#').to_ascii_lowercase(),
                    });
                }
            }
            stack.pop().ok_or(())?;
            i += 1;
            continue;
        }
        let Token::Text(key) = &tokens[i] else { return Err(()); };
        match tokens.get(i + 1).ok_or(())? {
            Token::Open => {
                if stack.is_empty() {
                    if found_root || key != "items_game" { return Err(()); }
                    found_root = true;
                }
                if stack == ["items_game", "items"] {
                    current_item = key.parse::<u32>().ok()
                        .map(|index| (i64::from(index), ItemFields::default()));
                }
                stack.push(key);
            },
            Token::Text(value) => {
                if stack.is_empty() { return Err(()); }
                if stack.len() == 3 {
                    if let Some((_, fields)) = current_item.as_mut() {
                        match key.as_str() {
                            "name" if fields.name.is_none() && !value.trim().is_empty() => fields.name = Some(value),
                            "item_logname" if fields.log_name.is_none() && !value.trim_start_matches('#').trim().is_empty() => {
                                fields.log_name = Some(value);
                            }
                            "item_slot" => fields.slot = value,
                            _ => {}
                        }
                    }
                }
            },
            Token::Close => return Err(()),
        }
        i += 2;
    }
    if found_root && found_named_item && stack.is_empty() { Ok(items) } else { Err(()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepts_schema_with_comments_bom_and_escaped_quotes() {
        assert!(parse_items("\u{feff}// TF2\n\"items_game\" { \"items\" { \"1\" { \"name\" \"weapon \\\"test\\\"\" } } }").is_ok());
    }

    #[test]
    fn extracts_only_direct_item_fields_and_prefers_explicit_log_name() {
        let items = parse_items(r##"
            items_game {
                items {
                    20 {
                        name "#Stock Sticky {launcher}"
                        item_logname "Stickybomb_Launcher" // } map entries below reuse 20
                        item_slot "Secondary"
                        attributes { 20 { name ignored item_logname ignored item_slot ignored } }
                    }
                    19 { item_logname "GrenadeLauncher" name "Stock Grenades" }
                    21 { name "#Stock Shotgun" item_logname "" }
                    -1 { name ignored }
                    4294967296 { name ignored }
                }
                maps { 20 { name cp_egypt_final item_slot ignored } 22 { name cp_steel } }
            }
        "##).unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[&20].log_name, "stickybomb_launcher");
        assert_eq!(items[&20].slot, "secondary");
        assert_eq!(items[&19].log_name, "grenadelauncher");
        assert!(items[&19].slot.is_empty());
        assert_eq!(items[&21].log_name, "stock shotgun");
    }

    #[test]
    fn rejects_unrelated_empty_truncated_and_malformed_files() {
        for text in ["", "hello", "\"other\" {}", "items_game { items {} }", "items_game { items { 1 { name weapon } }", "items_game { items { 1 { name } } }", "items_game { items { 1 { name weapon } } } }", "items_game { items { 1 { name \"unterminated } } }"] {
            assert!(parse_items(text).is_err(), "{text}");
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
