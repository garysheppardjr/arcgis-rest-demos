use reqwest::RequestBuilder;
use reqwest::Response;
use reqwest::Result;
use std::collections::HashMap;
use std::io;

extern crate rpassword;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

async fn login(username: String, password: String) -> Result<Response> {
    let mut params = HashMap::new();
    params.insert("username", username);
    params.insert("password", password);
    params.insert("referer", "Wanderer app".to_string());
    params.insert("f", "json".to_string());

    let client = reqwest::Client::new();
    client.post("https://www.arcgis.com/sharing/rest/generateToken")
        .form(&params)
        .send()
        .await
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
    let login_result = login(username, password).await;
    match login_result {
        Ok(response) => {
            println!("got a response");
            let text_result: Result<String> = response.text().await;
            match text_result {
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
