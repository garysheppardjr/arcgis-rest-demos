use std::collections::HashMap;
use std::error::Error;
use std::io;

#[macro_use]
extern crate json;
use json::JsonValue;
use reqwest::Client;

type BoxResult<T> = Result<T,Box<dyn Error>>;

const DEFAULT_FEATURE_LAYER_URL: &'static str = "https://services.arcgis.com/P3ePLMYs2RVChkJx/arcgis/rest/services/World_Cities/FeatureServer/0";
const GEOMETRY_SERVICE_URL: &'static str = "https://tasks.arcgisonline.com/ArcGIS/rest/services/Geometry/GeometryServer";
const DEFAULT_BUFFER_DISTANCE_M: i32 = 100_000;

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
    let buffer_distance: f64 = read_from_console(format!("Buffer distance in meters (default is {}):", DEFAULT_BUFFER_DISTANCE_M).as_str()).parse().unwrap_or(DEFAULT_BUFFER_DISTANCE_M.into());
    let dir: String = read_from_console("Direction: (n | s | e | w; default is all)");

    buffer_and_query(&reqwest_client, "esriGeometryPoint", object!{"x" => lon, "y" => lat}, buffer_distance, url.as_str()).await;

    read_from_console("Type Enter to exit");
}

async fn buffer_and_query(
    client: &reqwest::Client,
    geometry_type: &str,
    geometry: JsonValue,
    buffer_distance_m: f64,
    feature_layer_url: &str
)  {
    match buffer(client, geometry_type, geometry, &buffer_distance_m).await {
        Ok(response) => {
            let stringified_query_response;
            match query_features(client, Some("0=0"), Some("esriGeometryPolygon"), Some(response), feature_layer_url).await {
                Ok(query_response) => {
                    let response_count = query_response.len();
                    stringified_query_response = json::stringify_pretty(query_response, 2);
                    println!("Query response");
                    println!("{}", stringified_query_response);
                    println!("There are {} features inside the buffer.", response_count);
                },
                Err(err) => {
                    println!("Error: {:?}", err);
                }
            }
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
    buffer_distance_m: &f64
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

async fn query_features(
    client: &reqwest::Client,
    where_clause: Option<&str>,
    geometry_type: Option<&str>,
    geometry: Option<JsonValue>,
    feature_layer_url: &str
// TODO return BoxResult<Array> instead
) -> BoxResult<JsonValue> {
    let mut params = HashMap::new();
    params.insert("f", "json");
    params.insert("outFields", "*");
    params.insert("where", match where_clause {
        Some(_where_clause) => _where_clause,
        _ => "0=0"
    });
    let stringified_geometry;
    match geometry_type {
        Some(_geometry_type) => {
            match geometry {
                Some(_geometry) => {
                    stringified_geometry = json::stringify(_geometry);
                    params.insert("geometry", stringified_geometry.as_str());
                    params.insert("geometryType", _geometry_type);
                    params.insert("inSR", "4326");
                },
                _ => {}
            }
        },
        _ => {}
    }

    // Semantically, GET makes more sense than POST. However, the buffer string might be
    // too long for a GET.
    match client.post(format!("{}/query", feature_layer_url).as_str()).form(&params).send().await {
        Ok(response) => {
            match response.text().await {
                Ok(text) => Ok(json::parse(text.as_str()).unwrap().take()["features"].take()),
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
