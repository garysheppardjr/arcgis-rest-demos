use std::error::Error;
use std::io;
use std::result::Result;

use json::JsonValue;
use rpassword;
use uuid::Uuid;

type BoxResult<T> = Result<T,Box<dyn Error>>;

#[tokio::main]
async fn main() {
    let referrer = format!("Referrer {}", Uuid::new_v4());
    println!("ArcGIS Login Demo");
    println!("ArcGIS Online username:");
    let mut username = String::new();
    io::stdin()
        .read_line(&mut username)
        .expect("Failed to read line");
    username = String::from(username.trim());
    let password = rpassword::read_password_from_tty(Some("Password: ")).unwrap();

    let reqwest_client = reqwest::Client::new();
    let login_result: BoxResult<JsonValue> = quarenta::login(&reqwest_client, &username, &password, &referrer).await;
    match login_result {
        Ok(token_response) => {
            match token_response["token"].as_str() {
                Some(token) => {
                    println!("Token: {}...", &token[..20]);
                    println!("Expiry: {}", token_response["expires"]);
                    println!("SSL only: {}", token_response["ssl"]);
                },
                None => {
                    println!("Login issue: {}", token_response);
                }
            }
        },
        Err(err) => {
            println!("Could not login: {:?}", err);
        },
    }

    println!("Type Enter to exit");
    let mut exit = String::new();
    io::stdin()
        .read_line(&mut exit)
        .expect("Failed to read line");
}
