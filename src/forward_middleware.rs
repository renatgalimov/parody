// extern crate reqwest;
// use failure::Error;
// use iron::{IronError, IronResult};
// use std::path::PathBuf;

// use crate::response::Response;

// /// Caches responses from upstream in the local filesystem
// struct ForwardMiddleware {
//     upstream_url: reqwest::Url,
//     save_dir: PathBuf,
//     dir_query: Vec<u8>,
//     client: reqwest::Client,
// }

// impl ForwardMiddleware {
//     /// Proxy the request to the upstream
//     ///
//     /// # Arguments
//     ///
//     /// * `req` - a request made to the local server. It's URL looks like `http://127.0.0.1/path?query...
//     fn proxy(&self, req: &mut iron::Request) -> Result<reqwest::Response, Error> {
//         let mut target_url: url::Url = req.url.clone().into();
//         target_url.set_host(self.upstream_url.host_str())?;
//         target_url
//             .set_port(self.upstream_url.port())
//             .expect("Cannot set port");

//         let target_method: reqwest::Method = reqwest::Method::from_str(req.method.as_ref())
//             .expect(&format!("Unsupported method: {:?}", req.method));

//         Ok(self
//             .client
//             .execute(reqwest::Request::new(target_method, target_url))?)
//     }
// }

// impl iron::typemap::Key for Box<dyn Response> {
//     type Value = Box<dyn Response>;
// }

// impl iron::BeforeMiddleware for ForwardMiddleware {
//     fn before(&self, req: &mut iron::Request) -> iron::IronResult<()> {
//         let response: reqwest::Response = self
//             .proxy(req)
//             .map_err(|err| IronError::new(err.compat(), iron::status::InternalServerError))?;

//         let directory_structure = storage::get_response_storage_dir(req);
//         std::fs::create_dir_all(&directory_structure)
//             .map_err(|err| IronError::new(err, iron::status::InternalServerError))?;

//         let headers_path = directory_structure.join(format!("{}.headers.json", req.method));

//         let mut header_list = Vec::<(String, Vec<String>)>::new();
//         let response_headers = response.headers();

//         for header in response_headers.keys() {
//             let values: Vec<String> = response_headers
//                 .get_all(header)
//                 .iter()
//                 .map(|value| value.to_str().unwrap().to_owned())
//                 .collect();

//             header_list.push((header.to_string(), values));
//         }

//         File::open(headers_path)
//             .and_then(|mut file| file.write(serde_json::to_string_pretty(&header_list)?.as_ref()))
//             .map_err(|err| IronError::new(err, iron::status::InternalServerError))?;

//         req.extensions_mut()
//             .insert::<ProxiedResponse>(From::from(&response));

//         Ok(())
//     }
// }

// #[cfg(test)]
// mod test {
//     use super::*;

//     #[test]
//     fn proxy_should_forward_request() {
//         let forwarder = ForwardMiddleware {
//             upstream_url: url::Url::from_str("https://httpbin.org").unwrap(),
//             save_dir: PathBuf::from("./forward_middleware"),
//             client: reqwest::Client::new(),
//             dir_query: Vec::new(),
//         };

//         let mut chain =
//             Chain::new(|_: &mut Request| Ok(Response::with((iron::status::Ok, "Hello"))));

//         chain.link_before(forwarder);

//         std::thread::spawn(move || Iron::new(chain).http("localhost:3000").unwrap());
//     }
// }
