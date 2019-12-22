use std::{io::Cursor, str::FromStr};
extern crate env_logger;
extern crate regex;
extern crate serde_yaml;
extern crate tempfile;
extern crate url;
use super::*;

use crate::error::Error;
use config::Config;
use regex::Regex;
use std::{io::Read, path::Path};

const REQUEST_REGEX: &'static str = r"^(?:(?P<method>[A-Z]+) )?(?P<url>.*)$";
const DEFAULT_METHOD: &'static str = "GET";

fn get_method<'a>(req: &'a str) -> Option<&'a str> {
    let regex = Regex::new(REQUEST_REGEX).unwrap();
    let captures = regex.captures(req)?;
    captures.name("method").map(|method| method.as_str())
}

impl ParodyRequest for &str {
    fn get_url(&self) -> url::Url {
        url::Url::from_str(*self).unwrap()
    }

    fn get_method(&self) -> String {
        Regex::new(REQUEST_REGEX)
            .unwrap()
            .captures(self)
            .and_then(|captures| captures.name("method"))
            .map(|method| method.as_str().to_string())
            .unwrap_or_else(|| DEFAULT_METHOD.to_string())
    }
}

type TestResponse<'a, T, U> = (u16, T, U);

impl<'a, T, U: 'static> ParodyResponse for TestResponse<'a, T, U>
where
    T: AsRef<[(&'a str, &'a str)]>,
    U: Read,
{
    fn get_headers(&self) -> Vec<(String, String)> {
        self.1
            .as_ref()
            .iter()
            .map(|(header, value)| (String::from(*header), String::from(*value)))
            .collect()
    }

    fn get_body_reader(&mut self) -> &mut dyn Read {
        &mut self.2
    }

    fn get_status(&self) -> u16 {
        self.0
    }
}

header! { (XTestData, "X-Test-Data") => [String] }

#[test]
fn test_load_should_return_exactly_same_result_as_was_saved() {
    let storage_path = tempfile::tempdir().expect("Cannot create storage path");

    let storage = Storage::new_with_config(
        &"https://example.com/",
        Config::default().with_root_dir(storage_path.path().to_owned()),
    )
    .unwrap();

    storage
        .save(&mut (
            500 as u16,
            &[
                ("Content-Type", "application/json"),
                ("X-Test-Data", "1234567890"),
            ],
            Cursor::new("{\"lorem\": \"ipsum\"}".as_bytes()),
        ))
        .expect("Cannot save request to storage");

    let response = storage.load().expect("Cannot load response");
    let mut expected_headers = iron::Headers::new();

    expected_headers.set(iron::headers::ContentType::json());
    expected_headers.set(XTestData("1234567890".to_owned()));
    assert_eq!(response.headers, expected_headers);

    let mut write_body = response.body.expect("Response should have a write body");
    let mut body = Vec::<u8>::new();
    let mut body_cursor = Cursor::new(body);

    write_body
        .write_body(&mut body_cursor)
        .expect("Cannot write body to a cursor");

    assert_eq!(*body_cursor.get_ref(), b"{\"lorem\": \"ipsum\"}".to_vec());

    assert_eq!(response.status.expect("Response does not have status"), iron::status::InternalServerError);
 }

#[test]
fn test_load_when_response_cached_should_return_response() {
    let storage = Storage::new_with_config(
        &"https://example.com/status200?headers=Content-Type:application%2Fjson",
        Config::default()
            .with_root_dir(get_test_files_path())
            .with_query("headers"),
    )
    .unwrap();

    let response = storage.load().unwrap();

    let mut expected_headers = iron::Headers::new();
    expected_headers.set(iron::headers::ContentType::json());
    assert_eq!(response.headers, expected_headers);

    let mut write_body = response.body.expect("Response should have a write body");
    let mut body = Vec::<u8>::new();
    let mut body_cursor = Cursor::new(body);

    write_body
        .write_body(&mut body_cursor)
        .expect("Cannot write body to a cursor");

    assert_eq!(*body_cursor.get_ref(), b"{\"lorem\": \"ipsum\"}".to_vec())
}

#[test]
fn test_load_when_response_not_cached_should_return_cache_miss() {
    let storage_root = tempfile::tempdir().unwrap();

    let error = Storage::new_with_config(
        &"https://example.com/",
        Config::default().with_root_dir(storage_root.path().into()),
    )
    .unwrap()
    .load()
    .expect_err("Load didn't exit with an error");

    match error {
        Error::CacheMiss => {}
        error => panic!("Unexpected error: {:?}", error),
    };
}

fn get_test_files_path() -> PathBuf {
    Path::new(file!()).parent().unwrap().join("test_files")
}

#[test]
fn test_load_when_there_is_no_status_file_should_return_cache_miss() {
    let storage = Storage::new_with_config(
        &"https://example.com/failures/missing-status/?query=value",
        Config::default().with_root_dir(get_test_files_path()),
    )
    .expect("Cannot create new storage with config");

    match storage.load() {
        Err(Error::CacheMiss) => {}
        _ => panic!("load didn't exit with CacheMiss"),
    };
}

#[test]
fn test_storage_new_should_create_storage_for_request() {
    let storage = Storage::new(&"file:///test/location");
}

// #[test]
// fn test_load_should_load_response_status_from_status_file() {
//     let storage_root = tempfile::tempdir().unwrap();
//     let storage = Storage::new_with_config(
//         &"https://example.com/some-path/?query=value",
//         Config::default().with_root_dir(storage_root.path().into()),
//     )
//     .expect("Cannot create new storage with config");

//     let storage_path = storage_root.path().join("example.com/some-path/");

//     std::fs::create_dir_all(&storage_path).expect(&format!(
//         "Cannot create storage path: {}",
//         &storage_path.to_string_lossy()
//     ));

//     let status_file = storage_path.join("GET.status");

//     let mut status: Vec<u8> = Vec::new();
//     File::create(&status_file)
//         .expect("Cannot create status file")
//         .write(b"405\n")
//         .expect("Canot save status file");

//     let response = storage.load().expect("Cannot load a response");
//     assert_eq!(response.status.unwrap(), iron::status::MethodNotAllowed);
// }

#[test]
fn test_save_should_save_response_status_in_status_file() {
    let storage_root = tempfile::tempdir().unwrap();

    let storage = Storage::new_with_config(
        &"https://example.com/some-path/?query=value",
        Config::default().with_root_dir(storage_root.path().into()),
    )
    .expect("Cannot create new storage with config");

    storage.save(&mut (
        403 as u16,
        &[],
        Cursor::new("Lorem ipsum dolor sit amet".as_bytes()),
    ));

    let status_path = storage_root.path().join("example.com/some-path/GET.status");

    let mut status_file = File::open(&status_path).expect("Cannot open status file");

    let mut status: Vec<u8> = Vec::new();
    status_file
        .read_to_end(&mut status)
        .expect("Cannot read status");

    assert_eq!(status, b"403\n".to_vec());
}

#[test]
fn test_save_should_save_response_body_in_body_file() {
    let storage_root = tempfile::tempdir().unwrap();

    let storage = Storage::new_with_config(
        &"https://example.com/some-path/?query=value",
        Config::default()
            .with_root_dir(storage_root.path().into())
            .with_query("query"),
    )
    .expect("Cannot create new storage with config");

    storage
        .save(&mut (
            200 as u16,
            &[],
            Cursor::new("Lorem ipsum dolor sit amet".as_bytes()),
        ))
        .expect("Cannot save request to storage");

    let headers_path = storage_root
        .path()
        .join("example.com/some-path/:PARODY-QUERY/query=value/GET.body");

    let mut body_file = std::fs::File::open(&headers_path).expect(&format!(
        "Cannot open file at: {}",
        headers_path.to_str().unwrap()
    ));

    let mut body: Vec<u8> = Vec::new();
    body_file
        .read_to_end(&mut body)
        .expect("Cannot read body file");

    assert_eq!(body, "Lorem ipsum dolor sit amet".as_bytes());
}

#[test]
fn test_save_should_save_response_headers_in_headers_file() {
    let storage_root = tempfile::tempdir().unwrap();

    let storage = Storage::new_with_config(
        &"https://example.com/some-path/?query=value",
        Config::default()
            .with_root_dir(storage_root.path().into())
            .with_query("query"),
    )
    .expect("Cannot create new storage with config");
    storage
        .save(&mut (200 as u16, &[("Authorization", "Bearer")], Cursor::new(&[])))
        .expect("Cannot save request to storage");

    let headers_path = storage_root
        .path()
        .join("example.com/some-path/:PARODY-QUERY/query=value/GET.headers.yaml");

    let headers_file = std::fs::File::open(&headers_path).expect(&format!(
        "Cannot open file at: {}",
        headers_path.to_str().unwrap()
    ));

    let headers: Vec<(String, String)> = serde_yaml::from_reader(headers_file).expect(&format!(
        "Cannot read yaml file at: {}",
        headers_path.to_str().unwrap()
    ));

    assert_eq!(
        headers,
        vec![("Authorization".to_string(), "Bearer".to_string())]
    );
}

#[test]
fn test_save_should_save_request_body_in_body_file() {}

#[test]
fn test_save_should_save_request() {}

#[test]
fn test_get_response_storage_dir_when_request_has_path_should_return_target_path() {
    assert_eq!(
        get_response_storage_dir(&"file:///test/location", &Config::default()).unwrap(),
        PathBuf::from_str(":NO-HOST/test/location").unwrap()
    );
}

#[test]
fn test_get_response_storage_dir_when_request_has_query_should_not_include_them_in_target_path() {
    assert_eq!(
        get_response_storage_dir(&"file:///test/location", &Config::default()).unwrap(),
        PathBuf::from_str(":NO-HOST/test/location").unwrap()
    );
}

#[test]
fn init() {
    let _ = env_logger::builder().is_test(true).try_init();
}

#[test]
fn test_get_response_storage_dir_when_request_has_query_should_include_query_from_config() {
    assert_eq!(
        get_response_storage_dir(
            &"https://example.com/test string/unicode-α?query&query=&query-arg=value",
            &Config::default().with_query("query")
        )
        .unwrap(),
        PathBuf::from_str("example.com/test string/unicode-α/:PARODY-QUERY/query/query").unwrap()
    );
}

#[test]
fn test_get_response_storage_dir_when_request_has_host_should_return_target_path_with_host() {
    assert_eq!(
        get_response_storage_dir(&"https://example.com", &Config::default()).unwrap(),
        PathBuf::from_str("example.com").unwrap()
    );
}

#[test]
fn test_get_response_storage_dir_all_features() {
    assert_eq!(
        get_response_storage_dir(
            &"https://example.com/test string/unicode-α?query&query=&query-arg=value",
            &Config::default()
        )
        .unwrap(),
        PathBuf::from_str("example.com/test string/unicode-α/").unwrap()
    );
}
