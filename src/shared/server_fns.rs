// Server functions are disabled for now
// TODO: Re-enable when Dioxus server functions are properly configured

pub async fn get_users() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    // For demo, return hardcoded users
    Ok(vec!["user1@example.com".to_string(), "user2@example.com".to_string()])
}