use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;

use anyhow::format_err;
use opener;
use reqwest::blocking::Client;
use reqwest::Url;
use serde::Deserialize;
use structopt::StructOpt;

use crate::{auth::AuthStore, manifest::Manifest, package_index::PackageIndex};

/// Log into a registry.
#[derive(Debug, StructOpt)]
pub struct LoginSubcommand {
    /// Path to a project to decide how to login
    #[structopt(long = "project-path", default_value = ".")]
    pub project_path: PathBuf,
    /// GitHub auth token to set directly
    ///
    /// Must be used in conjunction with --api
    #[structopt(long = "token")]
    pub token: Option<String>,
    /// URL of the remote index to add an auth token for
    ///
    /// Must be used in conjunction with --token
    #[structopt(long = "api")]
    pub api: Option<String>,
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

fn wait_for_github_auth(
    device_code_response: DeviceCodeResponse,
    github_oauth_id: &str,
) -> anyhow::Result<DeviceCodeAuth> {
    sleep(Duration::from_secs(device_code_response.interval));

    let client = Client::new();
    let response = client
        .post("https://github.com/login/oauth/access_token")
        .header("accept", "application/json")
        .json(&serde_json::json!({
            "client_id": github_oauth_id,
            "device_code": device_code_response.device_code,
            "grant_type": "urn:ietf:params:oauth:grant-type:device_code",
        }))
        .send()?
        .json::<AuthResponse>()?;

    match response {
        AuthResponse::Ok(auth) => Ok(auth),
        AuthResponse::Err { error } => match error.as_ref() {
            "authorization_pending" => wait_for_github_auth(device_code_response, github_oauth_id),
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

fn prompt_github_auth(api: url::Url, github_oauth_id: &str) -> anyhow::Result<()> {
    let client = Client::new();
    let device_code_response = client
        .post("https://github.com/login/device/code")
        .header("accept", "application/json")
        .json(&serde_json::json!({
            "client_id": github_oauth_id,
            "scope": "read:user",
        }))
        .send()?
        .json::<DeviceCodeResponse>()?;

    println!();
    println!("Go to {}", device_code_response.verification_uri);
    println!("And enter the code: {}", device_code_response.user_code);
    println!();

    opener::open(&device_code_response.verification_uri).ok();

    println!("Awaiting authorization...");
    let auth = wait_for_github_auth(device_code_response, github_oauth_id)?;

    println!("Authorization successful!");
    AuthStore::set_token(api.as_str(), Some(&auth.access_token))
}

impl LoginSubcommand {
    pub fn run(self) -> anyhow::Result<()> {
        if self.api.is_some() != self.token.is_some() {
            anyhow::bail!("Both --api and --token must be passed to manually set an auth token")
        }
        match self.api.zip(self.token) {
            Some((api, token)) => AuthStore::set_token(&api, Some(&token)),
            None => {
                let manifest = Manifest::load(&self.project_path)?;
                let registry = Url::parse(&manifest.package.registry)?;
                let package_index = PackageIndex::new(&registry, None)?;
                let api = package_index.config()?.api;
                let github_oauth_id = package_index.config()?.github_oauth_id;

                match github_oauth_id {
                    None => prompt_api_key(api),
                    Some(github_oauth_id) => prompt_github_auth(api, &github_oauth_id),
                }
            }
        }
    }
}
