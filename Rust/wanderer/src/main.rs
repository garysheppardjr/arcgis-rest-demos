use reqwest::RequestBuilder;
use reqwest::Response;
use reqwest::Result;
use serde::Deserialize;
use std::collections::HashMap;
use std::io;
use uuid::Uuid;

extern crate json;
extern crate rpassword;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const FEATURE_LAYER_URL: &'static str = "https://services7.arcgis.com/iYTqAIgyDcVSpgzf/arcgis/rest/services/World_Cities/FeatureServer/0";

#[derive(Deserialize)]
struct TokenResponse {
    token: String,
    expires: i64,
    ssl: bool,
}

#[derive(Deserialize)]
struct Service {
    url: String,
}

#[derive(Deserialize)]
struct HelperServices {
    analysis: Service,
}

#[derive(Deserialize)]
struct PortalSelfResponse {
    #[serde(rename = "helperServices")]
    helper_services: HelperServices,
}

async fn login(client: &reqwest::Client, username: &String, password: &String, referrer: &String) -> Result<Response> {
    let mut params = HashMap::new();
    let f_json = String::from("json");
    params.insert("username", username);
    params.insert("password", password);
    params.insert("referer", referrer);
    params.insert("f", &f_json);

    client.post("https://www.arcgis.com/sharing/rest/generateToken")
        .form(&params)
        .send()
        .await
}

async fn get_portal_self(client: &reqwest::Client, token: &String, referrer: &String) -> Result<PortalSelfResponse> {
    let f_json = String::from("json");
    let mut result: Result<Response> = client.get("https://www.arcgis.com/sharing/rest/portals/self")
        .query(&[
            ("token", token),
            ("referer", referrer),
            ("f", &f_json),
        ])
        .send()
        .await;
    match result {
        Ok(response) => {
            response.json().await
        },
        Err(err) => {
            println!("Couldn't get portal self: {:?}", err);
            Err(err)
        },
    }
}

async fn get_cities_count(client: &reqwest::Client, token: &String, referrer: &String) -> u32 {
    let f_json = String::from("json");
    let mut result: Result<Response> = client.get(format!("{}/query", FEATURE_LAYER_URL).as_str())
        .query(&[
            ("where", "population IS NOT NULL"),
            ("returnCountOnly", "true"),
            ("token", token),
            ("referer", referrer),
            ("f", &f_json),
        ])
        .send()
        .await;
    match result {
        Ok(response) => {
            let response_result: Result<String> = response.text().await;
            match response_result {
                Ok(response_string) => {
                    match json::parse(response_string.as_str())
                        .unwrap()["count"].as_u32() {
                        Some(count) => count,
                        None => {
                            println!("Count is null (this should never happen)");
                            0
                        },
                    }
                },
                Err(err) => {
                    println!("Couldn't parse response: {:?}", err);
                    0
                }
            }
        },
        Err(err) => {
            println!("Couldn't get portal self: {:?}", err);
            0
        },
    }
}

async fn get_minimum_population(client: &reqwest::Client, token: &String, referrer: &String, city_count: u32) -> u32 {
    let f_json = String::from("json");
    let mut result: Result<Response> = client.get(format!("{}/query", FEATURE_LAYER_URL).as_str())
        .query(&[
            ("outFields", "population"),
            ("where", "population IS NOT NULL"),
            ("orderByFields", "population DESC"),
            ("resultOffset", (city_count - 1).to_string().as_str()),
            ("resultRecordCount", "1"),
            ("returnGeometry", "false"),
            ("token", token),
            ("referer", referrer),
            ("f", &f_json),
        ])
        .send()
        .await;
    match result {
        Ok(response) => {
            let response_result: Result<String> = response.text().await;
            match response_result {
                Ok(response_string) => {
                    match json::parse(response_string.as_str())
                        .unwrap()["features"][0]["attributes"]["population"].as_u32() {
                        Some(population) => population,
                        None => {
                            println!("Population is null (this should never happen)");
                            0
                        },
                    }
                },
                Err(err) => {
                    println!("Couldn't parse response: {:?}", err);
                    0
                }
            }
        },
        Err(err) => {
            println!("Couldn't get portal self: {:?}", err);
            0
        },
    }
}

async fn play_game(client: &reqwest::Client, token: &String, referrer: &String, city_count: u32) {
    println!("Let's play Wanderer with {} cities", city_count);
    
    // Get the minimum population for cities in this game
    let minimum_population = get_minimum_population(client, token, referrer, city_count).await;
    println!("Minimum population: {}", minimum_population);
    
    // We need the analysis URL
    let self_response: Result<PortalSelfResponse> = get_portal_self(client, token, referrer).await;
    match self_response {
        Ok(portal_self) => {
            println!("Analysis URL is {}", portal_self.helper_services.analysis.url);
        },
        Err(err) => println!("Error response for self: {:?}", err),
    }
}

#[tokio::main]
async fn main() {
    let referrer = format!("Referrer {}", Uuid::new_v4());
    println!("Wanderer {}", VERSION);
    println!("ArcGIS Online username:");
    let mut username = String::new();
    io::stdin().read_line(&mut username)
        .expect("Failed to read line");
    username = String::from(username.trim());
    let password = rpassword::read_password_from_tty(Some("Password: ")).unwrap();

    let reqwest_client = reqwest::Client::new();
    let login_result = login(&reqwest_client, &username, &password, &referrer).await;
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
                        3 => {
                            get_cities_count(&reqwest_client, &json.token, &referrer).await
                        },
                        _ => {
                            println!("Okay, then you get the default of 0 = easy.");
                            10
                        },
                    };
                    play_game(&reqwest_client, &json.token, &referrer, city_count).await;
                },
                Err(err) => println!("error here: {:?}", err),
            }
        },
        Err(err) => println!("error parsing response: {:?}", err),
    }
}
