#[macro_use] extern crate clap;
use clap::{App, Arg};

extern crate parody;// use parody::ParodyServer;
use iron::Request;

fn main() {
    let matches = App::new("parody-server")
        .version("0.1")
        .about("Saves responses from remote server")
        .arg(Arg::with_name("target-url")
             .value_name("TARGET_URL")
             .help("a proxy we forward requests to"))
        .arg(Arg::with_name("storage-dir")
             .value_name("STORAGE_DIR")
             .help("where to store requests we make"))
        .get_matches();

}
