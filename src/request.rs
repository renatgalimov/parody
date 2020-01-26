use ::url::Url;

pub trait ParodyRequest {
    fn get_url(&self) -> Url;
    fn get_method(&self) -> String;
}

impl std::fmt::Debug for dyn ParodyRequest + Send + Sync {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "Parody request: {}", self.get_url())
    }
}

pub struct RequestLogItem {
    url: Url,
    method: String,
    // headers: Vec<(String, Vec<u8>)>,
}

impl ParodyRequest for RequestLogItem {
    fn get_url(&self) -> Url {
        self.url.clone()
    }

    fn get_method(&self) -> String {
        self.method.clone()
    }
}

impl From<iron::Request<'_, '_>> for RequestLogItem {
    fn from(req: iron::Request<'_, '_>) -> Self {
        RequestLogItem {
            url: req.url.into(),
            method: req.method.as_ref().to_owned(),
            // headers: req
            //     .headers
            //     .iter()
            //     .map(|header| {
            //         (
            //             header.name().to_string(),
            //             header.value_string().as_bytes().to_vec(),
            //         )
            //     })
            //     .collect(),
        }
    }
}

impl From<&iron::Request<'_, '_>> for RequestLogItem {
    fn from(req: &iron::Request<'_, '_>) -> Self {
        RequestLogItem {
            url: req.url.clone().into(),
            method: req.method.as_ref().to_owned(),
            // headers: req
            //     .headers
            //     .iter()
            //     .map(|header| {
            //         (
            //             header.name().to_string(),
            //             header.value_string().as_bytes().to_vec(),
            //         )
            //     })
            //     .collect(),
        }
    }
}
