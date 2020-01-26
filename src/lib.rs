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
mod storage;

#[cfg(test)]
mod test;

pub use crate::{
    cache_middleware::{CacheMiddleware, ResponseCache},
    forward_middleware::{ForwardMiddleware, ProxyResponse},
};
use crate::{
    error::Error,
    forward_middleware::ProxyLoad,
    request::{ParodyRequest, RequestLogItem},
    result::Result,
};
use hyper::net::HttpListener;
use std::{
    net::{IpAddr, SocketAddr},
    path::PathBuf,
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

#[derive(Debug)]
pub struct Parody {
    listener: iron::Listening,
    a_storage: Option<Arc<Mutex<Requests>>>,
}

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

pub fn start(upstream_url: &str, storage_path: &str) -> Result<Parody> {
    start_default(
        url::Url::from_str(upstream_url)?,
        PathBuf::from(storage_path),
    )
}

pub fn start_default(upstream_url: url::Url, storage_path: PathBuf) -> Result<Parody> {
    let mut chain = iron::Chain::new(handle_request);
    chain.link_before(
        CacheMiddleware::new().with_root_dir(
            storage_path.join(
                upstream_url
                    .host_str()
                    .expect("Should always have host string"),
            ),
        ),
    );
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
