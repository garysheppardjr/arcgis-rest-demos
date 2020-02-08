extern crate geo;
extern crate json;
extern crate rand;
extern crate rpassword;
extern crate strfmt;

use geo::algorithm::bearing::Bearing;
use geo::Point;
use json::object;
use rand::Rng;
use reqwest::header::CACHE_CONTROL;
use reqwest::Response;
use reqwest::Result;
use serde::Deserialize;
use std::collections::HashMap;
use std::collections::HashSet;
use std::io;
use std::{thread, time};
use strfmt::strfmt;
use uuid::Uuid;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const FEATURE_LAYER_URL: &'static str = "https://services7.arcgis.com/iYTqAIgyDcVSpgzf/arcgis/rest/services/World_Cities/FeatureServer/0";
const WELCOME_MESSAGES: &[&str] = &[
    "Though you've just arrived, you look around and immediately realize that you are in {city}.",
    "Something in the air tells you you've just arrived in {city}.",
    "That rustic aroma seems so familiar. \"Ah yes,\" you tell yourself. \"This could only be {city}.\".",
    "The sunsets in {city} are so beautiful this time of year. If only you had time to linger.",
];

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
    geometry: Service,
}

#[derive(Deserialize)]
struct PortalSelf {
    #[serde(rename = "helperServices")]
    helper_services: HelperServices,
}

#[derive(Deserialize)]
struct City {
    city: String,
    lat: f64,
    lng: f64,
    country: String,
    admin_name: String,
    //#[serde(default)]
    population: u32,
    #[serde(alias = "FID", alias = "ORIG_FID")]
    fid: u32,
}

#[derive(Deserialize)]
struct CityFeature {
    #[serde(rename = "attributes")]
    city: City,
}

#[derive(Deserialize)]
struct QueryResponse {
    features: Vec<CityFeature>,
}

#[derive(Deserialize)]
struct NearestLayerValue {
    #[serde(rename = "featureSet")]
    feature_set: QueryResponse,
}

#[derive(Deserialize)]
struct NearestLayerResponse {
    value: NearestLayerValue,
}

async fn login(
    client: &reqwest::Client,
    username: &String,
    password: &String,
    referrer: &String,
) -> Result<Response> {
    let mut params = HashMap::new();
    let f_json = String::from("json");
    params.insert("username", username);
    params.insert("password", password);
    params.insert("referer", referrer);
    params.insert("f", &f_json);

    client
        .post("https://www.arcgis.com/sharing/rest/generateToken")
        .form(&params)
        .send()
        .await
}

async fn get_portal_self(
    client: &reqwest::Client,
    token: &String,
    referrer: &String,
) -> Result<PortalSelf> {
    let f_json = String::from("json");
    let mut result: Result<Response> = client
        .get("https://www.arcgis.com/sharing/rest/portals/self")
        .query(&[("token", token), ("referer", referrer), ("f", &f_json)])
        .send()
        .await;
    match result {
        Ok(response) => response.json().await,
        Err(err) => {
            println!("Couldn't get portal self: {:?}", err);
            Err(err)
        }
    }
}

async fn get_cities_count(client: &reqwest::Client, token: &String, referrer: &String) -> u32 {
    let f_json = String::from("json");
    let mut result: Result<Response> = client
        .get(format!("{}/query", FEATURE_LAYER_URL).as_str())
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
                    match json::parse(response_string.as_str()).unwrap()["count"].as_u32() {
                        Some(count) => count,
                        None => {
                            println!("Count is null (this should never happen)");
                            0
                        }
                    }
                }
                Err(err) => {
                    println!("Couldn't parse response: {:?}", err);
                    0
                }
            }
        }
        Err(err) => {
            println!("Couldn't get city count: {:?}", err);
            0
        }
    }
}

async fn get_minimum_population(
    client: &reqwest::Client,
    token: &String,
    referrer: &String,
    city_count: u32,
) -> u32 {
    let f_json = String::from("json");
    let mut result: Result<Response> = client
        .get(format!("{}/query", FEATURE_LAYER_URL).as_str())
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
                    match json::parse(response_string.as_str()).unwrap()["features"][0]
                        ["attributes"]["population"]
                        .as_u32()
                    {
                        Some(population) => population,
                        None => {
                            println!("Population is null (this should never happen)");
                            0
                        }
                    }
                }
                Err(err) => {
                    println!("Couldn't parse response: {:?}", err);
                    0
                }
            }
        }
        Err(err) => {
            println!("Couldn't get minimum population: {:?}", err);
            0
        }
    }
}

async fn get_cities(
    client: &reqwest::Client,
    token: &String,
    referrer: &String,
    fids: Vec<u32>,
) -> Vec<CityFeature> {
    let f_json = String::from("json");
    let fid_strings: Vec<String> = fids.iter().map(ToString::to_string).collect();
    let mut result: Result<Response> = client
        .get(format!("{}/query", FEATURE_LAYER_URL).as_str())
        .query(&[
            ("objectIds", &fid_strings.join(",")),
            ("outFields", &String::from("*")),
            ("token", token),
            ("referer", referrer),
            ("f", &f_json),
        ])
        .send()
        .await;
    match result {
        Ok(response) => {
            //println!("Here it is:\n{}", &response.text().await.unwrap());
            let response_result: Result<QueryResponse> = response.json().await;
            match response_result {
                Ok(query_response) => query_response.features,
                Err(err) => {
                    println!("Couldn't parse city query response: {:?}", err);
                    Vec::new()
                }
            }
        }
        Err(err) => {
            println!("Couldn't get query results: {:?}", err);
            Vec::new()
        }
    }
}

async fn get_random_city_pair(
    client: &reqwest::Client,
    token: &String,
    referrer: &String,
    minimum_population: u32,
) -> Result<(City, City)> {
    println!(
        "Getting a random city pair with minimum population {}",
        minimum_population
    );
    let f_json = String::from("json");
    let mut result: Result<Response> = client
        .get(format!("{}/query", FEATURE_LAYER_URL).as_str())
        .query(&[
            (
                "outStatistics",
                r#"
                [
                    {
                        "statisticType": "max",
                        "onStatisticField": "FID",
                        "outStatisticFieldName": "max_fid"
                    },
                    {
                        "statisticType": "min",
                        "onStatisticField": "FID",
                        "outStatisticFieldName": "min_fid"
                    }
                ]
            "#,
            ),
            (
                "where",
                format!("population >= {}", minimum_population).as_str(),
            ),
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
                    let json_value = json::parse(response_string.as_str()).unwrap();
                    let attributes = &json_value["features"][0]["attributes"];
                    let min_fid = attributes["min_fid"].as_u32().unwrap();
                    let max_fid = attributes["max_fid"].as_u32().unwrap();
                    let mut cities: Vec<City> = Vec::new();
                    let mut tried_fids = HashSet::new();
                    while 2 > cities.len() {
                        let mut rng = rand::thread_rng();
                        let mut fids = Vec::new();
                        while 2 > fids.len() {
                            let fid = rng.gen_range(min_fid, max_fid + 1);
                            if tried_fids.insert(fid) {
                                fids.push(fid);
                            }
                        }
                        let city_results = get_cities(client, token, referrer, fids).await;
                        for city_feature in city_results {
                            if 2 > cities.len()
                                && city_feature.city.population >= minimum_population
                            {
                                cities.push(city_feature.city);
                            }
                        }
                    }
                    Ok((cities.remove(0), cities.remove(0)))
                }
                Err(err) => {
                    println!("Couldn't parse statistics response: {:?}", err);
                    Err(err)
                }
            }
        }
        Err(err) => {
            println!("Couldn't get statistics: {:?}", err);
            Err(err)
        }
    }
}

fn point_geometry(x: f64, y: f64) -> json::JsonValue {
    let mut data = json::JsonValue::new_object();
    data["geometryType"] = "esriGeometryPoint".into();
    data["geometry"] = json::JsonValue::new_object().into();
    data["geometry"]["x"] = x.into();
    data["geometry"]["y"] = y.into();
    data
}

async fn get_distance(
    client: &reqwest::Client,
    token: &String,
    referrer: &String,
    portal_self: &PortalSelf,
    cities: &(&City, &City),
) -> f64 {
    let f_json = String::from("json");
    let mut result: Result<Response> = client
        .get(format!("{}/distance", &portal_self.helper_services.geometry.url).as_str())
        .query(&[
            (
                "geometry1",
                point_geometry(cities.0.lng, cities.0.lat).dump().as_str(),
            ),
            (
                "geometry2",
                point_geometry(cities.1.lng, cities.1.lat).dump().as_str(),
            ),
            ("sr", "4326"),
            ("distanceUnit", "9036"), // esriSRUnit_Kilometer
            ("geodesic", "true"),
            ("f", "json"),
        ])
        .send()
        .await;
    match result {
        Ok(response) => {
            let response_result: Result<String> = response.text().await;
            match response_result {
                Ok(response_string) => {
                    match json::parse(response_string.as_str()).unwrap()["distance"].as_f64() {
                        Some(distance) => distance,
                        None => {
                            println!("Distance is null (this should never happen)");
                            std::f64::MAX
                        }
                    }
                }
                Err(err) => {
                    println!("Couldn't parse response: {:?}", err);
                    0.0
                }
            }
        }
        Err(err) => {
            println!("Couldn't get distance: {:?}", err);
            0.0
        }
    }
}

async fn get_job_status(
    client: &reqwest::Client,
    token: &String,
    referrer: &String,
    portal_self: &PortalSelf,
    job_id: &String,
) -> String {
    match client
        .get(
            format!(
                "{}/FindNearest/jobs/{}",
                &portal_self.helper_services.analysis.url, job_id
            )
            .as_str(),
        )
        .header(CACHE_CONTROL, "no-cache")
        .query(&[
            ("token", token),
            ("referer", referrer),
            ("f", &String::from("json")),
        ])
        .send()
        .await
    {
        Ok(job_result) => match job_result.text().await {
            Ok(job_result_string) => {
                let json_result = json::parse(job_result_string.as_str()).unwrap();
                json_result["jobStatus"].as_str().unwrap().to_string()
            }
            Err(err) => {
                println!("Problem with job result string: {:?}", err);
                "unknown status".to_string()
            }
        },
        Err(err) => {
            println!("Could not get job result: {:?}", err);
            "unknown status".to_string()
        }
    }
}

async fn get_next_city(
    client: &reqwest::Client,
    token: &String,
    referrer: &String,
    portal_self: &PortalSelf,
    job_id: &String,
) -> Result<City> {
    println!("Job ID is {}", job_id);
    match client
        .get(
            format!(
                "{}/FindNearest/jobs/{}/results/nearestLayer",
                &portal_self.helper_services.analysis.url, job_id
            )
            .as_str(),
        )
        .query(&[
            ("token", token),
            ("referer", referrer),
            ("f", &String::from("json")),
        ])
        .send()
        .await
    {
        Ok(result) => match result.json().await {
            Ok(response) => {
                let mut response: NearestLayerResponse = response;
                let city: City = response.value.feature_set.features.remove(0).city;
                println!("Job result says {}", city.city);
                Ok(city)
            }
            Err(err) => {
                println!("Problem with job result string: {:?}", err);
                Err(err)
            }
        },
        Err(err) => {
            println!("Could not get job result: {:?}", err);
            Err(err)
        }
    }
}

fn directional_extent(city: &City, direction: &str) -> json::JsonValue {
    let mut extent = json::JsonValue::new_object();
    extent["spatialReference"] = json::JsonValue::new_object().into();
    extent["spatialReference"]["wkid"] = 4326.into();
    match direction {
        "n" => {
            extent["xmin"] = (-179.99999).into();
            extent["ymin"] = city.lat.into();
            extent["xmax"] = (179.99999).into();
            extent["ymax"] = (89.99999).into();
        }
        "s" => {
            extent["xmin"] = (-179.99999).into();
            extent["ymin"] = (-89.99999).into();
            extent["xmax"] = (179.99999).into();
            extent["ymax"] = city.lat.into();
        }
        "e" => {
            let mut xmax = city.lng + 180.;
            if xmax > 180. {
                xmax -= 360.;
            }
            extent["xmin"] = city.lng.into();
            extent["ymin"] = (-89.99999).into();
            extent["xmax"] = xmax.into();
            extent["ymax"] = (89.99999).into();
        }
        "w" => {
            let mut xmin = city.lng - 180.;
            if xmin < -180. {
                xmin -= 360.;
            }
            extent["xmin"] = xmin.into();
            extent["ymin"] = (-89.99999).into();
            extent["xmax"] = city.lng.into();
            extent["ymax"] = (89.99999).into();
        }
        _ => {}
    };
    extent
}

async fn create_game_item(
    client: &reqwest::Client,
    token: &String,
    referrer: &String,
    username: &String,
    cities_visited: &[&City]
) -> Option<String> {
    let mut params = HashMap::new();
    params.insert("token", token.as_str());
    params.insert("referer", referrer.as_str());
    params.insert("f", "json");
    params.insert("type", "Color Set");
    params.insert("typeKeywords", "Wanderer game");
    params.insert("title", "Wanderer Game 42");
    let city_ids: Vec<u32> = cities_visited.iter().map(|city| city.fid).collect();
    let text = json::stringify(object!{
        "cities_visited" => city_ids
    });
    params.insert("text", text.as_str());

    let result = client
        .post(format!("https://www.arcgis.com/sharing/rest/content/users/{}/addItem", &username).as_str())
        .form(&params)
        .send()
        .await;
    match result {
        Ok(response) => {
            let response_result: Result<String> = response.text().await;
            match response_result {
                Ok(response_string) => {
                    println!("response string is {}", response_string.as_str());
                    let response_json = json::parse(response_string.as_str()).unwrap();
                    match response_json["success"].as_bool() {
                        Some(success) => {
                            if success {
                                Some(String::from(response_json["id"].as_str().unwrap()))
                            } else {
                                None
                            }
                        },
                        None => {
                            println!("Could not add item. No 'success' value in response.");
                            None
                        }
                    }
                },
                Err(err) => {
                    println!("Could not add item: {}", err);
                    None
                }
            }
        },
        Err(err) => {
            println!("Could not add item: {}", err);
            None
        }
    }
}

async fn play_game(client: &reqwest::Client, token: &String, referrer: &String, username: &String, city_count: u32) {
    println!("Let's play Wanderer with {} cities", city_count);
    // We need the portal self for its URLs
    let portal_self = get_portal_self(client, token, referrer).await.unwrap();

    // Get the minimum population for cities in this game
    let minimum_population = get_minimum_population(client, token, referrer, city_count).await;
    println!("Minimum population: {}", minimum_population);
    // Get a couple of random cities
    match get_random_city_pair(client, token, referrer, minimum_population).await {
        Ok(cities) => {
            println!("Hey, Wanderer! Let's see if you can make it to the secret destination.");
            let mut current_city: &City = &cities.0;
            let target_city: &City = &cities.1;

            match create_game_item(client, token, referrer, username, &[current_city]).await {
                Some(id) => {
                    println!("Successfully created game item {}", id);
                },
                None => {
                    println!("Failed to create game item");    
                }
            }

            let mut distance_to_target: f64 = get_distance(
                client,
                token,
                referrer,
                &portal_self,
                &(current_city, target_city),
            )
            .await;
            let mut rng = rand::thread_rng();
            let mut welcome_vars = HashMap::new();
            welcome_vars.insert(String::from("city"), &current_city.city);
            println!(
                "{}",
                strfmt(
                    WELCOME_MESSAGES[rng.gen_range(0, WELCOME_MESSAGES.len())],
                    &welcome_vars
                )
                .unwrap()
            );
            loop {
                println!(
                    "You are now {:.0}km from your destination.",
                    distance_to_target
                );
                println!("What's next, Wanderer? (n, s, e, w, info)");
                let mut cmd = String::new();
                io::stdin()
                    .read_line(&mut cmd)
                    .expect("Failed to read command");
                cmd = String::from(cmd.trim()).to_lowercase();
                let cmd = cmd.as_str();
                match cmd {
                    "n" | "s" | "e" | "w" => {
                        println!("You decide to travel {}.", cmd);
                        let extent = directional_extent(&current_city, cmd);
                        let mut out_sr = json::JsonValue::new_object();
                        out_sr["wkid"] = 4326.into();
                        let mut context = json::JsonValue::new_object();
                        context["extent"] = extent.into();
                        context["outSR"] = out_sr.into();
                        let mut analysis_layer = json::JsonValue::new_object();
                        analysis_layer["url"] = FEATURE_LAYER_URL.into();
                        analysis_layer["filter"] = format!(
                            "population >= {} AND FID <> {}",
                            minimum_population, current_city.fid
                        )
                        .into();
                        let mut near_layer = json::JsonValue::new_object();
                        near_layer["url"] = FEATURE_LAYER_URL.into();
                        near_layer["filter"] = format!("FID = {}", current_city.fid).into();
                        println!("Token is {}", token);
                        let mut result: Result<Response> = client
                            .post(
                                format!(
                                    "{}/FindNearest/submitJob",
                                    &portal_self.helper_services.analysis.url
                                )
                                .as_str(),
                            )
                            .form(&[
                                ("analysisLayer", analysis_layer.dump().as_str()),
                                ("nearLayer", near_layer.dump().as_str()),
                                ("measurementType", "StraightLine"),
                                ("maxCount", "2"),
                                ("context", context.dump().as_str()),
                                ("token", token),
                                ("referer", referrer),
                                ("f", "json"),
                            ])
                            .send()
                            .await;
                        match result {
                            Ok(response) => {
                                let response_result: Result<String> = response.text().await;
                                match response_result {
                                    Ok(response_string) => {
                                        let response_json =
                                            json::parse(response_string.as_str()).unwrap();
                                        let mut status = String::from(
                                            response_json["jobStatus"].as_str().unwrap(),
                                        );
                                        let job_id =
                                            String::from(response_json["jobId"].as_str().unwrap());
                                        println!("Waiting for job {}", job_id);
                                        loop {
                                            match status.as_str() {
                                                "esriJobSucceeded" => {
                                                    let city = get_next_city(
                                                        client,
                                                        token,
                                                        referrer,
                                                        &portal_self,
                                                        &job_id,
                                                    )
                                                    .await
                                                    .unwrap();
                                                    let current_city = &city;
                                                    println!("The next city is {}", city.city);
                                                    break;
                                                }
                                                "esriJobFailed" | "esriJobTimedOut"
                                                | "esriJobCancelled" => {
                                                    println!(
                                                        "Could not move to a city at this time."
                                                    );
                                                    break;
                                                }
                                                _ => {
                                                    thread::sleep(time::Duration::from_millis(
                                                        5000,
                                                    ));
                                                    status = get_job_status(
                                                        client,
                                                        token,
                                                        referrer,
                                                        &portal_self,
                                                        &job_id,
                                                    )
                                                    .await;
                                                    println!("{}", status);
                                                }
                                            };
                                        }
                                    }
                                    Err(err) => {
                                        println!("Couldn't parse response: {:?}", err);
                                    }
                                }
                            }
                            Err(err) => {
                                println!("Couldn't get distance: {:?}", err);
                            }
                        }

                        welcome_vars.insert(String::from("city"), &current_city.city);
                        println!(
                            "{}",
                            strfmt(
                                WELCOME_MESSAGES[rng.gen_range(0, WELCOME_MESSAGES.len())],
                                &welcome_vars
                            )
                            .unwrap()
                        );
                    }
                    "info" => {
                        println!(
                            "Current location: {}, {}, {}",
                            &current_city.city, &current_city.admin_name, &current_city.country
                        );
                        let mut bearing = Point::<f64>::new(current_city.lng, current_city.lat)
                            .bearing(Point::<f64>::new(target_city.lng, target_city.lat));
                        while bearing > 360.0 {
                            bearing -= 360.0;
                        }
                        while bearing < 0.0 {
                            bearing += 360.0;
                        }
                        println!(
                            "Your destination is {:.0}km away at a bearing of {:.0} degrees.",
                            distance_to_target, bearing
                        );
                    }
                    _ => {
                        println!("I don't know how to {}", cmd);
                    }
                }
            }
        }
        Err(err) => println!("No cities?! {}", err),
    }
}

#[tokio::main]
async fn main() {
    let referrer = format!("Referrer {}", Uuid::new_v4());
    println!("Wanderer {}", VERSION);
    println!("ArcGIS Online username:");
    let mut username = String::new();
    io::stdin()
        .read_line(&mut username)
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
                    println!(
                        "Level of difficulty (0 = easy, 1 = medium, 2 = hard, 3 = legendary):"
                    );
                    let mut difficulty = String::new();
                    io::stdin()
                        .read_line(&mut difficulty)
                        .expect("Failed to read line");
                    let difficulty: u32 = match difficulty.trim().parse() {
                        Ok(num) => num,
                        Err(_) => {
                            println!("Okay, then you get the default of 0 = easy.");
                            0
                        }
                    };
                    let city_count: u32 = match difficulty {
                        0 => 10,
                        1 => 100,
                        2 => 1000,
                        3 => get_cities_count(&reqwest_client, &json.token, &referrer).await,
                        _ => {
                            println!("Okay, then you get the default of 0 = easy.");
                            10
                        }
                    };

                    play_game(&reqwest_client, &json.token, &referrer, &username, city_count).await;
                }
                Err(err) => println!("error here: {:?}", err),
            }
        }
        Err(err) => println!("error parsing response: {:?}", err),
    }
}
