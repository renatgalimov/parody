use super::*;
use crate::iron::Set;
use iron::{typemap::Key, BeforeMiddleware, Chain, Iron, IronResult, Plugin};
use persistent::Read;

#[derive(Clone, Copy)]
struct TestStatus;
impl Key for TestStatus {
    type Value = iron::status::Status;
}

#[derive(Clone, Copy)]
struct TestHeaders;
impl Key for TestHeaders {
    type Value = Vec<(&'static str, &'static str)>;
}

#[derive(Clone, Copy)]
struct TestBody;
impl Key for TestBody {
    type Value = Vec<u8>;
}

fn respond_from_environment(req: &mut iron::Request) -> IronResult<iron::Response> {
    trace!(target: "test", "Entered respond_from_environment");

    let a_status = req
        .get::<Read<TestStatus>>()
        .expect("Cannot get status for test request");
    let a_headers_raw = req
        .get::<Read<TestHeaders>>()
        .expect("Cannot get response headers");
    let a_body = req
        .get::<Read<TestBody>>()
        .expect("Cannot get response headers");

    let mut response = iron::Response::with((*a_status, (*a_body).clone()));

    for (key, value) in a_headers_raw.iter() {
        trace!("Setting header: {}={}", key, value);
        response
            .headers
            .append_raw(*key, (*value).as_bytes().to_vec());
    }

    Ok(response)
}

fn start_upstream(
    status: iron::status::Status,
    headers: Vec<(&'static str, &'static str)>,
) -> Result<iron::Listening> {
    let mut chain = Chain::new(&respond_from_environment);
    chain.link(Read::<TestStatus>::both(status));
    chain.link(Read::<TestHeaders>::both(headers));
    chain.link(Read::<TestBody>::both(
        "{\"Lorem\": \"ipsum\"}".as_bytes().to_vec(),
    ));
    Iron::new(chain)
        .http("localhost:3000")
        .map_err(|err| err.into())
}

fn to_iron_response(response: reqwest::Response) -> iron::Response {
    let mut result = iron::Response::with(iron::status::Unregistered(response.status().as_u16()));

    for (name, value) in response.headers() {
        result
            .headers
            .append_raw(name.as_str().to_owned(), value.as_bytes().to_vec());
    }

    result = result.set(iron::response::BodyReader(response));

    result
}

fn forward_from_environment(req: &mut iron::Request) -> IronResult<iron::Response> {
    let proxy_response = req
        .extensions
        .remove::<ProxyResponse>()
        .expect("Proxy response should exist in tests");

    proxy_response
        .load()
        .map(to_iron_response)
        .map_err(|error| iron::IronError::new(error, iron::status::InternalServerError))
}

fn init() {
    let _ = env_logger::builder().is_test(true).try_init();
}

struct DummyNetworkStream {
    response: std::io::Cursor<String>,
}

impl DummyNetworkStream {
    fn new() -> Self {
        let content = String::from("GET / HTTP/1.1\nHost: localhost\n\n");

        Self {
            response: std::io::Cursor::new(content),
        }
    }
}

impl std::io::Write for DummyNetworkStream {
    fn flush(&mut self) -> std::result::Result<(), std::io::Error> {
        unimplemented!()
    }
    fn write(&mut self, _: &[u8]) -> std::result::Result<usize, std::io::Error> {
        unimplemented!()
    }
}

impl std::io::Read for DummyNetworkStream {
    fn read(&mut self, buf: &mut [u8]) -> std::result::Result<usize, std::io::Error> {
        std::io::copy(&mut self.response, &mut std::io::Cursor::new(buf)).map(|n| n as usize)
    }
}

impl hyper::net::NetworkStream for DummyNetworkStream {
    fn set_write_timeout(
        &self,
        _: std::option::Option<std::time::Duration>,
    ) -> std::result::Result<(), std::io::Error> {
        unimplemented!()
    }
    fn set_read_timeout(
        &self,
        _: std::option::Option<std::time::Duration>,
    ) -> std::result::Result<(), std::io::Error> {
        unimplemented!()
    }
    fn peer_addr(&mut self) -> std::result::Result<std::net::SocketAddr, std::io::Error> {
        unimplemented!()
    }
}

#[test]
fn forward_middleware_before_should_replace_request_schema_with_upstream_schema() {
    init();
    let middleware =
        ForwardMiddleware::new(url::Url::from_str("https://example.com").expect("URL is valid"));

    let addr = std::net::SocketAddr::new(
        std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
        80,
    );

    let mut network_stream = DummyNetworkStream::new();
    let mut stream =
        hyper::buffer::BufReader::new(&mut network_stream as &mut dyn hyper::net::NetworkStream);
    let http_request = match iron::request::HttpRequest::new(&mut stream, addr) {
        Ok(request) => request,
        Err(error) => panic!("Cannot create HttpRequest: {}", error),
    };
    let mut request =
        iron::Request::from_http(http_request, addr, &iron::Protocol::http()).unwrap();

    middleware
        .before(&mut request)
        .expect("Before middleware should succeed");

    let cached_request: &reqwest::Request = request
        .extensions
        .get::<ProxyResponse>()
        .expect("'before' method shoud set proxy response");

    assert_eq!(request.url.scheme(), "http");
    assert_eq!(cached_request.url().scheme(), "https");
}

#[test]
fn forward_middleware_should_fill_environment_with_response() {
    init();
    let mut upstream_guard = start_upstream(
        iron::status::Accepted,
        vec![("content-type", "application/json")],
    )
    .expect("Upstream service should start");

    let mut test_chain = iron::Chain::new(&forward_from_environment);
    test_chain.link_before(ForwardMiddleware::new(
        url::Url::from_str("http://localhost:3000").expect("Test URL is valid"),
    ));
    let mut test_guard = Iron::new(test_chain)
        .listen(
            hyper::net::HttpListener::new(std::net::SocketAddr::new(
                std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
                0,
            ))
            .expect("HTTP listener should be created in tests"),
            iron::Protocol::http(),
        )
        .expect("Test service should always start successfully");

    let mut response = reqwest::get(&format!("http://localhost:{}", test_guard.socket.port()))
        .expect("Cache request succeeded");

    upstream_guard.close().unwrap();
    test_guard.close().unwrap();

    assert_eq!(response.status(), iron::status::Accepted.to_u16());
    let headers_raw: Vec<(&str, &str)> = response
        .headers()
        .iter()
        .filter_map(|(key, value)| match *key {
            reqwest::header::DATE | reqwest::header::TRANSFER_ENCODING => None,
            _ => Some((key.as_str(), value.to_str().unwrap())),
        })
        .collect();

    assert_eq!(
        headers_raw,
        vec![
            ("content-length", "18"),
            ("content-type", "application/json")
        ]
    );
    assert_eq!(
        response.text().expect("Response should have text body"),
        "{\"Lorem\": \"ipsum\"}"
    );
}
