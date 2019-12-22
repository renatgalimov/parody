extern crate percent_encoding;
extern crate serde_json;
extern crate serde_yaml;
extern crate url;
use crate::{error::Error, request::ParodyRequest, response::ParodyResponse, result::Result};
pub use config::Config;
use std::{
    borrow::Cow,
    fs::File,
    io::{Read, Write},
    path::PathBuf,
};

use crate::storage::error::StorageError;

mod config;
mod error;
#[cfg(test)]
mod test;

const QUERY_SEPARATOR: &'static str = ":PARODY-QUERY";
const NO_HOST_DIR: &'static str = ":NO-HOST";
const HEADERS_FILE_EXTENSION: &'static str = ".headers.yaml";
const BODY_FILE_EXTENSION: &'static str = ".body";
const STATUS_FILE_EXTENSION: &'static str = ".status";

/// Stores a request data
#[derive(Default)]
pub struct Storage {
    config: config::Config,
    /// A directory relative to root dir from the config where we store request details
    storage_path_relative: PathBuf,
    method: String,
}

struct CachedBodyWriter {
    body_file_path: PathBuf,
}

impl iron::response::WriteBody for CachedBodyWriter {
    fn write_body(&mut self, res: &mut dyn Write) -> std::io::Result<()> {
        let mut body_file = match File::open(&self.body_file_path) {
            Ok(body_file) => body_file,
            Err(error) => match error.kind() {
                std::io::ErrorKind::NotFound => return Ok(()),
                _ => return Err(error),
            },
        };

        std::io::copy(&mut body_file, res)?;

        Ok(())
    }
}

impl Storage {
    pub fn new<T: ParodyRequest>(req: &T) -> Result<Self> {
        Self::new_with_config(req, Config::default())
    }

    pub fn new_with_config<T: ParodyRequest>(req: &T, config: Config) -> Result<Self> {
        Ok(Self {
            storage_path_relative: get_response_storage_dir(req, &config)?,
            config: config,
            method: req.get_method(),
        })
    }

    pub fn get_absolute_storage_path(&self) -> PathBuf {
        let mut current_directory = match std::fs::canonicalize(".") {
            Ok(path) => path,
            Err(error) => panic!(
                "Failed to make absolute path for current directory: {}",
                error
            ),
        };

        current_directory.push(&self.storage_path_relative);
        current_directory
    }

    fn get_status_file_path(&self) -> PathBuf {
        self.get_absolute_storage_path()
            .join(self.method.clone() + STATUS_FILE_EXTENSION)
    }

    fn save_status<T: ParodyResponse>(&self, resp: &T) -> Result<()> {
        let status = resp.get_status();
        let status_file_path = self.get_status_file_path();

        writeln!(&mut File::create(&status_file_path)?, "{}", status)?;
        Ok(())
    }

    fn get_headers_file_path(&self) -> PathBuf {
        self.get_absolute_storage_path()
            .join(self.method.clone() + HEADERS_FILE_EXTENSION)
    }

    fn save_headers<T: ParodyResponse>(&self, resp: &T) -> Result<()> {
        let headers = resp.get_headers();

        let mut headers_file_path = self.get_headers_file_path();
        match serde_yaml::to_writer(File::create(&headers_file_path)?, &headers) {
            Ok(_) => {
                trace!(target: "storage", "Saved headers to {}", &headers_file_path.as_os_str().to_string_lossy())
            }
            Err(error) => {
                warn!(target: "storage", "{}", error);
                return Err(error.into());
            }
        }

        Ok(())
    }

    fn get_body_file_path(&self) -> PathBuf {
        self.get_absolute_storage_path()
            .join(self.method.clone() + BODY_FILE_EXTENSION)
    }

    fn save_body<T: ParodyResponse>(&self, resp: &mut T) -> Result<()> {
        std::io::copy(
            resp.get_body_reader(),
            &mut File::create(&self.get_body_file_path())?,
        );

        Ok(())
    }

    pub fn save<T: ParodyResponse>(&self, resp: &mut T) -> Result<()> {
        let storage_path = self.get_absolute_storage_path();

        debug!(target: "storage", "Saving response to: {}", &storage_path.to_string_lossy());
        std::fs::create_dir_all(&storage_path)?;
        self.save_body(resp)?;
        self.save_headers(resp)?;
        self.save_status(resp)?;
        info!(target: "storage", "Saved response to: {}", &storage_path.to_string_lossy());
        Ok(())
    }

    fn load_headers(&self) -> Result<iron::Headers> {
        let headers_file_path = self.get_headers_file_path();
        debug!(target: "storage", "Loading headers from: {}", headers_file_path.to_string_lossy());
        let mut headers = iron::headers::Headers::new();
        let mut headers_file = match File::open(&headers_file_path) {
            Ok(file) => file,
            Err(error) => match error.kind() {
                std::io::ErrorKind::NotFound => {
                    debug!(target: "storage", "Headers file not found. Returning empty headers.");
                    return Ok(headers);
                }
                _ => return Err(error.into()),
            },
        };

        let headers_raw: Vec<(String, String)> = serde_yaml::from_reader(headers_file)?;

        for (name, value) in headers_raw {
            headers.append_raw(name, value.as_bytes().to_vec());
        }

        Ok(headers)
    }

    fn load_status(&self) -> std::result::Result<iron::status::Status, StorageError> {
        let status_file_path = self.get_status_file_path();

        debug!(target: "storage", "Loading status from: {}", status_file_path.to_string_lossy());

        let mut status_raw = String::new();
        File::open(&status_file_path)
            .map_err(|error| match error.kind() {
                std::io::ErrorKind::NotFound => StorageError::StatusFileNotFound,
                _ => {
                    trace!(target: "storage", "Cannot load status: {:?}", error);
                    StorageError::Common(error.into())
                }
            })?
            .read_to_string(&mut status_raw);
        let status_raw = status_raw.trim();

        Ok(iron::status::Status::from_u16(status_raw.parse()?))
    }

    fn load_body(&self) -> CachedBodyWriter {
        CachedBodyWriter {
            body_file_path: self.get_body_file_path(),
        }
    }

    pub fn load(&self) -> Result<iron::Response> {
        let storage_path = self.get_absolute_storage_path();

        if !storage_path.exists() {
            trace!(target: "storage", "Storage dir doesn't exist: {:?}", storage_path.to_string_lossy());
            return Err(Error::CacheMiss);
        } else {
            trace!(target: "storage", "Storage dir exists: {:?}", storage_path.to_string_lossy());
        };

        let mut response = iron::Response::with(iron::status::Ok);

        response.status = match self.load_status() {
            Ok(status) => Some(status),
            Err(StorageError::StatusFileNotFound) => {
                trace!(target: "storage", "Status file not found in cache");
                return Err(Error::CacheMiss);
            }
            Err(StorageError::Common(common_error)) => return Err(common_error.into()),
        };

        response.headers = self.load_headers()?;
        response.body = Some(Box::new(self.load_body()));

        Ok(response)
    }
}

fn percent_encode_slash(input: &str) -> String {
    input.replace("/", "%2F")
}

/// Where to store response details
fn get_response_storage_dir<T: ParodyRequest>(req: &T, config: &Config) -> Result<PathBuf> {
    let url: url::Url = req.get_url();

    let mut target_path = config
        .get_root_dir()
        .join(url.host_str().unwrap_or(NO_HOST_DIR));

    if let Some(segments) = url.path_segments() {
        for dir in segments {
            let decoded_str = percent_encoding::percent_decode_str(dir).decode_utf8()?;
            target_path.push(percent_encode_slash(&decoded_str));
        }
    };

    let mut query: Vec<(Cow<str>, Cow<str>)> = url
        .query_pairs()
        .filter(|(arg, _value)| config.should_use_query_in_path(arg))
        .collect();

    if !query.is_empty() {
        target_path.push(QUERY_SEPARATOR);
        query.sort();
        for (argument, value) in query {
            let dir_name = if !value.is_empty() {
                format!("{}={}", argument.as_ref(), value.as_ref())
            } else {
                argument.to_string()
            };

            target_path.push(percent_encode_slash(&dir_name));
        }
    }

    Ok(target_path)
}
