use crate::{Request, Transport};
use reqwest::blocking::Client;
use std::error::Error;

pub struct Reqwest {
    client: Client,
}

impl Default for Reqwest {
    fn default() -> Self {
        Self::new()
    }
}

impl Reqwest {
    pub fn new() -> Self {
        Reqwest {
            client: Client::new(),
        }
    }
}

impl Transport for Reqwest {
    fn send(
        &self,
        signed: Request,
    ) -> Result<(u16, String), Box<dyn Error>> {
        let resp = self
            .client
            .post(signed.uri().to_string())
            .headers(signed.headers().clone())
            .body(signed.body().clone())
            .send()?;
        Ok((resp.status().as_u16(), resp.text()?))
    }
}
