use std::path::Path;

use libwally::{Args, ManifestToJsonSubcommand, Subcommand};

#[test]
fn minimal() {
    let project_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/test-projects/minimal"
    ));

    let args = Args {
        global: Default::default(),
        subcommand: Subcommand::ManifestToJson(ManifestToJsonSubcommand {
            project_path: project_path.to_owned(),
        }),
    };

    let _contents = args.run().unwrap();

    // TODO: make some assertions
}
