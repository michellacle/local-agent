use std::collections::{BTreeMap, BTreeSet};
use std::fs;

#[derive(Debug, Clone)]
struct DomainType {
    name: String,
    kind: TypeKind,
    module: String,
    fields: Vec<Field>,
    variants: Vec<String>,
    implements: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
enum TypeKind {
    Struct,
    Enum,
    Trait,
}

#[derive(Debug, Clone)]
struct Field {
    name: String,
    ty: String,
}

fn main() {
    let src_dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "src".to_string());

    let types = parse_source_files(&src_dir);
    let relationships = extract_relationships(&types);

    let docs_dir = "docs";
    fs::create_dir_all(docs_dir).expect("Failed to create docs directory");

    let mermaid = generate_mermaid(&types, &relationships);
    fs::write(format!("{}/domain-model.mmd", docs_dir), &mermaid)
        .expect("Failed to write Mermaid diagram");

    let ascii = generate_ascii(&types, &relationships);
    fs::write(format!("{}/domain-model.txt", docs_dir), &ascii)
        .expect("Failed to write ASCII diagram");

    println!("Generated docs/domain-model.mmd (Mermaid)");
    println!("Generated docs/domain-model.txt (ASCII)");
}

fn parse_source_files(src_dir: &str) -> BTreeMap<String, DomainType> {
    let mut types = BTreeMap::new();

    let entries = fs::read_dir(src_dir).expect("Failed to read src directory");
    for entry in entries.flatten() {
        let path = entry.path();
        match path.extension().and_then(|e| e.to_str()) {
            Some("rs") => {}
            _ => continue,
        }
        match path.file_name().and_then(|n| n.to_str()) {
            Some("main.rs") | Some("lib.rs") => continue,
            _ => {}
        }

        let content = fs::read_to_string(&path).expect("Failed to read file");
        let module = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        parse_file(&content, &module, &mut types);
    }

    types
}

fn parse_file(content: &str, module: &str, types: &mut BTreeMap<String, DomainType>) {
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        // pub struct Name {
        if let Some(name) = parse_pub_struct(trimmed) {
            let mut fields = Vec::new();
            i += 1;
            let mut brace_depth = 1;
            while i < lines.len() && brace_depth > 0 {
                let l = lines[i];
                for c in l.chars() {
                    if c == '{' {
                        brace_depth += 1;
                    } else if c == '}' {
                        brace_depth -= 1;
                    }
                }
                if brace_depth > 0 {
                    if let Some(field) = parse_field(l.trim()) {
                        fields.push(field);
                    }
                }
                i += 1;
            }
            // Collect derive traits
            let mut implements = Vec::new();
            if i >= 2 {
                let above = lines[i.saturating_sub(2)..i.saturating_sub(1)].to_vec();
                for al in &above {
                    if al.contains("#[derive(") {
                        let inner = al.trim();
                        if let Some(start) = inner.find("derive(") {
                            let rest = &inner[start + 7..];
                            if let Some(end) = rest.find(')') {
                                for token in rest[..end].split(',') {
                                    let token = token.trim();
                                    if !token.is_empty() && token != "Debug" && token != "Clone" && token != "PartialEq" {
                                        implements.push(token.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
            types.insert(
                name.clone(),
                DomainType {
                    name,
                    kind: TypeKind::Struct,
                    module: module.to_string(),
                    fields,
                    variants: Vec::new(),
                    implements,
                },
            );
            continue;
        }

        // pub enum Name {
        if let Some(name) = parse_pub_enum(trimmed) {
            let mut variants = Vec::new();
            i += 1;
            let mut brace_depth = 1;
            while i < lines.len() && brace_depth > 0 {
                let l = lines[i];
                for c in l.chars() {
                    if c == '{' {
                        brace_depth += 1;
                    } else if c == '}' {
                        brace_depth -= 1;
                    }
                }
                if brace_depth > 0 {
                    let v = l.trim().trim_end_matches(',');
                    if !v.is_empty() && !v.starts_with('#') && !v.starts_with("//") {
                        variants.push(v.to_string());
                    }
                }
                i += 1;
            }
            let mut implements = Vec::new();
            if i >= 2 {
                let above = lines[i.saturating_sub(2)..i.saturating_sub(1)].to_vec();
                for al in &above {
                    if al.contains("#[derive(") {
                        let inner = al.trim();
                        if let Some(start) = inner.find("derive(") {
                            let rest = &inner[start + 7..];
                            if let Some(end) = rest.find(')') {
                                for token in rest[..end].split(',') {
                                    let token = token.trim();
                                    if !token.is_empty() && token != "Debug" && token != "Clone" && token != "PartialEq" {
                                        implements.push(token.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
            types.insert(
                name.clone(),
                DomainType {
                    name,
                    kind: TypeKind::Enum,
                    module: module.to_string(),
                    fields: Vec::new(),
                    variants,
                    implements,
                },
            );
            continue;
        }

        // pub trait Name
        if let Some(name) = parse_pub_trait(trimmed) {
            let mut fields = Vec::new();
            i += 1;
            let mut brace_depth = 0;
            let mut found_open = false;
            while i < lines.len() {
                let l = lines[i];
                for c in l.chars() {
                    if c == '{' {
                        brace_depth += 1;
                        found_open = true;
                    } else if c == '}' {
                        brace_depth -= 1;
                    }
                }
                if found_open && brace_depth > 0 {
                    // Parse method signatures: fn name(...) -> Type;
                    if let Some(m) = l.trim().find("fn ") {
                        let rest = &l.trim()[m + 3..];
                        if let Some(paren_end) = rest.find(')') {
                            let method_name = rest[..paren_end].trim().to_string();
                            let after_paren = rest[paren_end + 1..].trim();
                            let ret = if let Some(arrow) = after_paren.find("->") {
                                simplify_type(after_paren[arrow + 2..].trim().trim_end_matches(';'))
                            } else {
                                "()".to_string()
                            };
                            fields.push(Field {
                                name: method_name,
                                ty: ret,
                            });
                        }
                    }
                }
                if found_open && brace_depth <= 0 {
                    break;
                }
                i += 1;
            }
            types.insert(
                name.clone(),
                DomainType {
                    name,
                    kind: TypeKind::Trait,
                    module: module.to_string(),
                    fields,
                    variants: Vec::new(),
                    implements: Vec::new(),
                },
            );
            continue;
        }

        // impl TraitName for StructName
        if let Some((trait_name, impl_name)) = parse_impl_for(trimmed) {
            if let Some(t) = types.get_mut(&impl_name) {
                t.implements.push(trait_name);
            }
        }

        i += 1;
    }
}

fn parse_pub_struct(line: &str) -> Option<String> {
    if !line.starts_with("pub struct ") {
        return None;
    }
    let rest = &line[11..];
    let name = rest.split(|c| c == ' ' || c == '{').next()?.trim().to_string();
    if name.is_empty() {
        return None;
    }
    Some(name)
}

fn parse_pub_enum(line: &str) -> Option<String> {
    if !line.starts_with("pub enum ") {
        return None;
    }
    let rest = &line[9..];
    let name = rest.split(|c| c == ' ' || c == '{').next()?.trim().to_string();
    if name.is_empty() {
        return None;
    }
    Some(name)
}

fn parse_pub_trait(line: &str) -> Option<String> {
    if !line.starts_with("pub trait ") {
        return None;
    }
    let rest = &line[10..];
    let name = rest.split(|c| c == ' ' || c == '{').next()?.trim().to_string();
    if name.is_empty() {
        return None;
    }
    Some(name)
}

fn parse_impl_for(line: &str) -> Option<(String, String)> {
    if !line.starts_with("impl ") {
        return None;
    }
    let rest = &line[5..];
    if !rest.contains(" for ") {
        return None;
    }
    let parts: Vec<&str> = rest.splitn(2, " for ").collect();
    if parts.len() != 2 {
        return None;
    }
    let trait_name = parts[0].trim().to_string();
    let struct_name = parts[1].trim().split(|c| c == ' ' || c == '{').next()?.trim().to_string();
    if trait_name.is_empty() || struct_name.is_empty() {
        return None;
    }
    Some((trait_name, struct_name))
}

fn parse_field(line: &str) -> Option<Field> {
    let line = line.trim().trim_end_matches(',');
    if line.is_empty() || line.starts_with('#') || line.starts_with("//") || line.starts_with("fn ") {
        return None;
    }
    // Strip optional "pub "
    let line = line.strip_prefix("pub ").unwrap_or(line);
    // Must have a colon
    let colon_pos = line.find(':')?;
    let name = line[..colon_pos].trim().to_string();
    let ty = line[colon_pos + 1..].trim().to_string();
    if name.is_empty() || ty.is_empty() || name.contains('(') {
        return None;
    }
    Some(Field {
        name,
        ty: simplify_type(&ty),
    })
}

fn simplify_type(ty: &str) -> String {
    let ty = ty.trim().trim_end_matches(';');
    if ty.starts_with("Vec<") {
        return format!("Vec<{}>", simplify_type(&ty[4..ty.len().saturating_sub(1)]));
    }
    if ty.starts_with("Option<") {
        return format!("Option<{}>", simplify_type(&ty[7..ty.len().saturating_sub(1)]));
    }
    if ty.starts_with("Box<dyn ") {
        return format!("Box<dyn {}>", &ty[8..ty.len().saturating_sub(1)]);
    }
    if ty.starts_with("HashMap<") {
        return "HashMap".to_string();
    }
    if ty.starts_with("Mutex<") {
        return format!("Mutex<{}>", simplify_type(&ty[6..ty.len().saturating_sub(1)]));
    }
    if ty.contains("::") {
        return ty.split("::").last().unwrap_or(ty).to_string();
    }
    ty.to_string()
}

fn extract_type_name(ty: &str) -> String {
    let ty = ty.trim();
    if ty.starts_with("Vec<") || ty.starts_with("Option<") {
        return extract_type_name(&ty[1..ty.len().saturating_sub(1)]);
    }
    if ty.starts_with("Box<dyn ") {
        return ty[8..ty.len().saturating_sub(1)].to_string();
    }
    if ty.starts_with("Mutex<") {
        return extract_type_name(&ty[6..ty.len().saturating_sub(1)]);
    }
    if ty.contains("::") {
        return ty.split("::").last().unwrap_or(ty).to_string();
    }
    ty.to_string()
}

fn extract_relationships(
    types: &BTreeMap<String, DomainType>,
) -> BTreeMap<String, BTreeSet<(String, String)>> {
    let mut rels: BTreeMap<String, BTreeSet<(String, String)>> = BTreeMap::new();

    for (name, ty) in types {
        if ty.kind != TypeKind::Struct {
            continue;
        }
        for field in &ty.fields {
            let target = extract_type_name(&field.ty);
            if types.contains_key(&target) {
                rels
                    .entry(name.clone())
                    .or_default()
                    .insert((target.clone(), field.name.clone()));
            }
            if field.ty.starts_with("Box<dyn ") {
                let trait_name = &field.ty[8..field.ty.len().saturating_sub(1)];
                if types.contains_key(trait_name) {
                    rels
                        .entry(name.clone())
                        .or_default()
                        .insert((trait_name.to_string(), field.name.clone()));
                }
            }
        }

        for impl_trait in &ty.implements {
            if types.contains_key(impl_trait) {
                rels
                    .entry(name.clone())
                    .or_default()
                    .insert((impl_trait.clone(), "implements".to_string()));
            }
        }
    }

    rels
}

fn generate_mermaid(
    types: &BTreeMap<String, DomainType>,
    _rels: &BTreeMap<String, BTreeSet<(String, String)>>,
) -> String {
    let mut out = String::new();
    out.push_str("%% Domain Model — local-agent\n");
    out.push_str("classDiagram\n\n");

    for (_name, ty) in types {
        match &ty.kind {
            TypeKind::Struct => {
                out.push_str(&format!("  class {} {{\n", ty.name));
                for field in &ty.fields {
                    out.push_str(&format!("    {} : {}\n", field.ty, field.name));
                }
                out.push_str("  }\n\n");
            }
            TypeKind::Enum => {
                out.push_str(&format!("  class {} {{\n", ty.name));
                for variant in &ty.variants {
                    out.push_str(&format!("    <<{}>>\n", variant));
                }
                out.push_str("  }\n\n");
            }
            TypeKind::Trait => {
                out.push_str(&format!("  class {} {{\n", ty.name));
                out.push_str("    <<trait>>\n");
                for field in &ty.fields {
                    out.push_str(&format!("    {} {}\n", field.name, field.ty));
                }
                out.push_str("  }\n\n");
            }
        }
    }

    for (name, ty) in types {
        if ty.kind != TypeKind::Struct {
            continue;
        }
        for field in &ty.fields {
            let target = extract_type_name(&field.ty);
            if types.contains_key(&target) {
                let arrow = if field.ty.starts_with("Box<dyn") {
                    "..|>"
                } else if field.ty.starts_with("Option<") {
                    "..>"
                } else {
                    "-->"
                };
                out.push_str(&format!(
                    "  {} {} \"{}\" : {}\n",
                    name, arrow, field.name, target
                ));
            }
        }

        for impl_trait in &ty.implements {
            if types.contains_key(impl_trait) {
                out.push_str(&format!("  {} ..|> {}\n", name, impl_trait));
            }
        }
    }

    out
}

fn generate_ascii(
    types: &BTreeMap<String, DomainType>,
    rels: &BTreeMap<String, BTreeSet<(String, String)>>,
) -> String {
    let mut out = String::new();
    out.push_str("=================================================================\n");
    out.push_str("  Domain Model — local-agent\n");
    out.push_str("=================================================================\n\n");

    for (_name, ty) in types {
        let kind_label = match &ty.kind {
            TypeKind::Struct => "struct",
            TypeKind::Enum => "enum",
            TypeKind::Trait => "trait",
        };

        let header = format!("{} ({})", ty.name, kind_label);
        let module_line = format!("module: {}", ty.module);
        let max_inner = header.len().max(module_line.len());

        let mut inner_lines: Vec<String> = Vec::new();
        inner_lines.push(header);
        inner_lines.push(module_line);

        if ty.kind == TypeKind::Enum {
            for variant in &ty.variants {
                inner_lines.push(format!("  [variant] {}", variant));
            }
        }

        if ty.kind == TypeKind::Struct {
            for field in &ty.fields {
                inner_lines.push(format!("  {} : {}", field.name, field.ty));
            }
        }

        if ty.kind == TypeKind::Trait {
            for field in &ty.fields {
                inner_lines.push(format!("  {}() -> {}", field.name, field.ty));
            }
        }

        if !ty.implements.is_empty() {
            inner_lines.push(format!("  implements: {}", ty.implements.join(", ")));
        }

        let max_w = inner_lines.iter().map(|s| s.len()).max().unwrap_or(0);
        let box_w = max_w + 2;
        let hline = format!("+{}+\n", "-".repeat(box_w));

        out.push_str(&hline);
        for inner in &inner_lines {
            let padding = box_w - inner.len();
            out.push_str(&format!("| {}{} |\n", inner, " ".repeat(padding)));
        }
        out.push_str(&hline);

        if let Some(relationships) = rels.get(&ty.name) {
            for (target, field_name) in relationships {
                let rel_label = if field_name == "implements" {
                    "implements"
                } else {
                    "has"
                };
                out.push_str(&format!(
                    "  |--{}--> {} (via .{})\n",
                    rel_label, target, field_name
                ));
            }
        }
        out.push('\n');
    }

    out
}
