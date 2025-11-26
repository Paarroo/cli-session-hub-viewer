use super::*;

pub fn generate_page(
    name: &str,
    auth_required: bool,
    admin_only: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let page_name = to_pascal_case(name);
    let file_name = format!("{}.rs", to_snake_case(name));

    println!("📄 Generating page: {}", page_name);

    let page_content = generate_page_content(&page_name, auth_required, admin_only)?;
    let page_path = Path::new("src").join("app").join("pages").join(&file_name);
    write_file(&page_path, &page_content)?;

    // Update pages mod.rs
    update_pages_mod(&file_name)?;

    println!("🎉 Page {} generated successfully!", page_name);
    Ok(())
}

fn generate_page_content(
    page_name: &str,
    auth_required: bool,
    admin_only: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut content = String::new();

    content.push_str("use dioxus::prelude::*;\n");
    content.push('\n');

    // Add auth guard if needed
    if auth_required || admin_only {
        content.push_str("// TODO: Add authentication guard\n");
        if admin_only {
            content.push_str("// TODO: Add admin-only check\n");
        }
        content.push('\n');
    }

    content.push_str("#[component]\n");
    content.push_str(&format!("pub fn {}() -> Element {{\n", page_name));

    if auth_required || admin_only {
        content.push_str("    // TODO: Check authentication status\n");
        content.push_str("    // if not authenticated { return redirect to login }\n");
        content.push_str("    // if admin_only && !is_admin { return forbidden }\n");
        content.push('\n');
    }

    content.push_str("    rsx! {\n");
    content.push_str("        div {\n");
    content.push_str(&format!("            class: \"page-{}\",\n", to_snake_case(page_name)));
    content.push_str("            h1 {\n");
    content.push_str(&format!("                \"{}\"\n", page_name));
    content.push_str("            }\n");
    content.push_str("            p {\n");
    content.push_str("                \"Page content goes here\"\n");
    content.push_str("            }\n");
    content.push_str("        }\n");
    content.push_str("    }\n");
    content.push_str("}\n");

    Ok(content)
}

fn update_pages_mod(file_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mod_path = Path::new("src").join("app").join("pages").join("mod.rs");
    let mut content = fs::read_to_string(&mod_path).unwrap_or_else(|_| "".to_string());

    let mod_name = file_name.trim_end_matches(".rs");

    if !content.contains(&format!("pub mod {};", mod_name)) {
        content.push_str(&format!("pub mod {};\n", mod_name));
        write_file(&mod_path, &content)?;
    }

    Ok(())
}