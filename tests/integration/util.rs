use std::collections::BTreeMap;
use std::path::Path;

use fs_err as fs;
use insta::assert_snapshot;
use serde::{Deserialize, Serialize};

use crate::temp_project::TempProject;

#[macro_export]
macro_rules! assert_dir_snapshot {
    ( $path:expr ) => {
        let result = crate::util::read_path($path).unwrap();
        insta::assert_yaml_snapshot!(result);
    };
}

#[macro_export]
macro_rules! open_test_project {
    ( $path:expr ) => {
        TempProject::new(Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test-projects/",
            $path
        )))
        .unwrap()
    };
}

pub fn snapshot_manifest(project: &TempProject) {
    assert_snapshot!(fs::read_to_string(project.path().join("wally.toml")).unwrap())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Entry {
    File(String),
    Dir(BTreeMap<String, Entry>),
}

pub fn read_path(path: &Path) -> anyhow::Result<Entry> {
    let meta = fs_err::metadata(path)?;

    if meta.is_dir() {
        let children = fs_err::read_dir(path)?
            .map(|dir_entry| {
                let path = dir_entry?.path();
                let name = path.file_name().unwrap().to_str().unwrap().to_owned();
                let entry = read_path(&path)?;

                Ok((name, entry))
            })
            .collect::<anyhow::Result<BTreeMap<String, Entry>>>()?;

        Ok(Entry::Dir(children))
    } else {
        let contents = fs_err::read_to_string(path)?;
        Ok(Entry::File(contents))
    }
}
