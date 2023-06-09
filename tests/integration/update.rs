use crate::temp_project::TempProject;
use libwally::{Args, GlobalOptions, InstallSubcommand, Subcommand, UpdateSubcommand};
use std::path::Path;

#[test]
fn test_test() {
    let source_project = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/test-projects/diamond-graph/root/0.1.0"
    ));

    let project = TempProject::new(&source_project).unwrap();

    Args {
        global: GlobalOptions {
            test_registry: true,
            ..Default::default()
        },
        subcommand: Subcommand::Install(InstallSubcommand {
            project_path: project.path().to_owned(),
        }),
    }
    .run()
    .unwrap();

    let args = Args {
        global: GlobalOptions {
            test_registry: true,
            ..Default::default()
        },
        subcommand: Subcommand::Update(UpdateSubcommand {
            project_path: project.path().to_owned(),
            target_packages: Vec::new(),
        }),
    };

    args.run().unwrap();
    // assert_dir_snapshot!(&project.path());
}
