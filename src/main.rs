#[macro_use]
extern crate clap;
extern crate iron;
extern crate parody; // use parody::ParodyServer;

#[macro_use]
extern crate log;

use clap::{App, Arg};
use std::str::FromStr;

fn main() {
    env_logger::init();

    let matches = App::new("parody-server")
        .version("0.1")
        .about("Saves responses from remote server")
        .arg(
            Arg::with_name("target-url")
                .required(true)
                .value_name("TARGET_URL")
                .help("a proxy we forward requests to"),
        )
        .arg(
            Arg::with_name("storage-dir")
                .required(true)
                .value_name("STORAGE_DIR")
                .help("where to store requests we make"),
        )
        .get_matches();

    let target_url = match url::Url::from_str(
        matches
            .value_of("target-url")
            .expect("Target URL should be supplied"),
    ) {
        Ok(url) => url,
        Err(error) => {
            eprintln!("Target URL is invalid: {}", error);
            std::process::exit(2);
        }
    };

    let storage_dir_path = std::path::Path::new(
        matches
            .value_of("storage-dir")
            .expect("Storage dir should be supplied"),
    );

    if !storage_dir_path.exists() {
        if let Err(error) = std::fs::create_dir_all(storage_dir_path) {
            eprintln!("Cannot create target directory: {}", error);
            std::process::exit(2);
        } else {
            let abs_path = std::fs::canonicalize(storage_dir_path).expect("Should handle abs path");
            info!(
                "Created target directory in: {}",
                abs_path.to_string_lossy()
            );
        }
    } else {
        debug!("Storage dir path already exists");
    }

    let listener = match parody::start_default(target_url, storage_dir_path.to_owned()) {
        Ok(listener) => {
            println!("PARODY_HOST={}", listener.socket.ip());
            println!("PARODY_PORT={}", listener.socket.port());
            listener
        }
        Err(error) => {
            eprintln!("Cannot start server: {}", error);
            std::process::exit(2);
        }
    };
}
