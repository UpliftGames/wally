use std::process::exit;

use structopt::StructOpt;

use libwally::Args;

fn main() {
    let args = Args::from_args();

    let log_filter = match args.global.verbosity {
        0 => "libwally=info",
        1 => "libwally=debug",
        2 => "libwally=trace",
        _ => "trace",
    };

    let log_env = env_logger::Env::default().default_filter_or(log_filter);

    env_logger::Builder::from_env(log_env)
        .format_module_path(false)
        .format_timestamp(None)
        // Indent following lines equal to the log level label, like `[ERROR] `
        .format_indent(Some(8))
        .init();

    if let Err(err) = args.run() {
        eprintln!("{:?}", err);
        exit(1);
    }
}
