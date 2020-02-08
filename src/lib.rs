//! Web-service mocking library and server

extern crate url;
#[macro_use]
extern crate log;
extern crate http;
extern crate iron;
extern crate regex;
extern crate serde_json;
#[cfg(test)]
#[macro_use]
extern crate hyper;

mod cache_middleware;
mod error;
mod forward_middleware;
mod log_middleware;
mod request;
mod response;
mod result;
pub mod storage;

pub use crate::{
    cache_middleware::{CacheMiddleware, ResponseCache},
    forward_middleware::{ForwardMiddleware, ProxyResponse},
};
use crate::{
    error::{Error, UtilError},
    forward_middleware::ProxyLoad,
    request::{ParodyRequest, RequestLogItem},
    result::Result,
};
use hyper::net::HttpListener;
use std::{
    net::{IpAddr, SocketAddr},
    path::{Path, PathBuf},
    str::FromStr,
    sync::{Arc, Mutex},
};

fn handle_request(req: &mut iron::Request) -> iron::IronResult<iron::Response> {
    trace!("Handling request: {} {}", req.method, req.url);

    req.extensions
        .get::<persistent::Write<RequestStorage>>()
        .map(|a_storage| {
            a_storage
                .lock()
                .unwrap()
                .push(Box::new(RequestLogItem::from(req as &iron::Request)));
            debug!("Logged request: {} {}", req.method, req.url);
        });

    let proxy = req
        .extensions
        .remove::<ProxyResponse>()
        .expect("Proxy response should be always found");

    let response_storage = req
        .extensions
        .get::<ResponseCache>()
        .expect("Response cache should be always found");

    match response_storage.load() {
        Ok(cached_response) => {
            warn!("Found cached response for: {} {}", req.method, req.url);
            return Ok(cached_response.into());
        }
        Err(Error::CacheMiss) => {
            debug!("Cache miss for: {} {}", req.method, req.url);
        }
        Err(error) => {
            warn!("Cannot load response from cache: {}", error);
            return Err(iron::IronError::new(
                error,
                iron::status::InternalServerError,
            ));
        }
    };

    let mut response = match proxy.load() {
        Ok(upstream_response) => upstream_response,
        Err(error) => {
            return Err(iron::IronError::new(
                error,
                iron::status::InternalServerError,
            ))
        }
    };

    response_storage
        .save(&mut response)
        .map_err(|error| iron::IronError::new(error, iron::status::InternalServerError))?;

    Ok(iron::Response::with(iron::status::Ok))
}

type Requests = Vec<Box<dyn ParodyRequest + Send + Sync>>;
struct RequestStorage;
impl iron::typemap::Key for RequestStorage {
    type Value = Requests;
}

/// Represents a running Parody server
#[derive(Debug)]
pub struct Parody {
    listener: iron::Listening,
    a_storage: Option<Arc<Mutex<Requests>>>,
}

/// Stops the listener on destruction
impl Drop for Parody {
    fn drop(&mut self) {
        match self.listener.close() {
            Ok(_) => {}
            Err(error) => error!("Cannot close listener: {}", error),
        }
    }
}

impl Parody {
    pub fn ip(&self) -> IpAddr {
        self.listener.socket.ip()
    }

    pub fn port(&self) -> u16 {
        self.listener.socket.port()
    }

    pub fn requests(&self) -> Option<Arc<Mutex<Requests>>> {
        self.a_storage.clone().map(|a_storage| a_storage.clone())
    }
}

fn get_storage_directory(upstream_url: &str, file: &str) -> Result<PathBuf> {
    let url = url::Url::from_str(upstream_url)?;
    let domain = url.domain().ok_or(UtilError::DomainMissing)?;

    let parent_path = Path::new(file)
        .parent()
        .ok_or(UtilError::InvalidCurrentFilePath)?;

    Ok(parent_path.join(domain))
}

/// Starts a server listening at random port at localhost for a directory relative to the given file
///
/// # Example
/// ```
/// use std::path::Path;
/// let parody = parody::start_relative_to_file("https://example.com", file!()).unwrap();
/// println!("PARODY_IP={}", parody.ip());
/// println!("PARODY_PORT={}", parody.port());
/// ```
pub fn start_relative_to_file(upstream_url: &str, file: &str) -> Result<Parody> {
    start(
        url::Url::from_str(upstream_url)?,
        storage::Config::default().with_root_dir(get_storage_directory(upstream_url, file)?),
    )
}

/// Starts a server at random port at localhost
///
/// With this function you can configure your Parody server precisely
///
/// # Example
/// ```
/// use std::str::FromStr;
/// use std::path::Path;
/// let storage_config = parody::storage::Config::default().with_root_dir(Path::new("/tmp/parody/example.com").to_owned());
/// let upstream_url = url::Url::from_str("http://example.com").unwrap();
/// let parody = parody::start(upstream_url, storage_config).unwrap();
/// println!("PARODY_IP={}", parody.ip());
/// println!("PARODY_PORT={}", parody.port());
/// ```
pub fn start(upstream_url: url::Url, storage_config: storage::Config) -> Result<Parody> {
    let mut chain = iron::Chain::new(handle_request);
    chain.link_before(CacheMiddleware::new().with_storage_config(storage_config));
    chain.link_before(ForwardMiddleware::new(upstream_url));

    let a_storage = Arc::new(Mutex::new(Vec::new()));

    chain.link(persistent::Write::<RequestStorage>::both(a_storage.clone()));

    let listener: HttpListener = HttpListener::new(SocketAddr::from(([127, 0, 0, 1], 0)))?;

    iron::Iron::new(chain)
        .listen(listener, iron::Protocol::http())
        .map(|listener| Parody {
            listener: listener,
            a_storage: Some(a_storage),
        })
        .map_err(|err| err.into())
}
