#[macro_use]
extern crate rocket;

mod auth;
mod config;
mod error;
mod search;
mod storage;

#[cfg(test)]
mod tests;

use std::convert::TryInto;
use std::io::{Cursor, Read, Seek};
use std::sync::RwLock;

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
use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Header;
use rocket::request::{FromRequest, Outcome};
use rocket::response::stream::ReaderStream;
use rocket::serde::json::Json;
use rocket::{
    data::{Data, ToByteUnit},
    fairing::AdHoc,
    http::{ContentType, Status},
    response::content,
    State,
};
use rocket::{Build, Request, Response};
use semver::Version;
use serde_json::json;
use storage::StorageMode;
use zip::ZipArchive;

use crate::auth::{ReadAccess, WriteAccess};
use crate::config::Config;
use crate::error::{ApiErrorContext, ApiErrorStatus, Error};
use crate::search::SearchBackend;
use crate::storage::{GcsStorage, LocalStorage, StorageBackend, StorageOutput};

#[cfg(feature = "s3-storage")]
use crate::storage::S3Storage;

#[get("/")]
fn root() -> content::RawJson<serde_json::Value> {
    content::RawJson(json!({
        "message": "Wally Registry is up and running!",
    }))
}

#[get("/v1/package-contents/<scope>/<name>/<version>")]
async fn package_contents(
    storage: &State<Box<dyn StorageBackend>>,
    _read: Result<ReadAccess, Error>,
    scope: String,
    name: String,
    version: String,
    _cli_version: Result<WallyVersion, Error>,
) -> Result<(ContentType, ReaderStream![StorageOutput]), Error> {
    _read?;
    _cli_version?;

    let package_name = PackageName::new(scope, name)
        .context("error parsing package name")
        .status(Status::BadRequest)?;
    let version: Version = version
        .parse()
        .context("error parsing version")
        .status(Status::BadRequest)?;
    let package_id = PackageId::new(package_name, version);

    match storage.read(&package_id).await.map(ReaderStream::one) {
        Ok(stream) => Ok((ContentType::GZIP, stream)),
        Err(e) => Err(e).status(Status::NotFound),
    }
}

#[get("/v1/package-metadata/<scope>/<name>")]
async fn package_info(
    index: &State<PackageIndex>,
    _read: Result<ReadAccess, Error>,
    scope: String,
    name: String,
) -> Result<Json<serde_json::Value>, Error> {
    _read?;

    let package_name = PackageName::new(scope, name)
        .context("error parsing package name")
        .status(Status::BadRequest)?;

    let metadata = &*index.get_package_metadata(&package_name)?;

    Ok(Json(serde_json::to_value(metadata)?))
}

#[get("/v1/package-search?<query>")]
async fn package_search(
    search_backend: &State<RwLock<SearchBackend>>,
    _read: Result<ReadAccess, Error>,
    query: String,
) -> Result<Json<serde_json::Value>, Error> {
    _read?;

    if let Ok(search_backend) = search_backend.read() {
        let result = search_backend.search(&query)?;
        Ok(Json(serde_json::to_value(result)?))
    } else {
        Err(
            format_err!("Unexpected error during search. Try again later.")
                .status(Status::InternalServerError),
        )
    }
}

#[post("/v1/publish", data = "<data>")]
async fn publish(
    storage: &State<Box<dyn StorageBackend>>,
    search_backend: &State<RwLock<SearchBackend>>,
    index: &State<PackageIndex>,
    authorization: Result<WriteAccess, Error>,
    _cli_version: Result<WallyVersion, Error>,
    data: Data<'_>,
) -> Result<Json<serde_json::Value>, Error> {
    _cli_version?;
    let authorization = authorization?;

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

    if !authorization.can_write_package(&package_id, &index)? {
        return Err(format_err!(
            "you do not have permission to write in scope {}",
            package_id.name().scope()
        )
        .status(Status::Unauthorized));
    }

    // If a user can write but isn't in the scope owner file then we should add them!
    if let WriteAccess::Github(github_info) = authorization {
        let user_id = github_info.id();
        let scope = package_id.name().scope();

        if !index.is_scope_owner(scope, user_id)? {
            index.add_scope_owner(scope, user_id)?;
        }
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

    if let Ok(mut search_backend) = search_backend.try_write() {
        // TODO: Recrawling the whole index for each publish is very wasteful!
        // Eventually this will get too expensive and we should only add the new package.
        search_backend.crawl_packages(&index)?;
    }

    Ok(Json(json!({
        "message": "Package published successfully!"
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

pub fn server(figment: Figment) -> rocket::Rocket<Build> {
    let config: Config = figment.extract().expect("could not read configuration");

    println!("Using authentication mode: {:?}", config.auth);

    println!("Using storage backend: {:?}", config.storage);
    let storage_backend: Box<dyn StorageBackend> = match config.storage {
        StorageMode::Local { path } => Box::new(LocalStorage::new(path)),
        StorageMode::Gcs { bucket, cache_size } => {
            Box::new(configure_gcs(bucket, cache_size).unwrap())
        }
        #[cfg(feature = "s3-storage")]
        StorageMode::S3 { bucket, cache_size } => {
            Box::new(configure_s3(bucket, cache_size).unwrap())
        }
    };

    println!("Cloning package index repository...");
    let package_index = PackageIndex::new_temp(&config.index_url, config.github_token).unwrap();

    println!("Initializing search backend...");
    let search_backend = SearchBackend::new(&package_index).unwrap();

    rocket::custom(figment)
        .mount(
            "/",
            routes![
                root,
                package_contents,
                publish,
                package_info,
                package_search,
                cors_options,
            ],
        )
        .manage(storage_backend)
        .manage(package_index)
        .manage(RwLock::new(search_backend))
        .attach(AdHoc::config::<Config>())
        .attach(Cors)
}

fn configure_gcs(bucket: String, cache_size: Option<u64>) -> anyhow::Result<GcsStorage> {
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

    Ok(GcsStorage::new(client, cache_size))
}

#[cfg(feature = "s3-storage")]
fn configure_s3(bucket: String, cache_size: Option<u64>) -> anyhow::Result<S3Storage> {
    use std::env;

    use rusoto_core::{credential::ChainProvider, request::HttpClient, Region};

    use rusoto_s3::S3Client;

    let client = S3Client::new_with(
        HttpClient::new()?,
        ChainProvider::new(),
        Region::Custom {
            name: env::var("AWS_REGION_NAME").unwrap_or_else(|_| "us-east-1".to_string()),
            endpoint: env::var("AWS_REGION_ENDPOINT")?,
        },
    );

    Ok(S3Storage::new(client, bucket, cache_size))
}

struct Cors;

#[rocket::async_trait]
impl Fairing for Cors {
    fn info(&self) -> Info {
        Info {
            name: "Add CORS headers to responses",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
        response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(Header::new("Access-Control-Allow-Methods", "GET"));
        response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
    }
}

#[options("/<_..>")]
fn cors_options() -> Result<(), Error> {
    Ok(())
}

struct WallyVersion;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for WallyVersion {
    type Error = Error;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let config = request
            .guard::<&State<Config>>()
            .await
            .expect("Failed to load config");

        let minimum_version = match &config.minimum_wally_version {
            Some(version) => version,
            None => return Outcome::Success(WallyVersion),
        };

        let version = match request.headers().get_one("Wally-Version") {
            Some(version) => version,
            None => {
                return format_err!(
                    "Wally version header required. Try upgrading your wally installation."
                )
                .status(Status::UpgradeRequired)
                .into();
            }
        };

        let version = match Version::parse(version) {
            Ok(version) => version,
            Err(err) => {
                return format_err!("Failed to parse wally version header: {}", err)
                    .status(Status::BadRequest)
                    .into();
            }
        };

        if &version < minimum_version {
            format_err!(
                "This registry requires Wally {} (you are using {})",
                minimum_version,
                version
            )
            .status(Status::UpgradeRequired)
            .into()
        } else {
            Outcome::Success(WallyVersion)
        }
    }
}

#[launch]
fn rocket() -> _ {
    let figment = Figment::from(rocket::Config::default())
        .merge(Toml::file("Rocket.toml").nested())
        .merge(Env::prefixed("WALLY_").global());

    server(figment)
}
