//! # Quarenta: a Rust library and API for ArcGIS REST interfaces
//! 
//! `quarenta` helps Rust developers access ArcGIS RESTful services.
#![crate_name = "quarenta"]

use std::collections::HashMap;
use std::error::Error;

use json::JsonValue;

type BoxResult<T> = Result<T,Box<dyn Error>>;

/// Attempts to login to ArcGIS Online.
/// 
/// # Arguments
/// 
/// * `client` - A `reqwest` client. If `None`, this method creates a new one. It is advisable to create one and reuse it.
/// * `username` - The username
/// * `password` - The password
/// * `referrer` - A referrer string that will need to be used with the resulting token
/// 
/// # Return
/// 
/// The function returns a JSON result which, if `Ok`, contains either values for `token`,
/// `expires`, and `ssl` or an `error` object. An invalid username/password combination
/// results in the `error` object.
/// 
/// # Errors
/// 
/// The function's result will be either `Ok` or `Err`. If `Err`, it's probably because the login
/// request went wrong (e.g. no network connectivity), not because of a bad username or password.
/// 
/// # Examples
/// 
/// ```
/// let reqwest_client = reqwest::Client::new();
/// let login_result: Result<T,Box<dyn Error>> = login(&reqwest_client, &username, &password, &referrer).await;
/// match login_result {
///    Ok(token_response) => {
///        match token_response["token"].as_str() {
///            Some(token) => {
///                println!("Token: {}...", &token[..20]);
///                println!("Expiry: {}", token_response["expires"]);
///                println!("SSL only: {}", token_response["ssl"]);
///            },
///            None => {
///                println!("Login returned but was not successful: {}", token_response);
///            }
///        }
///    },
///    Err(err) => {
///        println!("Something went wrong while calling login: {:?}", err);
///    },
/// }
/// ```
pub async fn login(
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
                Ok(text) => Ok(json::parse(text.as_str()).unwrap()),
                Err(err) => Err(Box::new(err)),
            }
        },
        Err(err) => Err(Box::new(err)),
    }
}
