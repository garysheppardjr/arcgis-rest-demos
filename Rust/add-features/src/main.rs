use std::collections::HashMap;
use std::error::Error;
use std::io;
use std::result::Result;

#[macro_use]
extern crate json;
use json::Array;
use json::JsonValue;

type BoxResult<T> = Result<T,Box<dyn Error>>;

const TYPES_DESCRIPTION: &str = r#"Incident types:
    1. Dead animal
    2. Graffiti
    3. Manhole cover
    4. Pothole
    5. Street light
    6. Street sign
    7. Other"#;

#[tokio::main]
async fn main() {
    let reqwest_client = reqwest::Client::new();
    println!("Add Features Demo");

    let lon: f64 = read_from_console("Longitude:").parse().unwrap();
    let lat: f64 = read_from_console("Latitude:").parse().unwrap();
    let incident_type: String = read_from_console(format!("{}\n\nIncident type:", TYPES_DESCRIPTION).as_str());
    let incident_description: String = read_from_console("Incident description:");

    let feature: JsonValue = object!{
        "attributes" => object!{
            "IncidentType" => incident_type,
            "IncidentDescription" => incident_description
        },
        "geometry" => object!{
            "x" => lon,
            "y" => lat,
            "spatialReference" => object! { "wkid" => 4326 }
        }
    };
    match add_features(&reqwest_client, [ feature ].to_vec()).await {
        Ok(response) => {
            println!("The response: {}", response);
        },
        Err(err) => {
            println!("Error: {:?}", err);
        }
    }

    println!("Type Enter to exit");
    let mut exit = String::new();
    io::stdin()
        .read_line(&mut exit)
        .expect("Failed to read line");
}

fn read_from_console(prompt: &str) -> String {
    println!("{}", prompt);
    let mut value = String::new();
    io::stdin()
        .read_line(&mut value)
        .expect("Failed to read line");
    String::from(value.trim())
}

async fn add_features(
    client: &reqwest::Client,
    features: Array
) -> BoxResult<JsonValue> {
    let features_string = json::stringify(features);
    let mut params = HashMap::new();
    params.insert("f", "json");
    params.insert("features", features_string.as_str());

    match client.post(
        "https://services.arcgis.com/V6ZHFr6zdgNZuVG0/ArcGIS/rest/services/IncidentsReport/FeatureServer/0/addFeatures"
    ).form(&params).send().await {
        Ok(response) => {
            match response.text().await {
                Ok(text) => Ok(json::parse(text.as_str()).unwrap()),
                Err(err) => Err(Box::new(err)),
            }
        },
        Err(err) => Err(Box::new(err)),
    }
}
