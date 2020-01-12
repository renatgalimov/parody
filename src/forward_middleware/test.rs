use super::*;
use iron::{typemap::Key, Chain, Iron, IronResult, Plugin};
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
    let headers = iron::headers::HeaderMap::new();

    for (name, value) in response.headers() {
        headers.append_raw(name.as_str(), value.as_bytes().to_vec());
    }

    let mut result = iron::Response::with((
        iron::status::Unregistered(response.status().as_u16()),
        headers,
    ));
    result
}

fn forward_from_environment(req: &mut iron::Request) -> IronResult<iron::Response> {
    let proxy_response = req
        .extensions
        .remove::<ProxyResponse>()
        .expect("Proxy response should exist intests");

    let iron_response: iron::Response = proxy_response
        .load()
        .expect("Proxy request should always succeed");

    Ok(iron_response)
}

fn init() {
    let _ = env_logger::builder().is_test(true).try_init();
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
