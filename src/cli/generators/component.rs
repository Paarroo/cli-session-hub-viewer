use super::*;

pub fn generate_component(
    name: &str,
    props: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let component_name = to_pascal_case(name);
    let file_name = format!("{}.rs", to_snake_case(name));

    println!("🧩 Generating component: {}", component_name);

    let component_content = generate_component_content(&component_name, props)?;
    let component_path = Path::new("src").join("app").join("components").join(&file_name);
    write_file(&component_path, &component_content)?;

    // Update components mod.rs
    update_components_mod(&file_name)?;

    println!("🎉 Component {} generated successfully!", component_name);
    Ok(())
}

fn generate_component_content(
    component_name: &str,
    props: &[String],
) -> Result<String, Box<dyn std::error::Error>> {
    let mut content = String::new();

    content.push_str("use dioxus::prelude::*;\n");
    content.push('\n');

    // Props struct if any
    if !props.is_empty() {
        content.push_str("#[derive(Props, PartialEq, Clone)]\n");
        content.push_str("pub struct Props {\n");
        for prop in props {
            let (prop_name, prop_type, _) = parse_field(prop);
            content.push_str(&format!("    pub {}: {},\n", to_snake_case(&prop_name), prop_type));
        }
        content.push_str("}\n\n");
    }

    // Component function
    if props.is_empty() {
        content.push_str("#[component]\n");
        content.push_str(&format!("pub fn {}() -> Element {{\n", component_name));
    } else {
        content.push_str("#[component]\n");
        content.push_str(&format!("pub fn {}(props: Props) -> Element {{\n", component_name));
    }

    content.push_str("    rsx! {\n");
    content.push_str("        div {\n");
    content.push_str(&format!("            class: \"{}\",\n", to_snake_case(component_name)));
    content.push_str(&format!("            \"{}\"", component_name));
    content.push_str("        }\n");
    content.push_str("    }\n");
    content.push_str("}\n");

    Ok(content)
}

fn update_components_mod(file_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mod_path = Path::new("src").join("app").join("components").join("mod.rs");
    let mut content = fs::read_to_string(&mod_path).unwrap_or_else(|_| "".to_string());

    let mod_name = file_name.trim_end_matches(".rs");

    if !content.contains(&format!("pub mod {};", mod_name)) {
        content.push_str(&format!("pub mod {};\n", mod_name));
        write_file(&mod_path, &content)?;
    }

    Ok(())
}