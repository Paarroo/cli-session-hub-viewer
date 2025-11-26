use super::*;

pub fn generate_controller(
    name: &str,
    actions: &[String],
    resource: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let controller_name = name.replace("/", "_");
    let file_name = format!("{}.rs", controller_name);

    println!("🎛️  Generating controller: {}", controller_name);

    let actions_to_generate = if resource {
        vec!["index".to_string(), "show".to_string(), "create".to_string(), "update".to_string(), "destroy".to_string()]
    } else if actions.is_empty() {
        vec!["index".to_string()]
    } else {
        actions.to_vec()
    };

    let controller_content = generate_controller_content(&controller_name, &actions_to_generate)?;
    let controller_path = Path::new("src").join("infrastructure").join("api").join(&file_name);
    write_file(&controller_path, &controller_content)?;

    // Update the api mod.rs
    update_api_mod(&file_name)?;

    println!("🎉 Controller {} generated successfully!", controller_name);
    Ok(())
}

fn generate_controller_content(
    _controller_name: &str,
    actions: &[String],
) -> Result<String, Box<dyn std::error::Error>> {
    let mut content = String::new();

    // Imports
    content.push_str("use axum::{extract::Path, http::StatusCode, Json};\n");
    content.push_str("use sea_orm::{ActiveModelTrait, EntityTrait};\n");
    content.push_str("use serde::{Deserialize, Serialize};\n");
    content.push('\n');

    // Generate action functions
    for action in actions {
        match action.as_str() {
            "index" => {
                content.push_str("#[axum::debug_handler]\n");
                content.push_str("pub async fn index() -> Result<Json<Vec<serde_json::Value>>, StatusCode> {\n");
                content.push_str("    // TODO: Implement index action\n");
                content.push_str("    Ok(Json(vec![]))\n");
                content.push_str("}\n\n");
            }
            "show" => {
                content.push_str("#[axum::debug_handler]\n");
                content.push_str("pub async fn show(Path(id): Path<i32>) -> Result<Json<serde_json::Value>, StatusCode> {\n");
                content.push_str("    // TODO: Implement show action\n");
                content.push_str("    Ok(Json(serde_json::json!({})))\n");
                content.push_str("}\n\n");
            }
            "create" => {
                content.push_str("#[derive(Deserialize)]\n");
                content.push_str("pub struct CreateRequest {\n");
                content.push_str("    // TODO: Add fields\n");
                content.push_str("}\n\n");
                content.push_str("#[axum::debug_handler]\n");
                content.push_str("pub async fn create(Json(_req): Json<CreateRequest>) -> Result<Json<serde_json::Value>, StatusCode> {\n");
                content.push_str("    // TODO: Implement create action\n");
                content.push_str("    Ok(Json(serde_json::json!({})))\n");
                content.push_str("}\n\n");
            }
            "update" => {
                content.push_str("#[derive(Deserialize)]\n");
                content.push_str("pub struct UpdateRequest {\n");
                content.push_str("    // TODO: Add fields\n");
                content.push_str("}\n\n");
                content.push_str("#[axum::debug_handler]\n");
                content.push_str("pub async fn update(Path(id): Path<i32>, Json(_req): Json<UpdateRequest>) -> Result<Json<serde_json::Value>, StatusCode> {\n");
                content.push_str("    // TODO: Implement update action\n");
                content.push_str("    Ok(Json(serde_json::json!({})))\n");
                content.push_str("}\n\n");
            }
            "destroy" => {
                content.push_str("#[axum::debug_handler]\n");
                content.push_str("pub async fn destroy(Path(id): Path<i32>) -> Result<StatusCode, StatusCode> {\n");
                content.push_str("    // TODO: Implement destroy action\n");
                content.push_str("    Ok(StatusCode::NO_CONTENT)\n");
                content.push_str("}\n\n");
            }
            _ => {}
        }
    }

    Ok(content)
}

fn update_api_mod(file_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mod_path = Path::new("src").join("infrastructure").join("api").join("mod.rs");
    let mut content = fs::read_to_string(&mod_path).unwrap_or_else(|_| "".to_string());

    let mod_name = file_name.trim_end_matches(".rs");

    if !content.contains(&format!("pub mod {};", mod_name)) {
        content.push_str(&format!("pub mod {};\n", mod_name));
        write_file(&mod_path, &content)?;
    }

    Ok(())
}