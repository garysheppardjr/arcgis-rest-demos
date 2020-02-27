use std::collections::HashMap;
use std::error::Error;
use std::io;
use std::result::Result;

#[macro_use]
extern crate json;
use json::JsonValue;
use reqwest::Client;
use rpassword;
use uuid::Uuid;

type BoxResult<T> = Result<T,Box<dyn Error>>;

const PORTAL_ROOT_URL: &str = "https://www.arcgis.com";
// const PORTAL_ROOT_URL: &str = "https://host.domain.com/portal-web-adaptor";

#[tokio::main]
async fn main() {
    println!("Add Item Demo");

    let referrer = format!("Referrer {}", Uuid::new_v4());
    let username = read_from_console("ArcGIS Online username:");
    let password = rpassword::read_password_from_tty(Some("Password: ")).unwrap();

    let reqwest_client = Client::new();
    let login_result: BoxResult<JsonValue> = login(&reqwest_client, &username, &password, &referrer).await;
    match login_result {
        Ok(token_response) => {
            match token_response["token"].as_str() {
                Some(token) => {
                    let mut done = false;
                    while !done {
                        let name = read_from_console("Your name:");
                        let color = read_from_console("Your favorite color:");
                        let mut item_type = read_from_console("Item type: [Color Set]");
                        if item_type.trim().is_empty() {
                            item_type = String::from("Color Set");
                        }
                        let title = read_from_console("Item title:");
                        let add_item_result: BoxResult<JsonValue> = add_item(
                            &reqwest_client, &username, &String::from(token), &referrer,
                            &name, &color, &item_type, &title
                        ).await;
                        match add_item_result {
                            Ok(add_item_response) => {
                                if add_item_response.has_key("error") {
                                    println!("Problem creating item: {}", add_item_response["error"]["message"]);
                                } else if add_item_response["success"].as_bool().unwrap_or(false) {
                                    println!("Success!");
                                    println!("Item page: https://www.arcgis.com/home/item.html?id={}", add_item_response["id"]);
                                    println!("Item JSON: {}/sharing/rest/content/items/{}?f=json&token={}", PORTAL_ROOT_URL, add_item_response["id"], token);
                                    println!("Item data JSON: {}/sharing/rest/content/items/{}/data?f=json&token={}", PORTAL_ROOT_URL, add_item_response["id"], token);
                                } else {
                                    println!("Failed to create item. Cause unknown. ☹");
                                }
                            },
                            Err(err) => {
                                println!("Could not add item: {:?}", err);
                            }
                        }
                        done = "y" != read_from_console("Another? (y or n) [n]");
                    }
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

    read_from_console("Type Enter to exit");
}

async fn add_item(
    client: &reqwest::Client,
    username: &String,
    token: &String,
    referrer: &String,
    name: &String,
    color: &String,
    item_type: &String,
    item_title: &String,
) -> BoxResult<JsonValue> {
    let mut params = HashMap::new();
    let f_json = String::from("json");
    params.insert("token", token);
    params.insert("referer", referrer);
    params.insert("type", item_type);
    params.insert("title", item_title);
    params.insert("f", &f_json);

    let text = json::stringify(object!{
        "name" => name.as_str(),
        "color" => color.as_str(),
    });
    params.insert("text", &text);

    let add_item_url = format!("{}/sharing/rest/content/users/{}/addItem", PORTAL_ROOT_URL, username);
    match client.post(add_item_url.as_str()).form(&params).send().await {
        Ok(response) => {
            match response.text().await {
                Ok(text) => Ok(json::parse(text.as_str()).unwrap()),
                Err(err) => Err(Box::new(err)),
            }
        },
        Err(err) => Err(Box::new(err)),
    }
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

    match client.post(format!("{}/sharing/rest/generateToken", PORTAL_ROOT_URL).as_str()).form(&params).send().await {
        Ok(response) => {
            match response.text().await {
                Ok(text) => Ok(json::parse(text.as_str()).unwrap()),
                Err(err) => Err(Box::new(err)),
            }
        },
        Err(err) => Err(Box::new(err)),
    }
}

fn read_from_console(prompt: &str) -> String {
    println!("{}", prompt);
    let mut value = String::new();
    io::stdin()
        .read_line(&mut value)
        .expect("Failed to read line");
    String::from(value.trim())
}
