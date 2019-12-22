use ::url::Url;

pub trait ParodyRequest {
    fn get_url(&self) -> Url;
    fn get_method(&self) -> String;
}
