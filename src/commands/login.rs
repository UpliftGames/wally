use anyhow::format_err;
use serde::Deserialize;
use std::{thread, time};
use structopt::StructOpt;

use crate::auth::AuthStore;

/// Log into a registry.
#[derive(Debug, StructOpt)]
pub struct LoginSubcommand {
    /// Authentication token for the registry. If not specified, Wally will
    /// prompt.
    pub token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}

#[derive(Deserialize)]
struct DeviceCodeAuth {
    access_token: String,
    token_type: String,
    scope: String,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum AuthResponse {
    Ok(DeviceCodeAuth),
    Err { error: String },
}

fn await_github_auth(device_code_response: DeviceCodeResponse) -> anyhow::Result<DeviceCodeAuth> {
    let client = reqwest::blocking::Client::new();

    thread::sleep(time::Duration::from_secs(device_code_response.interval));

    let response = client
        .post("https://github.com/login/oauth/access_token")
        .header("accept", "application/json")
        .json(&serde_json::json!({
            "client_id": "7bd503594a0f9a9f7ed3",
            "device_code": device_code_response.device_code,
            "grant_type": "urn:ietf:params:oauth:grant-type:device_code",
        }))
        .send()?
        .json::<AuthResponse>()?;

    match response {
        AuthResponse::Ok(auth) => Ok(auth),
        AuthResponse::Err { error } => match error.as_ref() {
            "authorization_pending" => await_github_auth(device_code_response),
            _ => Err(format_err!("Oauth request error: {}", error)),
        },
    }
}

impl LoginSubcommand {
    pub fn run(self) -> anyhow::Result<()> {
        /*let token = match self.token {
            Some(token) => token,
            None => {
                println!("Wally currently authenticates to registries with an API token.");
                println!("In the future, Wally will support GitHub authentication.");
                println!();
                rpassword::prompt_password_stdout("Enter token: ")?
            }
        };

        AuthStore::set_token(Some(&token))?;*/

        let client = reqwest::blocking::Client::new();
        let device_code_response = client
            .post("https://github.com/login/device/code")
            .header("accept", "application/json")
            .json(&serde_json::json!({
                "client_id": "7bd503594a0f9a9f7ed3",
                "scope": "read:user",
            }))
            .send()?
            .json::<DeviceCodeResponse>()?;

        println!(
            "Go to {} and enter the code {}",
            device_code_response.verification_uri, device_code_response.user_code
        );

        println!("Awaiting authorization...");

        match await_github_auth(device_code_response) {
            Ok(auth) => {
                println!("Authorization successful!");
                AuthStore::set_token(Some(&auth.access_token))?;
                Ok(())
            }
            Err(error) => Err(error),
        }
    }
}
