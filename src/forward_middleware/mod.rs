extern crate tempfile;
use crate::{error::Error, result::Result};
use iron::typemap::Key;
use std::{fs::File, io::Seek, str::FromStr};
#[cfg(test)]
mod test;

// extern crate reqwest;
// use failure::Error;
// use iron::{IronError, IronResult};
// use std::path::PathBuf;

// use crate::response::Response;

// /// Caches responses from upstream in the local filesystem
pub struct ForwardMiddleware {
    upstream_url: reqwest::Url,
}

pub trait ProxyLoad {
    fn load(self) -> Result<reqwest::Response>;
}

#[derive(Clone, Copy)]
pub struct ProxyResponse;

impl ProxyLoad for reqwest::Request {
    fn load(self) -> Result<reqwest::Response> {
        reqwest::Client::new().execute(self).map_err(|error| error.into())
        // let mut proxy_response = iron::Response::new();
        // proxy_response.status = Some(iron::status::Unregistered(real_response.status().as_u16()));

        // for (name, value) in real_response.headers() {
        //     proxy_response
        //         .headers
        //         .append_raw(name.as_str().to_owned(), value.as_bytes().to_vec());
        // }

        // proxy_response.body = Some(Box::new(ReqwestWriteBody {
        //     response: real_response,
        // }));

        // Ok(proxy_response)
    }
}

impl Key for ProxyResponse {
    type Value = reqwest::Request;
}

impl ForwardMiddleware {
    pub fn new(upstream_url: url::Url) -> Self {
        Self {
            upstream_url: upstream_url,
        }
    }
    // /// Proxy the request to the upstream
    // ///
    // /// # Arguments
    // ///
    // /// * `req` - a request made to the local server. It's URL looks like `http://127.0.0.1/path?query...
    // fn proxy(&self, req: &mut iron::Request) -> Result<reqwest::Response, Error> {
    //     let mut target_url: url::Url = req.url.clone().into();
    //     target_url.set_host(self.upstream_url.host_str())?;
    //     target_url
    //         .set_port(self.upstream_url.port())
    //         .expect("Cannot set port");

    //     let target_method: reqwest::Method = reqwest::Method::from_str(req.method.as_ref())
    //         .expect(&format!("Unsupported method: {:?}", req.method));

    //     Ok(self
    //         .client
    //         .execute(reqwest::Request::new(target_method, target_url))?)
    // }
}

impl iron::BeforeMiddleware for ForwardMiddleware {
    fn before(&self, req: &mut iron::Request) -> iron::IronResult<()> {
        let mut new_url: url::Url = req.url.clone().into();
        new_url
            .set_host(self.upstream_url.host_str())
            .expect("Copying host should always succeed");
        new_url
            .set_port(self.upstream_url.port())
            .expect("Copying port should always succeed");

        let mut proxy_request = reqwest::Client::new().request(
            reqwest::Method::from_str(req.method.as_ref())
                .expect("Iron method should convert to Reqwest method"),
            new_url,
        );

        for header in req.headers.iter() {
            trace!(target: "forward", "Setting header: {}: {}", header.name(), header.value_string());
            proxy_request = proxy_request.header(header.name(), header.value_string());
        }

        let mut body_file: File = tempfile::tempfile().expect("Temporary file should be created");
        std::io::copy(&mut req.body, &mut body_file).expect("Should never happen when testing");
        body_file
            .seek(std::io::SeekFrom::Start(0))
            .expect("Should neven happen when testing");

        proxy_request = proxy_request.body(body_file);

        req.extensions.insert::<ProxyResponse>(
            proxy_request
                .build()
                .expect("Request conversion should never fail"),
        );

        Ok(())
    }
}

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
