extern crate url;
#[macro_use]
extern crate log;
extern crate http;
#[macro_use]
extern crate hyper;
extern crate iron;
extern crate regex;
extern crate serde_json;

mod cache_middleware;
mod error;
mod forward_middleware;
mod request;
mod response;
mod result;
mod storage;

pub use crate::{
    cache_middleware::{CacheMiddleware, ResponseCache},
    forward_middleware::{ForwardMiddleware, ProxyResponse},
};
use crate::{error::Error, forward_middleware::ProxyLoad, result::Result};
use std::path::PathBuf;

fn handle_response(req: &mut iron::Request) -> iron::IronResult<iron::Response> {
    trace!("Handling request: {} {}", req.method, req.url);

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
            return Ok(cached_response.into());
        }
        Err(Error::CacheMiss) => {
            debug!("Cache miss for: {} {}", req.method, req.url);
        }
        Err(error) => {
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

pub fn start_default(upstream_url: url::Url, storage_path: PathBuf) -> Result<iron::Listening> {
    let mut chain = iron::Chain::new(handle_response);
    chain.link_before(CacheMiddleware::new().with_root_dir(storage_path));
    chain.link_before(ForwardMiddleware::new(upstream_url));
    let listener: hyper::net::HttpListener =
        hyper::net::HttpListener::new(std::net::SocketAddr::from(([127, 0, 0, 1], 0)))?;

    iron::Iron::new(chain)
        .listen(listener, iron::Protocol::http())
        .map_err(|err| err.into())
}

// pub type RequestTree = HashMap<http::Method, HashMap<String, Vec<TestRequest>>>;

// #[derive(Copy, Clone)]
// pub struct SavedRequests;
// impl Key for SavedRequests {
//     type Value = RequestTree;
// }

// pub struct ParodyServer {
//     a_request_tree: Arc<Mutex<RequestTree>>,
// }

// #[derive(Debug, Clone)]
// pub struct CachedResponse {
//     method: http::Method,
//     url: url::Url,
// }

// impl Key for CachedResponse {
//     type Value = CachedResponse;
// }

// #[derive(Debug, Fail)]
// enum CacheError {
//     #[fail(display = "invalid base: (path={}, reason={})", _0, _1)]
//     InvalidBasePath(String, String),
// }

// impl CachedResponse {
//     // fn get_status(&self) -> &http::StatusCode {
//     //     &self.status
//     // }

//     // fn get_status_str(&self) -> &str {
//     //     self.get_status().as_str()
//     // }

//     fn get_path(&self) -> Vec<&str> {
//         self.url
//             .path_segments()
//             .expect("URL cannot be a base")
//             .collect()
//     }

//     fn get_path_str(&self) -> &str {
//         self.url.path()
//     }

//     /// Construct a response from the URL
//     ///
//     /// Base path is a mapping of URL request into a file system -
//     /// in a folder pointed by a base path we store all cached responses
//     /// from a resource.
//     ///
//     /// # Example
//     /// `example.com/some/path/!PARODY-QUERY/query=args/METHOD`
//     pub fn from_base_path(base_path: &str) -> Result<CachedResponse, Error> {
//         let regex = regex::Regex::new(
//             r"^(?P<base>[^/]+)/(?P<path>.*)/!PARODY-QUERY/(?P<query>.*)/(?P<method>[A-Z]+)$",
//         )
//         .expect("Invalid regex.");
//         let captures = regex
//             .captures(base_path)
//             .ok_or(CacheError::InvalidBasePath(
//                 base_path.to_owned(),
//                 "wrong path structure".to_owned(),
//             ))?;
//         let method = captures
//             .name("method")
//             .ok_or(CacheError::InvalidBasePath(
//                 base_path.to_owned(),
//                 "method is missing".to_owned(),
//             ))?
//             .as_str();
//         let url = captures
//             .name("base")
//             .ok_or(CacheError::InvalidBasePath(
//                 base_path.to_owned(),
//                 "base is missing".to_owned(),
//             ))?
//             .as_str();
//         let mut new_url: url::Url = url::Url::from_str(url)?;

//         Ok(CachedResponse {
//             method: reqwest::Method::from_str(method)?,
//             url: reqwest::Url::from_str(url)?,
//         })
//     }

//     pub fn from_request(req: &mut Request) -> CachedResponse {
//         CachedResponse {
//             method: http::Method::from_str(req.method.as_ref()).expect(&format!(
//                 "Cannot construct http::Method from: {}",
//                 req.method.as_ref()
//             )),
//             url: req.url.clone().into(),
//         }
//     }

//     pub fn response_files_base(&self) -> PathBuf {
//         let mut target_path = PathBuf::new();

//         for part in self.get_path() {
//             target_path.push(part);
//         }

//         target_path.push("!PARODY-QUERY");

//         let mut query: Vec<(Cow<str>, Cow<str>)> = self.url.query_pairs().collect();
//         query.sort();

//         for (argument, value) in query {
//             target_path.push(format!("{}={}", argument.as_ref(), value.as_ref()));
//         }

//         target_path.push(self.method.as_str());
//         target_path
//     }
// }

// #[derive(Debug, Clone)]
// pub struct ProxiedResponse {
//     status: http::StatusCode,
//     headers: http::HeaderMap,
// }

// impl Key for ProxiedResponse {
//     type Value = ProxiedResponse;
// }

// impl From<&reqwest::Response> for ProxiedResponse {
//     fn from(source: &reqwest::Response) -> ProxiedResponse {
//         ProxiedResponse {
//             status: source.status(),
//             headers: source.headers().clone(),
//         }
//     }
// }

// #[derive(Debug, Clone)]
// pub struct TestRequest {
//     method: http::Method,
//     path: String,
//     query: Vec<(String, String)>,
//     body: Vec<u8>,
// }

// impl Key for TestRequest {
//     type Value = TestRequest;
// }

// struct ParodyMiddleware;
// impl ParodyMiddleware {
//     /// Saves a request into the memory
//     ///
//     /// Useful for unit-tests when you need to
//     /// examine which requests did you do to a service
//     fn save_request(&self, req: &mut Request) -> IronResult<()> {
//         let mut saved_request = req
//             .extensions_mut()
//             .remove::<TestRequest>()
//             .expect("Current request extension is missing.");

//         req.body
//             .read_to_end(&mut saved_request.body)
//             .map_err(|err| IronError::new(err, Status::InternalServerError))?;

//         let a_saved_requests = req.get::<persistent::Write<SavedRequests>>().unwrap();

//         let saved_requests: &mut RequestTree = &mut a_saved_requests.as_ref().lock().unwrap();

//         saved_requests
//             .entry(
//                 reqwest::Method::from_str(req.method.as_ref())
//                     .expect(&format!("Unsupported method: {:?}", req.method)),
//             )
//             .or_insert_with(HashMap::new)
//             .entry(req.url.path().join("/"))
//             .or_insert_with(Vec::<TestRequest>::new)
//             .push(saved_request);

//         Ok(())
//     }
// }

// impl BeforeMiddleware for ParodyMiddleware {
//     fn before(&self, req: &mut Request) -> IronResult<()> {
//         self.save_request(req)?;
//         Ok(())
//     }

//     fn catch(&self, req: &mut Request, err: IronError) -> IronResult<()> {
//         self.save_request(req)?;
//         Err(err)
//     }
// }

// impl AfterMiddleware for ParodyMiddleware {
//     fn after(&self, _: &mut Request, res: Response) -> IronResult<Response> {
//         Ok(res)
//     }

//     fn catch(&self, _: &mut Request, err: IronError) -> IronResult<Response> {
//         Err(err)
//     }
// }

// impl ParodyServer {
//     pub fn new(handler: impl Handler) -> Self {
//         let a_request_tree = Arc::new(Mutex::new(RequestTree::new()));
//         let mut chain = Chain::new(handler);

//         chain.link(persistent::Write::<SavedRequests>::both(
//             a_request_tree.clone(),
//         ));
//         chain.link_before(ParodyMiddleware);
//         chain.link_after(ParodyMiddleware);

//         ParodyServer {
//             a_request_tree: a_request_tree.clone(),
//         }
//     }

//     pub fn clone_request_tree(&self) -> RequestTree {
//         let request_tree: &RequestTree = &self.a_request_tree.as_ref().lock().unwrap();
//         <RequestTree as Clone>::clone(request_tree)
//     }

//     pub fn get_a_request_tree(&self) -> Arc<Mutex<RequestTree>> {
//         return self.a_request_tree.clone();
//     }
// }

// impl Default for ParodyServer {
//     /// A response caching version of the servermessagegame
//     ///
//     /// The first the first request to an URL is forwarded to an upstream
//     /// and following requests are got from the cache
//     fn default() -> Self {
//         ParodyServer::new(|req: &mut Request| {
//             let response = iron::Response::with((iron::status::Ok, ""));
//             let proxied_response = req.extensions().get::<ProxiedResponse>().unwrap();

//             Ok(response)
//         })
//     }
// }

// impl TestRequest {
//     pub fn from_iron_request(req: &Request) -> Self {
//         let url: url::Url = req.url.clone().into();

//         TestRequest {
//             method: reqwest::Method::from_str(req.method.as_ref())
//                 .expect(&format!("Unsupported method: {:?}", req.method)),
//             path: url.path().to_string(),
//             query: url.query_pairs().into_owned().collect(),
//             body: Vec::new(),
//         }
//     }
// }
