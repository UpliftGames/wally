//! Contains utilities for writing tests against packages.

use std::{
    collections::BTreeMap,
    io::{Cursor, Write},
};

use zip::write::{FileOptions, ZipWriter};

use crate::{
    manifest::{Manifest, Package, Realm},
    package_contents::PackageContents,
    package_id::PackageId,
    package_location::PackageLocation,
    package_req::PackageReq,
};

pub struct PackageBuilder {
    manifest: Manifest,
    files: BTreeMap<String, String>,
}

impl PackageBuilder {
    pub fn new<S>(identity: S) -> Self
    where
        S: AsRef<str>,
    {
        let id: PackageId = identity.as_ref().parse().expect("invalid PackageId");
        let (name, version) = id.into_parts();

        let manifest = Manifest {
            package: Package {
                name,
                version,
                registry: String::new(),
                realm: Realm::Shared,
                description: None,
                license: None,
                authors: Vec::new(),
                include: Vec::new(),
                exclude: Vec::new(),
            },
            place: Default::default(),
            dependencies: Default::default(),
            server_dependencies: Default::default(),
            dev_dependencies: Default::default(),
        };

        Self {
            manifest,
            files: BTreeMap::new(),
        }
    }

    pub fn with_realm(mut self, realm: Realm) -> Self {
        self.manifest.package.realm = realm;
        self
    }

    pub fn with_dep<A, R>(mut self, alias: A, package_req: R) -> Self
    where
        A: Into<String>,
        R: AsRef<str>,
    {
        let req: PackageReq = package_req.as_ref().parse().expect("invalid PackageReq");

        self.manifest
            .dependencies
            .insert(alias.into(), PackageLocation::Registry(req));
        self
    }

    pub fn with_server_dep<A, R>(mut self, alias: A, package_req: R) -> Self
    where
        A: Into<String>,
        R: AsRef<str>,
    {
        let req: PackageReq = package_req.as_ref().parse().expect("invalid PackageReq");

        self.manifest
            .server_dependencies
            .insert(alias.into(), PackageLocation::Registry(req));
        self
    }

    pub fn with_file<P, C>(mut self, path: P, contents: C) -> Self
    where
        P: Into<String>,
        C: Into<String>,
    {
        self.files.insert(path.into(), contents.into());
        self
    }

    pub fn into_manifest(self) -> Manifest {
        self.manifest
    }

    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    pub fn contents(&self) -> PackageContents {
        let mut buffer = Vec::new();
        let mut archive = ZipWriter::new(Cursor::new(&mut buffer));

        for (path, contents) in &self.files {
            archive.start_file(path, FileOptions::default()).unwrap();
            archive.write_all(contents.as_bytes()).unwrap();
        }

        let encoded_manifest = toml::to_string_pretty(&self.manifest).unwrap();
        archive
            .start_file("wally.toml", FileOptions::default())
            .unwrap();
        archive.write_all(encoded_manifest.as_bytes()).unwrap();

        archive.finish().unwrap();
        drop(archive);

        let contents = PackageContents::from_buffer(buffer);
        contents
    }

    pub fn package(self) -> (Manifest, PackageContents) {
        let contents = self.contents();
        (self.manifest, contents)
    }
}
