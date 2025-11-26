use super::*;

pub fn generate_scaffold(
    name: &str,
    fields: &[String],
    skip: &[String],
    _force: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🏗️  Generating scaffold: {}", name);

    // Generate model
    if !skip.contains(&"model".to_string()) {
        println!("  📝 Generating model...");
        model::generate_model(name, fields, true, false, false)?;
    }



    // Generate API routes
    if !skip.contains(&"api".to_string()) {
        println!("  🌐 Generating API routes...");
        controller::generate_controller(&format!("api/v1/{}", to_snake_case(&format!("{}s", name))), &[], true)?;
    }

    // Generate server functions
    if !skip.contains(&"server_fns".to_string()) {
        println!("  ⚡ Generating server functions...");
        // TODO: Implement server functions generator
    }

    // Generate UI components
    if !skip.contains(&"ui".to_string()) {
        println!("  🎨 Generating UI components...");
        component::generate_component(&format!("{}Form", name), &[])?;
        component::generate_component(&format!("{}List", name), &[])?;
    }

    // Generate pages
    if !skip.contains(&"pages".to_string()) {
        println!("  📄 Generating pages...");
        page::generate_page(&format!("{}/index", to_snake_case(&format!("{}s", name))), false, false)?;
        page::generate_page(&format!("{}/show/:id", to_snake_case(&format!("{}s", name))), false, false)?;
        page::generate_page(&format!("{}/new", to_snake_case(&format!("{}s", name))), false, false)?;
        page::generate_page(&format!("{}/edit/:id", to_snake_case(&format!("{}s", name))), false, false)?;
    }

    println!("🎉 Scaffold {} generated successfully!", name);
    Ok(())
}