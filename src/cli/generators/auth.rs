use super::*;

pub fn generate_auth(model_name: &str, method: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔐 Generating authentication system for model: {}", model_name);

    match method {
        "jwt" => generate_jwt_auth(model_name),
        "session" => generate_session_auth(model_name),
        _ => {
            println!("❌ Unknown auth method: {}. Use 'jwt' or 'session'", method);
            Ok(())
        }
    }
}

fn generate_jwt_auth(model_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Generate auth middleware
    let middleware_content = generate_jwt_middleware()?;
    let middleware_path = Path::new("src").join("infrastructure").join("auth.rs");
    write_file(&middleware_path, &middleware_content)?;

    // Generate login component
    component::generate_component("LoginForm", &["email:String".to_string(), "password:String".to_string()])?;

    // Generate register component
    component::generate_component("RegisterForm", &["email:String".to_string(), "password:String".to_string(), "password_confirmation:String".to_string()])?;

    // Generate auth server functions
    let server_fn_content = generate_auth_server_fns(model_name)?;
    let server_fn_path = Path::new("src").join("shared").join("server_fns").join("auth.rs");
    write_file(&server_fn_path, &server_fn_content)?;

    // Update server_fns mod
    update_server_fns_mod("auth.rs")?;

    println!("🎉 JWT Authentication system generated!");
    println!("   Don't forget to set JWT_SECRET environment variable.");
    Ok(())
}

fn generate_session_auth(_model_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("📋 Session-based authentication not yet implemented.");
    println!("   Use 'jwt' method for now.");
    Ok(())
}

fn generate_jwt_middleware() -> Result<String, Box<dyn std::error::Error>> {
    let content = r#"use axum::{
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: i32,  // user id
    pub exp: usize,
    pub admin: bool,
}

pub async fn auth_middleware(mut req: Request, next: Next) -> Result<Response, StatusCode> {
    let auth_header = req.headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "));

    let token = match auth_header {
        Some(token) => token,
        None => return Err(StatusCode::UNAUTHORIZED),
    };

    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "your-secret-key".to_string());
    let decoding_key = DecodingKey::from_secret(secret.as_bytes());

    match decode::<Claims>(token, &decoding_key, &Validation::default()) {
        Ok(token_data) => {
            // Store user info in request extensions
            req.extensions_mut().insert(token_data.claims);
            Ok(next.run(req).await)
        }
        Err(_) => Err(StatusCode::UNAUTHORIZED),
    }
}
"#;

    Ok(content.to_string())
}

fn generate_auth_server_fns(model_name: &str) -> Result<String, Box<dyn std::error::Error>> {
    let content = format!(r#"use dioxus::prelude::*;
use serde::{{Deserialize, Serialize}};
use jsonwebtoken::{{encode, EncodingKey, Header}};
use bcrypt::{{hash, verify, DEFAULT_COST}};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LoginRequest {{
    pub email: String,
    pub password: String,
}}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RegisterRequest {{
    pub email: String,
    pub password: String,
    pub password_confirmation: String,
}}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AuthResponse {{
    pub token: String,
    pub user: User,
}}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct User {{
    pub id: i32,
    pub email: String,
    pub admin: bool,
}}

#[server]
pub async fn login(req: LoginRequest) -> Result<AuthResponse, ServerFnError> {{
    // TODO: Implement login logic with {} model
    // - Find user by email
    // - Verify password
    // - Generate JWT token

    Err(ServerFnError::ServerError("Login not implemented yet".to_string()))
}}

#[server]
pub async fn register(req: RegisterRequest) -> Result<AuthResponse, ServerFnError> {{
    // TODO: Implement registration logic with {} model
    // - Validate passwords match
    // - Hash password
    // - Create user
    // - Generate JWT token

    Err(ServerFnError::ServerError("Registration not implemented yet".to_string()))
}}

#[server]
pub async fn logout() -> Result<(), ServerFnError> {{
    // TODO: Implement logout logic
    // - Invalidate token/session

    Ok(())
}}
"#, model_name, model_name);

    Ok(content)
}

fn update_server_fns_mod(file_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mod_path = Path::new("src").join("shared").join("server_fns").join("mod.rs");
    let mut content = fs::read_to_string(&mod_path).unwrap_or_else(|_| "".to_string());

    let mod_name = file_name.trim_end_matches(".rs");

    if !content.contains(&format!("pub mod {};", mod_name)) {
        content.push_str(&format!("pub mod {};\n", mod_name));
        write_file(&mod_path, &content)?;
    }

    Ok(())
}