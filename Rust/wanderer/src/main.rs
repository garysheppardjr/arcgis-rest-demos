use std::collections::HashMap;
use std::io;

extern crate rpassword;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

async fn login(username: String, password: String) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    // TODO do the actual login using client.post, instead of this demo
    let resp = reqwest::get("https://httpbin.org/ip")
        .await?
        .json::<HashMap<String, String>>()
        .await?;
    println!("{:#?}", resp);
    Ok(())
}

#[tokio::main]
async fn main() {
    println!("Wanderer {}", VERSION);
    println!("ArcGIS Online username:");
    let mut username = String::new();
    io::stdin().read_line(&mut username)
        .expect("Failed to read line");
    username = username.trim().to_string();
    let password = rpassword::read_password_from_tty(Some("Password: ")).unwrap();
    println!("Logging in as {}...", username);
    
    let foo = login(username, password);
    println!("Looks like we made it!");
}
