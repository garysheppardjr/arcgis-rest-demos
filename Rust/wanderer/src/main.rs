use reqwest::Response;
use reqwest::Result;
use std::collections::HashMap;
use std::io;

extern crate rpassword;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

async fn login(username: String, password: String) -> Result<Response> {
    println!("This is login; username is {}", username);
    let client = reqwest::Client::new();
    // TODO do the actual login using client.post, instead of this demo
    reqwest::get("https://httpbin.org/ip").await
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
    
    let foo = login(username, password).await;
    match foo {
        Ok(response) => {
            println!("got a response");
            match response.text().await {
                Ok(text) => {
                    println!("In JSON, it's this: {}", text);
                },
                Err(err) => println!("error here: {:?}", err),
            }
        },
        Err(err) => println!("error parsing response: {:?}", err),
    }
    println!("Looks like we made it!");
}
