use std::{io::Read, string::String};

pub trait ParodyResponse {
    fn get_status(&self) -> u16;
    fn get_headers(&self) -> Vec<(String, Vec<u8>)>;
    fn get_body_reader(&mut self) -> &mut dyn Read;
}

impl ParodyResponse for reqwest::Response {
    fn get_body_reader(&mut self) -> &mut dyn std::io::Read {
        self
    }

    fn get_headers(&self) -> Vec<(String, Vec<u8>)> {
        self.headers()
            .iter()
            .map(|(name, value)| {
                (
                    name.as_str().to_owned(),
                    value.as_bytes().to_vec()
                )
            })
            .collect()
    }

    fn get_status(&self) -> u16 {
        self.status().as_u16()
    }
}
