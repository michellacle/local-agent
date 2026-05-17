use std::collections::BTreeSet;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

/// Represents a parsed domain entity (struct, enum, or trait).
struct Entity {
    kind: String,
    name: String,
    doc: String,
    fields: Vec<Field>,
}

/// Represents a field within an entity.
struct Field {
    name: String,
    r#type: String,
    doc: String,
}

fn main() {
    let project_root = find_project_root();
    let src_dir = project_root.join("src");
    let docs_dir = project_root.join("docs");
    fs::create_dir_all(&docs_dir).expect("failed to create docs/");

    let mut entities = parse_all_sources(&src_dir);
    entities.sort_by(|a, b| a.name.cmp(&b.name));

    let language_path = docs_dir.join("domain-language.md");
    let mut lang_file =
        fs::File::create(&language_path).expect("failed to create domain-language.md");
    generate_domain_language(&mut lang_file, &entities);
    eprintln!("Generated {}", language_path.display());

    let mermaid_path = docs_dir.join("domain-model.mmd");
    let mut mm_file = fs::File::create(&mermaid_path).expect("failed to create domain-model.mmd");
    generate_mermaid(&mut mm_file, &entities);
    eprintln!("Generated {}", mermaid_path.display());
}

fn find_project_root() -> PathBuf {
    let mut current = std::env::current_dir().expect("cannot determine cwd");
    loop {
        if current.join("Cargo.toml").exists() {
            return current;
        }
        if !current.pop() {
            break;
        }
    }
    std::env::current_dir().expect("fallback to cwd")
}

fn parse_all_sources(src_dir: &std::path::Path) -> Vec<Entity> {
    let mut entities = Vec::new();
    let entries = fs::read_dir(src_dir).expect("failed to read src/");
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let fname = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if fname == "lib.rs" || fname == "main.rs" {
            continue;
        }
        if fname.starts_with("bin") {
            continue;
        }
        entities.extend(parse_file(&path));
    }
    entities
}

fn parse_file(path: &std::path::Path) -> Vec<Entity> {
    let file = fs::File::open(path).expect("failed to open source file");
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().map_while(Result::ok).collect();

    let mut entities = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = &lines[i];
        let trimmed = line.trim();

        // Collect leading /// doc comments
        let mut doc_lines = Vec::new();
        let mut start = i;
        while start > 0 {
            let prev = &lines[start - 1];
            if prev.trim_start().starts_with("///") {
                let doc = prev
                    .trim_start()
                    .trim_start_matches("///")
                    .trim()
                    .to_string();
                doc_lines.insert(0, doc);
                start -= 1;
            } else {
                break;
            }
        }

        // Match: pub struct Name, pub enum Name, pub trait Name
        if let Some((kind, name)) = try_parse_entity_header(trimmed) {
            let doc = doc_lines.join("\n");
            let mut fields = Vec::new();

            // Count braces on the header line
            let mut brace_depth: i32 =
                trimmed.matches('{').count() as i32 - trimmed.matches('}').count() as i32;

            i += 1;
            let mut pending_field_doc = String::new();

            while i < lines.len() && brace_depth > 0 {
                let fline = &lines[i];
                let ftrimmed = fline.trim();

                brace_depth +=
                    ftrimmed.matches('{').count() as i32 - ftrimmed.matches('}').count() as i32;

                // Field doc comment
                if ftrimmed.starts_with("///") {
                    pending_field_doc = ftrimmed.trim_start_matches("///").trim().to_string();
                    i += 1;
                    continue;
                }

                // Field definition: pub name: Type or name: Type (skip fn, #[attr], etc.)
                if !pending_field_doc.is_empty()
                    && ftrimmed.contains(':')
                    && !ftrimmed.starts_with("fn ")
                    && !ftrimmed.starts_with("#")
                    && (ftrimmed.starts_with("pub ") || ftrimmed.starts_with(char::is_alphanumeric))
                {
                    if let Some(field) = try_parse_field(ftrimmed, &pending_field_doc) {
                        fields.push(field);
                    }
                    pending_field_doc.clear();
                    i += 1;
                    continue;
                }

                // Reset pending doc if we hit a non-field, non-attr line
                if !ftrimmed.starts_with("///") && !ftrimmed.starts_with("#[") {
                    pending_field_doc.clear();
                }

                i += 1;
            }

            entities.push(Entity {
                kind,
                name,
                doc,
                fields,
            });
        } else {
            i += 1;
        }
    }

    entities
}

fn try_parse_entity_header(line: &str) -> Option<(String, String)> {
    if let Some(rest) = line.strip_prefix("pub struct ") {
        let name = rest.split_whitespace().next()?.to_string();
        return Some(("struct".into(), name));
    }
    if let Some(rest) = line.strip_prefix("pub enum ") {
        let name = rest.split_whitespace().next()?.to_string();
        return Some(("enum".into(), name));
    }
    if let Some(rest) = line.strip_prefix("pub trait ") {
        // "CacheStore: Send + Sync {" -> "CacheStore"
        let name = rest
            .split_whitespace()
            .next()?
            .split(':')
            .next()?
            .trim()
            .to_string();
        return Some(("trait".into(), name));
    }
    None
}

fn try_parse_field(line: &str, doc: &str) -> Option<Field> {
    let trimmed = line.trim();
    // Handle both `pub field: Type` and `field: Type`
    let rest = trimmed.strip_prefix("pub ").unwrap_or(trimmed);
    let colon_pos = rest.find(':')?;
    let name = rest[..colon_pos].trim().to_string();
    let mut r#type = rest[colon_pos + 1..].trim().to_string();
    r#type = r#type.trim_end_matches(',').trim().to_string();
    r#type = r#type.replace("<'_>", "");

    if name.is_empty() || r#type.is_empty() {
        return None;
    }

    Some(Field {
        name,
        r#type,
        doc: doc.to_string(),
    })
}

fn generate_domain_language<W: Write>(out: &mut W, entities: &[Entity]) {
    writeln!(out, "# Domain Language").unwrap();
    writeln!(out).unwrap();
    writeln!(
        out,
        "Auto-generated vocabulary of domain entities from source code."
    )
    .unwrap();
    writeln!(out).unwrap();

    for entity in entities {
        let kind_label = match entity.kind.as_str() {
            "enum" => "Enum",
            "struct" => "Struct",
            "trait" => "Trait",
            other => other,
        };

        writeln!(out, "## {}", entity.name).unwrap();
        writeln!(out).unwrap();
        writeln!(out, "**Kind:** {}", kind_label).unwrap();
        writeln!(out).unwrap();
        if !entity.doc.is_empty() {
            writeln!(out, "{}", entity.doc).unwrap();
            writeln!(out).unwrap();
        }

        if !entity.fields.is_empty() {
            writeln!(out, "### Fields").unwrap();
            writeln!(out).unwrap();
            writeln!(out, "| Field | Type | Description |").unwrap();
            writeln!(out, "| ----- | ---- | ----------- |").unwrap();
            for field in &entity.fields {
                let bt = char::from(0x60);
                writeln!(
                    out,
                    "| {}{}{} | {}{}{} | {} |",
                    bt, field.name, bt, bt, field.r#type, bt, field.doc
                )
                .unwrap();
            }
            writeln!(out).unwrap();
        } else {
            writeln!(out).unwrap();
        }
    }
}

fn generate_mermaid<W: Write>(out: &mut W, entities: &[Entity]) {
    writeln!(out, "```mermaid").unwrap();
    writeln!(out, "%% Domain Model \u{2014} auto-generated from rustdoc").unwrap();
    writeln!(out, "classDiagram").unwrap();
    writeln!(out).unwrap();

    let known_names: BTreeSet<&str> = entities.iter().map(|e| e.name.as_str()).collect();

    for entity in entities {
        writeln!(out, "  class {} {{", entity.name).unwrap();
        if entity.kind == "enum" {
            writeln!(out, "    <<enum>>").unwrap();
        } else if entity.kind == "trait" {
            writeln!(out, "    <<trait>>").unwrap();
        }
        if !entity.doc.is_empty() {
            let escaped = entity.doc.replace('"', "\\\"");
            writeln!(out, "    {}", escaped).unwrap();
        }
        for field in &entity.fields {
            writeln!(
                out,
                "    {} : {}",
                clean_type_for_mermaid(&field.r#type),
                field.name
            )
            .unwrap();
        }
        writeln!(out, "  }}").unwrap();
        writeln!(out).unwrap();
    }

    for entity in entities {
        for field in &entity.fields {
            let clean = clean_type_for_relationship(&field.r#type);
            if known_names.contains(clean.as_str()) {
                writeln!(out, "  {} --> \"{}\" {}", entity.name, field.name, clean).unwrap();
            }
        }
    }

    writeln!(out, "```").unwrap();
}

fn clean_type_for_mermaid(r#type: &str) -> String {
    let t = r#type;
    if let Some(inner) = t.strip_prefix("Option<").and_then(|s| s.strip_suffix(">")) {
        return inner.trim().to_string();
    }
    if let Some(inner) = t.strip_prefix("Box<dyn ").and_then(|s| s.strip_suffix(">")) {
        return inner.trim().to_string();
    }
    if let Some(inner) = t.strip_prefix("Box<").and_then(|s| s.strip_suffix(">")) {
        return inner.trim().to_string();
    }
    t.to_string()
}

fn clean_type_for_relationship(r#type: &str) -> String {
    let cleaned = clean_type_for_mermaid(r#type);
    if let Some(bracket) = cleaned.find('<') {
        cleaned[..bracket].trim().to_string()
    } else {
        cleaned
    }
}
