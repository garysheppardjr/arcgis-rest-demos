use std::collections::HashMap;
use std::error::Error;
use std::io;

#[macro_use]
extern crate json;
use json::Array;
use json::JsonValue;
use reqwest::Client;

type BoxResult<T> = Result<T,Box<dyn Error>>;

const DEFAULT_FEATURE_LAYER_URL: &'static str = "https://services.arcgis.com/P3ePLMYs2RVChkJx/arcgis/rest/services/World_Cities/FeatureServer/0";
const GEOMETRY_SERVICE_URL: &'static str = "https://tasks.arcgisonline.com/ArcGIS/rest/services/Geometry/GeometryServer";

#[tokio::main]
async fn main() {
    let reqwest_client = Client::new();
    println!("Buffer and Query Demo");

    let lon: f64 = read_from_console("Longitude:").parse().unwrap();
    let lat: f64 = read_from_console("Latitude:").parse().unwrap();
    let mut url: String = read_from_console(format!("Feature layer URL:\n\t(Default: {} )", DEFAULT_FEATURE_LAYER_URL).as_str());
    if "" == url {
        url = String::from(DEFAULT_FEATURE_LAYER_URL);
    }
    let dir: String = read_from_console("Direction: (n | s | e | w; default is all)");

    buffer_and_query(&reqwest_client, "esriGeometryPoint", object!{"x" => lon, "y" => lat}, 100_000).await;

    read_from_console("Type Enter to exit");
}

async fn buffer_and_query(
    client: &reqwest::Client,
    geometry_type: &str,
    geometry: JsonValue,
    buffer_distance_m: i32
)  {
    match buffer(client, geometry_type, geometry, &buffer_distance_m).await {
        Ok(response) => {
            println!("The response: {}", response);
        },
        Err(err) => {
            println!("Error: {:?}", err);
        }
    }
}

async fn buffer(
    client: &reqwest::Client,
    geometry_type: &str,
    geometry: JsonValue,
    buffer_distance_m: &i32
) -> BoxResult<JsonValue> {
    let geometries: String = json::stringify(object!{
        "geometryType" => geometry_type,
        "geometries" => array![ geometry ]
    });
    let distance_string = buffer_distance_m.to_string();
    let mut params = HashMap::new();
    params.insert("f", "json");
    params.insert("geometries", geometries.as_str());
    params.insert("inSR", "4326");
    params.insert("outSR", "4326");
    params.insert("distances", distance_string.as_str());
    params.insert("unit", "9001"); // meters
    params.insert("geodesic", "true");

    match client.get(format!("{}/buffer", GEOMETRY_SERVICE_URL).as_str()).query(&params).send().await {
        Ok(response) => {
            match response.text().await {
                Ok(text) => Ok(json::parse(text.as_str()).unwrap().take()["geometries"].take()[0].take()),
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
