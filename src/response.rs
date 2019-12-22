use std::io::Read;

pub trait ParodyResponse {
    fn get_status(&self) -> u16;
    fn get_headers(&self) -> Vec<(String, String)>;
    fn get_body_reader(&mut self) -> &mut dyn Read;
 }
