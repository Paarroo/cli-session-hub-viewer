use super::*;
use std::path::Path;

pub fn generate_model(
    name: &str,
    fields: &[String],
    timestamps: bool,
    uuid: bool,
    soft_delete: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let model_name = to_pascal_case(name);
    let table_name = to_snake_case(&format!("{}s", name));
    let file_name = format!("{}.rs", to_snake_case(name));

    println!("🏗️  Generating model: {}", model_name);

    // Generate the model file
    let model_content = generate_model_content(&model_name, &table_name, fields, timestamps, uuid, soft_delete)?;
    let model_path = Path::new("src").join("domain").join("models").join(&file_name);
    write_file(&model_path, &model_content)?;

    // Update the mod.rs file to include the new model
    update_models_mod(&file_name)?;

    println!("🎉 Model {} generated successfully!", model_name);
    Ok(())
}

fn generate_model_content(
    _model_name: &str,
    table_name: &str,
    fields: &[String],
    timestamps: bool,
    uuid: bool,
    soft_delete: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut content = String::new();

    // Imports
    content.push_str("#[cfg(feature = \"server\")]\n");
    content.push_str("use sea_orm::entity::prelude::*;\n");
    content.push_str("#[cfg(feature = \"server\")]\n");
    content.push_str("use chrono::{DateTime, Utc};\n");
    content.push_str("#[cfg(feature = \"server\")]\n");
    content.push_str("use rust_decimal::Decimal;\n");
    content.push_str("#[cfg(feature = \"server\")]\n");
    content.push_str("use uuid::Uuid;\n");
    content.push_str("use serde::{Deserialize, Serialize};\n");
    content.push('\n');

    // Model struct
    content.push_str("#[cfg(feature = \"server\")]\n");
    content.push_str("#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]\n");
    content.push_str(&format!("#[sea_orm(table_name = \"{}\")]\n", table_name));
    content.push_str("pub struct Model {\n");

    // Primary key
    if uuid {
        content.push_str("    #[sea_orm(primary_key)]\n");
        content.push_str("    pub id: Uuid,\n");
    } else {
        content.push_str("    #[sea_orm(primary_key)]\n");
        content.push_str("    pub id: i32,\n");
    }

    // User-defined fields
    for field in fields {
        let (field_name, field_type, options) = parse_field(field);
        let rust_type = map_field_type_to_rust(&field_type);

        // Add field attributes
        if options.contains(&"unique".to_string()) {
            content.push_str("    #[sea_orm(unique)]\n");
        }

        content.push_str(&format!("    pub {}: {},\n", to_snake_case(&field_name), rust_type));
    }

    // Timestamps
    if timestamps {
        content.push_str("    pub created_at: DateTime<Utc>,\n");
        content.push_str("    pub updated_at: DateTime<Utc>,\n");
    }

    // Soft delete
    if soft_delete {
        content.push_str("    pub deleted_at: Option<DateTime<Utc>>,\n");
    }

    content.push_str("}\n\n");

    // Relations (empty for now)
    content.push_str("#[cfg(feature = \"server\")]\n");
    content.push_str("#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]\n");
    content.push_str("pub enum Relation {}\n\n");

    // ActiveModelBehavior
    content.push_str("#[cfg(feature = \"server\")]\n");
    content.push_str("impl ActiveModelBehavior for ActiveModel {}\n");

    Ok(content)
}

fn update_models_mod(file_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mod_path = Path::new("src").join("domain").join("models").join("mod.rs");
    let mut content = fs::read_to_string(&mod_path).unwrap_or_else(|_| "pub mod user;\n".to_string());

    // Remove .rs extension for the mod declaration
    let mod_name = file_name.trim_end_matches(".rs");

    // Check if already included
    if !content.contains(&format!("pub mod {};", mod_name)) {
        content.push_str(&format!("#[cfg(feature = \"server\")]\npub mod {};\n", mod_name));
        write_file(&mod_path, &content)?;
    }

    Ok(())
}