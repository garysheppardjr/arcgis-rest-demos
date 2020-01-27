use reqwest::RequestBuilder;
use reqwest::Response;
use reqwest::Result;
use serde::Deserialize;
use std::collections::HashMap;
use std::io;
use uuid::Uuid;

extern crate rpassword;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[derive(Deserialize)]
struct TokenResponse {
    token: String,
    expires: i64,
    ssl: bool,
}

async fn login(username: String, password: String, referrer: String) -> Result<Response> {
    let mut params = HashMap::new();
    params.insert("username", username);
    params.insert("password", password);
    params.insert("referer", referrer);
    params.insert("f", "json".to_string());

    let client = reqwest::Client::new();
    client.post("https://www.arcgis.com/sharing/rest/generateToken")
        .form(&params)
        .send()
        .await
}

fn play_game(token: String, city_count: u32) {
    println!("Let's play Wanderer with up to {} cities and token {}", city_count, token);
}

#[tokio::main]
async fn main() {
    let referrer = format!("Referrer {}", Uuid::new_v4());
    println!("Wanderer {}", VERSION);
    println!("ArcGIS Online username:");
    let mut username = String::new();
    io::stdin().read_line(&mut username)
        .expect("Failed to read line");
    username = username.trim().to_string();
    let password = rpassword::read_password_from_tty(Some("Password: ")).unwrap();

    println!("Logging in as {}...", username);
    let login_result = login(username, password, referrer).await;
    match login_result {
        Ok(response) => {
            let json_result: Result<TokenResponse> = response.json().await;
            match json_result {
                Ok(json) => {
                    println!("Level of difficulty (0 = easy, 1 = medium, 2 = hard, 3 = legendary):");
                    let mut difficulty = String::new();
                    io::stdin().read_line(&mut difficulty)
                        .expect("Failed to read line");
                    let difficulty: u32 = match difficulty.trim().parse() {
                        Ok(num) => num,
                        Err(_) => {
                            println!("Okay, then you get the default of 0 = easy.");
                            0
                        },
                    };
                    let city_count: u32 = match difficulty {
                        0 => 10,
                        1 => 100,
                        2 => 1000,
                        3 => std::u32::MAX,
                        _ => {
                            println!("Okay, then you get the default of 0 = easy.");
                            10
                        },
                    };
                    play_game(json.token, city_count);
                },
                Err(err) => println!("error here: {:?}", err),
            }
        },
        Err(err) => println!("error parsing response: {:?}", err),
    }
}
