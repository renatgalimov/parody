use super::*;
use iron::Iron;
use std::path::Path;

fn respond_from_cache(req: &mut iron::Request) -> IronResult<iron::Response> {
    let response_cache = req
        .extensions
        .get::<ResponseCache>()
        .expect("response cache should always exist if cache middleware installed");

    match response_cache.load() {
        Ok(response) => Ok(response),
        Err(Error::Common(error)) => Err(error.into()),
        Err(Error::CacheMiss) => Err(IronError::new(
            Error::CacheMiss,
            iron::status::InternalServerError,
        )),
        Err(error) => Err(IronError::new(error, iron::status::InternalServerError)),
    }
}

fn init() {
    let _ = env_logger::builder().is_test(true).try_init();
}

#[test]
fn test_cache_middleware_when_response_cached_should_add_response_cache_to_environment() {
    init();
    let storage = Path::new(file!())
        .parent()
        .expect("source file always has a parent directory")
        .join("test_files")
        .join("localhost");

    let mut middleware = CacheMiddleware::new();
    middleware.set_root_dir(storage);

    let mut chain = iron::Chain::new(&respond_from_cache);
    chain.link_before(middleware);

    let mut cache_middleware_guard = Iron::new(chain)
        .http("localhost:3001")
        .expect("cache middleware service started");

    let mut response = reqwest::get("http://localhost:3001").expect("Cache request succeeded");
    cache_middleware_guard.close().unwrap();
    assert_eq!(response.status(), iron::status::Created.to_u16());

    let headers_raw: Vec<(&str, &str)> = response
        .headers()
        .iter()
        .filter_map(|(key, value)| match *key {
            reqwest::header::DATE | reqwest::header::TRANSFER_ENCODING => None,
            _ => Some((key.as_str(), value.to_str().unwrap())),
        })
        .collect();

    assert_eq!(headers_raw, vec![("content-type", "application/json")]);
    assert_eq!(
        response.text().expect("Response should have text body"),
        "{\"lorem\": \"ipsum\"}\n"
    );
}
