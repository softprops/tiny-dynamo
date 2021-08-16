use crate::{Request, Requests};
use fastly::{Body, Request as FastlyRequest};
use std::error::Error;

pub struct Fastly {
    backend: String,
}

impl Fastly {
    pub fn new(backend: impl AsRef<str>) -> Self {
        Self {
            backend: backend.as_ref().to_string(),
        }
    }
}

impl Requests for Fastly {
    fn send(
        &self,
        signed: Request,
    ) -> Result<(u16, String), Box<dyn Error>> {
        let (parts, body) = signed.into_parts();
        let fastly_body: Body = body.into();
        let fastly_compat = http::Request::from_parts(parts, fastly_body);
        let fr: FastlyRequest = fastly_compat.into();
        let resp = fr.send(&self.backend)?;
        Ok((resp.get_status().as_u16(), resp.into_body_str()))
    }
}
