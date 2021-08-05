#[macro_use]
extern crate rocket;

mod auth;
mod config;
mod error;
mod storage;

#[cfg(test)]
mod tests;

use std::convert::TryInto;
use std::io::{Cursor, Read, Seek};

use anyhow::{format_err, Context};
use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
use libwally::{
    manifest::{Manifest, MANIFEST_FILE_NAME},
    package_id::PackageId,
    package_index::PackageIndex,
    package_name::PackageName,
};
use rocket::{
    data::{Data, ToByteUnit},
    fairing::AdHoc,
    http::{ContentType, Status},
    response::{Content, Stream},
    State,
};
use rocket_contrib::json::Json;
use semver::Version;
use serde_json::json;
use storage::StorageMode;
use zip::ZipArchive;

use crate::auth::{ReadAccess, WriteAccess};
use crate::config::Config;
use crate::error::{ApiErrorContext, ApiErrorStatus, Error};
use crate::storage::{GcsStorage, LocalStorage, StorageBackend, StorageOutput};

#[get("/")]
fn root() -> Json<serde_json::Value> {
    Json(json!({
        "message": "Wally Registry is up and running!",
    }))
}

#[get("/v1/package-contents/<scope>/<name>/<version>")]
async fn package_contents(
    storage: State<'_, Box<dyn StorageBackend>>,
    _read: ReadAccess,
    scope: String,
    name: String,
    version: String,
) -> Result<Content<Stream<StorageOutput>>, Error> {
    let package_name = PackageName::new(scope, name)
        .context("error parsing package name")
        .status(Status::BadRequest)?;
    let version: Version = version
        .parse()
        .context("error parsing version")
        .status(Status::BadRequest)?;
    let package_id = PackageId::new(package_name, version);

    match storage.read(&package_id).await.map(Stream::from) {
        Ok(stream) => Ok(Content(ContentType::GZIP, stream)),
        Err(e) => Err(e).status(Status::NotFound),
    }
}

#[post("/v1/publish", data = "<data>")]
async fn publish(
    storage: State<'_, Box<dyn StorageBackend>>,
    index: State<'_, PackageIndex>,
    authorization: WriteAccess,
    data: Data,
) -> Result<Json<serde_json::Value>, Error> {
    let contents = data
        .open(2.mebibytes())
        .into_bytes()
        .await
        .context("could not read request body")?;

    if !contents.is_complete() {
        return Err(format_err!("request body too large").status(Status::BadRequest));
    }

    let contents = Cursor::new(contents.value);
    let mut archive = ZipArchive::new(contents)
        .context("could not read ZIP archive")
        .status(Status::BadRequest)?;

    index.update()?;

    let manifest = get_manifest(&mut archive).status(Status::BadRequest)?;
    let package_id = manifest.package_id();

    if let Err(message) = authorization.can_write_package(&package_id, &index) {
        return Err(format_err!(message).status(Status::Unauthorized));
    }

    let package_metadata = index.get_package_metadata(manifest.package_id().name());

    if let Ok(metadata) = package_metadata {
        if metadata.versions.iter().any(|published_manifest| {
            published_manifest.package.version == manifest.package.version
        }) {
            return Err(format_err!("package already exists in index").status(Status::Conflict));
        }
    }

    storage
        .write(&manifest.package_id(), &archive.into_inner().into_inner())
        .await
        .context("could not write package to storage backend")?;

    index
        .publish(&manifest)
        .context("could not publish package to index")?;

    Ok(Json(json!({
        "message": "TODO: Implement this endpoint"
    })))
}

fn get_manifest<R: Read + Seek>(archive: &mut ZipArchive<R>) -> anyhow::Result<Manifest> {
    let mut manifest_file = archive
        .by_name(MANIFEST_FILE_NAME)
        .context("could not find manifest file")?;

    let uncompressed_size: usize = manifest_file
        .size()
        .try_into()
        .context("uncompressed archive size is too big")?;

    let mut manifest_contents = Vec::with_capacity(uncompressed_size);
    manifest_file
        .read_to_end(&mut manifest_contents)
        .context("could not read manifest file")?;

    let manifest = Manifest::from_slice(&manifest_contents)?;

    Ok(manifest)
}

pub fn server(figment: Figment) -> rocket::Rocket {
    let config: Config = figment.extract().expect("could not read configuration");

    println!("Using authentication mode: {:?}", config.auth);

    println!("Using storage backend: {:?}", config.storage);
    let storage_backend: Box<dyn StorageBackend> = match config.storage {
        StorageMode::Local { path } => Box::new(LocalStorage::new(path)),
        StorageMode::Gcs { bucket } => Box::new(configure_gcs(bucket).unwrap()),
    };

    println!("Cloning package index repository...");
    let package_index = PackageIndex::new_temp(&config.index_url, config.github_token).unwrap();

    rocket::custom(figment)
        .mount("/", routes![root, package_contents, publish])
        .manage(storage_backend)
        .manage(package_index)
        .attach(AdHoc::config::<Config>())
}

fn configure_gcs(bucket: String) -> anyhow::Result<GcsStorage> {
    use cloud_storage_lite::{
        token_provider::{
            oauth::{OAuthTokenProvider, ServiceAccount, SCOPE_STORAGE_FULL_CONTROL},
            RenewingTokenProvider,
        },
        Client,
    };

    let token_provider = RenewingTokenProvider::new(OAuthTokenProvider::new(
        ServiceAccount::read_from_canonical_env()?,
        SCOPE_STORAGE_FULL_CONTROL,
    )?);
    let client = Client::new(token_provider).into_bucket_client(bucket);

    Ok(GcsStorage::new(client))
}

#[launch]
fn rocket() -> rocket::Rocket {
    let figment = Figment::from(rocket::Config::default())
        .merge(Toml::file("Rocket.toml").nested())
        .merge(Env::prefixed("WALLY_").global());

    server(figment)
}
