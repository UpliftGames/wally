use std::io::BufReader;
use std::path::PathBuf;

use anyhow::Context;
use fs_err::File;

use crate::manifest::Manifest;
use crate::package_id::PackageId;
use crate::package_req::PackageReq;
use crate::package_source::{PackageContents, PackageSource};

pub struct TestRegistry {
    path: PathBuf,
}

impl TestRegistry {
    pub fn new<P: Into<PathBuf>>(path: P) -> Self {
        Self { path: path.into() }
    }
}

impl PackageSource for TestRegistry {
    fn update(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn query(&self, package_req: &PackageReq) -> anyhow::Result<Vec<Manifest>> {
        // Each package has all of its versions stored in a folder based on its
        // scope and name.
        let mut package_path = self.path.clone();
        package_path.push("index");
        package_path.push(package_req.name().scope());
        package_path.push(package_req.name().name());

        // Construct a buffered file reader, with a nice error message in the
        // event of failure. We might want to return a structured error from
        // this method in the future to distinguish between general I/O errors
        // and a package not existing.
        let file = File::open(&package_path)
            .with_context(|| format!("could not open package {} from index", package_req.name()))?;
        let file = BufReader::new(file);

        // Read all of the manifests from the package file.
        //
        // Entries into the index are stored as JSON Lines. This block will
        // either parse all of the entries, or fail with a single error.
        let manifest_stream: Result<Vec<Manifest>, serde_json::Error> =
            serde_json::Deserializer::from_reader(file)
                .into_iter::<Manifest>()
                .filter(|manifest| {
                    if let Ok(manifest) = manifest {
                        package_req.matches(&manifest.package.name, &manifest.package.version)
                    } else {
                        true
                    }
                })
                .collect();

        let versions = manifest_stream.with_context(|| {
            format!(
                "could not parse package index entry for {}",
                package_req.name()
            )
        })?;

        Ok(versions)
    }

    fn download_package(&self, package_id: &PackageId) -> anyhow::Result<PackageContents> {
        let mut package_path = self.path.clone();
        package_path.push("contents");
        package_path.push(package_id.name().scope());
        package_path.push(package_id.name().name());
        package_path.push(format!("{}.zip", package_id.version()));

        let data = fs_err::read(&package_path)?;
        Ok(PackageContents::from_buffer(data))
    }
}
