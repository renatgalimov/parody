extern crate tempfile;
use crate::result::Result;
use iron::typemap::Key;
use std::{fs::File, io::Seek, path::PathBuf, str::FromStr};
#[cfg(test)]
mod test;

/// Lazily makes requests to the Upstream
///
/// In general, this middleware doesn't make requests,
/// it inserts a request-making object to the environment
/// and if necessary - the request might be executed.
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
        trace!("Loading proxy response: {} {}", self.method(), self.url());

        let response = reqwest::Client::new()
            .execute(self)
            .map_err(|error| error.into());

        response
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
}

impl iron::BeforeMiddleware for ForwardMiddleware {
    fn before(&self, req: &mut iron::Request) -> iron::IronResult<()> {
        trace!(
            "In ForwardMiddleware for: {} {:.32}",
            req.method,
            req.url
        );

        let mut new_url_path = PathBuf::from(self.upstream_url.path());
        new_url_path = new_url_path.join(req.url.path().join("/"));
        let mut new_url: url::Url = req.url.clone().into();

        new_url
            .set_host(self.upstream_url.host_str())
            .expect("Copying host should always succeed");
        new_url
            .set_port(self.upstream_url.port())
            .expect("Copying port should always succeed");
        new_url
            .set_scheme(self.upstream_url.scheme())
            .expect("Copying scheme should always succeed");
        new_url.set_path(
            new_url_path
                .to_str()
                .expect("New path should not contain invalid characters"),
        );

        trace!("New forward URL: {:32}", new_url.as_str());

        let mut proxy_request = reqwest::Client::new().request(
            reqwest::Method::from_str(req.method.as_ref())
                .expect("Iron method should convert to Reqwest method"),
            new_url,
        );

        for header in req.headers.iter() {
            if header.name() == "host" {
                trace!(target: "forward", "Skipped header: {}: {}", header.name(), header.value_string());
                continue;
            }

            trace!(target: "forward", "Setting header: {}: {}", header.name(), header.value_string());
            proxy_request = proxy_request.header(header.name(), header.value_string());
        }

        let host = self
            .upstream_url
            .host_str()
            .expect("There should be a host string in the upstream URL");

        trace!(target: "forward", "Setting header: host: {}", host);
        proxy_request = proxy_request.header("host", host);

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
