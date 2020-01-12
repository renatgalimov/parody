use std::io::Read;

pub trait ParodyResponse {
    fn get_status(&self) -> u16;
    fn get_headers(&self) -> Vec<(String, String)>;
    fn get_body_reader(&mut self) -> &mut dyn Read;
}


impl ParodyResponse for reqwest::Response {
    fn get_body_reader(&mut self) -> &mut dyn std::io::Read {
        self
    }

    fn get_headers(&self) -> std::vec::Vec<(std::string::String, std::string::String)> {
        self.headers()
            .iter()
            .map(|(name, value)| {
                (
                    name.as_str().to_owned(),
                    std::str::from_utf8(value.as_bytes())
                        .expect("Fixme: never convert header values")
                        .to_owned(),
                )
            })
            .collect()
    }

    fn get_status(&self) -> u16 {
        self.status().as_u16()
    }
}
