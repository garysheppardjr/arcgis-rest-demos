use std::collections::HashMap;
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
    let login_result: BoxResult<JsonValue> = login(&reqwest_client, &username, &password, &referrer).await;
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

async fn login(
    client: &reqwest::Client,
    username: &String,
    password: &String,
    referrer: &String,
) -> BoxResult<JsonValue> {
    let mut params = HashMap::new();
    let f_json = String::from("json");
    params.insert("username", username);
    params.insert("password", password);
    params.insert("referer", referrer);
    params.insert("f", &f_json);

    match client.post("https://www.arcgis.com/sharing/rest/generateToken").form(&params).send().await {
        Ok(response) => {
            match response.text().await {
                Ok(text) => {
                    Ok(json::parse(text.as_str()).unwrap())
                },
                Err(err) => {
                    Err(Box::new(err))
                }
            }
        },
        Err(err) => {
            Err(Box::new(err))
        },
    }
}
