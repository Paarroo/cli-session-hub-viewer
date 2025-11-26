pub mod model;
pub mod scaffold;
pub mod controller;
pub mod auth;
pub mod component;
pub mod page;

use std::fs;
use std::path::Path;

/// Helper function to create directories if they don't exist
pub fn ensure_dir_exists(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

/// Helper function to write file with directory creation
pub fn write_file(path: &Path, content: &str) -> Result<(), Box<dyn std::error::Error>> {
    ensure_dir_exists(path)?;
    fs::write(path, content)?;
    println!("  ✅ Created {}", path.display());
    Ok(())
}

/// Convert string to snake_case
pub fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut prev_is_upper = false;

    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 && !prev_is_upper {
                result.push('_');
            }
            result.push(ch.to_lowercase().next().unwrap());
            prev_is_upper = true;
        } else {
            result.push(ch);
            prev_is_upper = false;
        }
    }
    result
}

/// Convert string to PascalCase
pub fn to_pascal_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;

    for ch in s.chars() {
        if ch == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(ch.to_uppercase().next().unwrap());
            capitalize_next = false;
        } else {
            result.push(ch);
        }
    }
    result
}

/// Parse field definition like "name:string:unique" into (name, type, options)
pub fn parse_field(field: &str) -> (String, String, Vec<String>) {
    let parts: Vec<&str> = field.split(':').collect();
    let name = parts.first().unwrap_or(&"").to_string();
    let field_type = parts.get(1).unwrap_or(&"string").to_string();
    let options = parts.iter().skip(2).map(|s| s.to_string()).collect();

    (name, field_type, options)
}

/// Map Rust types to SeaORM column types
pub fn map_field_type_to_sea_orm(field_type: &str) -> &'static str {
    match field_type {
        "string" | "text" => "string()",
        "integer" | "int" => "integer()",
        "boolean" | "bool" => "boolean()",
        "float" | "f32" => "float()",
        "double" | "f64" => "double()",
        "decimal" => "decimal()",
        "date" => "date()",
        "datetime" | "timestamp" => "timestamp()",
        "uuid" => "uuid()",
        "json" => "json()",
        _ => "string()", // default fallback
    }
}

/// Map Rust types to actual Rust types for the model
pub fn map_field_type_to_rust(field_type: &str) -> &'static str {
    match field_type {
        "string" | "text" => "String",
        "integer" | "int" => "i32",
        "boolean" | "bool" => "bool",
        "float" => "f32",
        "double" | "f64" => "f64",
        "decimal" => "Decimal",
        "date" => "Date",
        "datetime" | "timestamp" => "DateTime",
        "uuid" => "Uuid",
        "json" => "Json",
        _ => "String",
    }
}