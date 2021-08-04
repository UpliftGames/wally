use anyhow::format_err;
use serde::Deserialize;
use std::path::PathBuf;
use std::{thread, time};
use structopt::StructOpt;
use webbrowser;

use crate::{auth::AuthStore, manifest::Manifest, package_index::PackageIndex};

/// Log into a registry.
#[derive(Debug, StructOpt)]
pub struct LoginSubcommand {
    /// Path to a project to decide how to login
    #[structopt(long = "project-path", default_value = ".")]
    pub project_path: PathBuf,
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

fn await_github_auth(
    device_code_response: DeviceCodeResponse,
    oauth_id: &str,
) -> anyhow::Result<DeviceCodeAuth> {
    let client = reqwest::blocking::Client::new();

    thread::sleep(time::Duration::from_secs(device_code_response.interval));

    let response = client
        .post("https://github.com/login/oauth/access_token")
        .header("accept", "application/json")
        .json(&serde_json::json!({
            "client_id": oauth_id,
            "device_code": device_code_response.device_code,
            "grant_type": "urn:ietf:params:oauth:grant-type:device_code",
        }))
        .send()?
        .json::<AuthResponse>()?;

    match response {
        AuthResponse::Ok(auth) => Ok(auth),
        AuthResponse::Err { error } => match error.as_ref() {
            "authorization_pending" => await_github_auth(device_code_response, oauth_id),
            _ => Err(format_err!("Oauth request error: {}", error)),
        },
    }
}

fn prompt_api_key(api: url::Url) -> anyhow::Result<()> {
    println!("Enter an API token for {}", api);
    println!();
    let token = rpassword::prompt_password_stdout("Enter token: ")?;

    AuthStore::set_token(api.as_str(), Some(&token))
}

fn prompt_github_auth(api: url::Url, oauth_id: &str) -> anyhow::Result<()> {
    let client = reqwest::blocking::Client::new();
    let device_code_response = client
        .post("https://github.com/login/device/code")
        .header("accept", "application/json")
        .json(&serde_json::json!({
            "client_id": oauth_id,
            "scope": "read:user",
        }))
        .send()?
        .json::<DeviceCodeResponse>()?;

    println!("Go to {}", device_code_response.verification_uri);
    println!("And enter the code: {}", device_code_response.user_code);

    webbrowser::open(&device_code_response.verification_uri).ok();

    println!("Awaiting authorization...");

    match await_github_auth(device_code_response, oauth_id) {
        Ok(auth) => {
            println!("Authorization successful!");
            AuthStore::set_token(api.as_str(), Some(&auth.access_token))?;
            Ok(())
        }
        Err(error) => Err(error),
    }
}

impl LoginSubcommand {
    pub fn run(self) -> anyhow::Result<()> {
        let manifest = Manifest::load(&self.project_path)?;
        let registry = url::Url::parse(&manifest.package.registry)?;
        let package_index = PackageIndex::new(&registry, None)?;
        let api = package_index.config()?.api;
        let oauth_id = package_index.config()?.oauth_id;

        match oauth_id {
            None => prompt_api_key(api),
            Some(oauth_id) => prompt_github_auth(api, &oauth_id),
        }
    }
}
