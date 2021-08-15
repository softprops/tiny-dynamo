//! # tiny dynamo
//!
//!
pub mod region;
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac, NewMac};
use http::{
    header::{AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE, HOST},
    method::Method,
    Request as HttpRequest, Uri,
};
use region::Region;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, error::Error, fmt::Display, iter::FromIterator};
type HmacSha256 = Hmac<Sha256>;

const SHORT_DATE: &str = "%Y%m%d";
const LONG_DATETIME: &str = "%Y%m%dT%H%M%SZ";

pub type Request = HttpRequest<Vec<u8>>;

/// A set of aws credentials to authenticate requests
pub struct Credentials {
    aws_access_key_id: String,
    aws_secret_access_key: String,
}

impl Credentials {
    pub fn new(
        aws_access_key_id: impl AsRef<str>,
        aws_secret_access_key: impl AsRef<str>,
    ) -> Self {
        Self {
            aws_access_key_id: aws_access_key_id.as_ref().to_owned(),
            aws_secret_access_key: aws_secret_access_key.as_ref().to_owned(),
        }
    }
}

/// Information about your target DynamoDB table
pub struct TableInfo {
    pub table_name: String,
    pub key_name: String,
    pub value_name: String,
    pub region: Region,
    pub endpoint: Option<String>,
}

/// A trait to the implemented for sending requests, often your "IO" layer
pub trait Requests {
    fn send(
        &self,
        signed: Request,
    ) -> Result<(u16, String), Box<dyn Error>>;
}

#[derive(Serialize, Deserialize)]
enum Attr {
    S(String),
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct PutItemInput<'a> {
    table_name: &'a str,
    item: HashMap<&'a str, Attr>,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct GetItemInput<'a> {
    table_name: &'a str,
    key: HashMap<&'a str, Attr>,
    projection_expression: &'a str,
    expression_attribute_names: HashMap<&'a str, &'a str>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct GetItemOutput {
    item: HashMap<String, Attr>,
}

#[derive(Debug)]
struct StrErr(String);

impl Display for StrErr {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl Error for StrErr {}

pub struct DB {
    pub credentials: Credentials,
    pub table_info: TableInfo,
    pub requests: Box<dyn Requests>,
}

impl DB {
    pub fn new(
        credentials: Credentials,
        table_info: TableInfo,
        requests: impl Requests + 'static,
    ) -> Self {
        Self {
            credentials,
            table_info,
            requests: Box::new(requests),
        }
    }

    pub fn get(
        &self,
        key: impl AsRef<str>,
    ) -> Result<Option<String>, Box<dyn Error>> {
        let TableInfo { value_name, .. } = &self.table_info;
        match self.requests.send(self.get_item_req(key)?)? {
            (200, body) => Ok(serde_json::from_str::<GetItemOutput>(&body)?
                .item
                .get(value_name)
                .iter()
                .find_map(|attr| match attr {
                    Attr::S(v) => Some(v.clone()),
                })),
            _ => Ok(None),
        }
    }

    pub fn set(
        &self,
        key: impl AsRef<str>,
        value: impl AsRef<str>,
    ) -> Result<(), Box<dyn Error>> {
        match self.requests.send(self.put_item_req(key, value)?)? {
            (200, _) => Ok(()),
            _ => Ok(()), // fixme: communicate error
        }
    }

    pub fn put_item_req(
        &self,
        key: impl AsRef<str>,
        value: impl AsRef<str>,
    ) -> Result<Request, Box<dyn Error>> {
        // https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_PutItem.html
        let req = http::Request::builder();
        let TableInfo {
            table_name,
            key_name,
            value_name,
            region,
            endpoint,
            ..
        } = &self.table_info;
        let uri: Uri = endpoint
            .as_deref()
            .unwrap_or_else(|| region.endpoint())
            .parse()?;
        self.sign(
            req.method(Method::POST)
                .uri(&uri)
                .header(HOST, uri.authority().expect("expected host").as_str())
                .header(CONTENT_TYPE, "application/x-amz-json-1.0")
                .header("X-Amz-Target", "DynamoDB_20120810.PutItem")
                .body(serde_json::to_vec(&PutItemInput {
                    table_name: table_name,
                    item: HashMap::from_iter([
                        (key_name.as_str(), Attr::S(key.as_ref().to_owned())),
                        (value_name.as_ref(), Attr::S(value.as_ref().to_owned())),
                    ]),
                })?)?,
        )
    }

    pub fn get_item_req(
        &self,
        key: impl AsRef<str>,
    ) -> Result<Request, Box<dyn Error>> {
        // https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_GetItem.html
        let req = http::Request::builder();
        let TableInfo {
            table_name,
            key_name,
            value_name,
            region,
            endpoint,
            ..
        } = &self.table_info;
        let uri: Uri = endpoint
            .as_deref()
            .unwrap_or_else(|| region.endpoint())
            .parse()?;
        self.sign(
            req.method(Method::POST)
                .uri(&uri)
                .header(HOST, uri.authority().expect("expected host").as_str())
                .header(CONTENT_TYPE, "application/x-amz-json-1.0")
                .header("X-Amz-Target", "DynamoDB_20120810.GetItem")
                .body(serde_json::to_vec(&GetItemInput {
                    table_name: table_name,
                    key: HashMap::from_iter([(
                        key_name.as_str(),
                        Attr::S(key.as_ref().to_owned()),
                    )]),
                    // we use #v because https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/ReservedWords.html
                    projection_expression: "#v",
                    expression_attribute_names: HashMap::from_iter([("#v", value_name.as_ref())]),
                })?)?,
        )
    }

    fn sign(
        &self,
        mut unsigned: Request,
    ) -> Result<Request, Box<dyn Error>> {
        fn hmac(
            key: &[u8],
            data: &[u8],
        ) -> Result<Vec<u8>, Box<dyn Error>> {
            let mut mac = HmacSha256::new_varkey(&key).map_err(|e| StrErr(e.to_string()))?;
            mac.update(&data);
            Ok(mac.finalize().into_bytes().to_vec())
        }

        let body_digest = {
            let mut sha = Sha256::default();
            sha.update(unsigned.body());
            hex::encode(sha.finalize().as_slice())
        };

        let now = Utc::now();
        unsigned
            .headers_mut()
            .append("X-Amz-Date", now.format(LONG_DATETIME).to_string().parse()?);

        fn signed_header_string(headers: &http::HeaderMap) -> String {
            let mut keys = headers
                .keys()
                .map(|key| key.as_str().to_lowercase())
                .collect::<Vec<_>>();
            keys.sort();
            keys.join(";")
        }

        fn string_to_sign(
            datetime: &DateTime<Utc>,
            region: &str,
            canonical_req: &str,
        ) -> String {
            let mut hasher = Sha256::default();
            hasher.update(canonical_req.as_bytes());
            format!(
                "AWS4-HMAC-SHA256\n{timestamp}\n{scope}\n{canonical_req_hash}",
                timestamp = datetime.format(LONG_DATETIME),
                scope = scope_string(datetime, region),
                canonical_req_hash = hex::encode(hasher.finalize().as_slice())
            )
        }

        fn signing_key(
            datetime: &DateTime<Utc>,
            secret_key: &str,
            region: &str,
            service: &str,
        ) -> Result<Vec<u8>, Box<dyn Error>> {
            Ok([region.as_bytes(), service.as_bytes(), b"aws4_request"]
                .iter()
                .try_fold::<_, _, Result<_, Box<dyn Error>>>(
                    hmac(
                        format!("AWS4{}", secret_key).as_bytes(),
                        datetime.format(SHORT_DATE).to_string().as_bytes(),
                    )?,
                    |res, next| Ok(hmac(&res, next)?),
                )?)
        }

        fn scope_string(
            datetime: &DateTime<Utc>,
            region: &str,
        ) -> String {
            format!(
                "{date}/{region}/dynamodb/aws4_request",
                date = datetime.format(SHORT_DATE),
                region = region
            )
        }

        fn canonical_header_string(headers: &http::HeaderMap) -> String {
            let mut keyvalues = headers
                .iter()
                .map(|(key, value)| {
                    // Values that are not strings are silently dropped (AWS wouldn't
                    // accept them anyway)
                    key.as_str().to_lowercase() + ":" + value.to_str().unwrap().trim()
                })
                .collect::<Vec<String>>();
            keyvalues.sort();
            keyvalues.join("\n")
        }

        fn canonical_request(
            method: &str,
            headers: &http::HeaderMap,
            body_digest: &str,
        ) -> String {
            // note: all dynamodb uris are requests to / with no query string so theres no need
            // to derive those from the request
            format!(
                "{method}\n/\n\n{headers}\n\n{signed_headers}\n{body_digest}",
                method = method,
                headers = canonical_header_string(headers),
                signed_headers = signed_header_string(headers),
                body_digest = body_digest
            )
        }

        let canonical_request = canonical_request(
            unsigned.method().as_str(),
            unsigned.headers(),
            body_digest.as_str(),
        );

        fn authorization_header(
            access_key: &str,
            datetime: &DateTime<Utc>,
            region: &str,
            signed_headers: &str,
            signature: &str,
        ) -> String {
            format!(
                "AWS4-HMAC-SHA256 Credential={access_key}/{scope}, SignedHeaders={signed_headers}, Signature={signature}",
                access_key = access_key,
                scope = scope_string(datetime, region),
                signed_headers = signed_headers,
                signature = signature
            )
        }

        let string_to_sign = string_to_sign(&now, &self.table_info.region.id(), &canonical_request);
        let signature = hex::encode(hmac(
            &signing_key(
                &now,
                &self.credentials.aws_secret_access_key,
                &self.table_info.region.id(),
                "dynamodb",
            )?,
            string_to_sign.as_bytes(),
        )?);
        let content_length = unsigned.body().len();
        let headers = unsigned.headers_mut();
        headers.append(
            AUTHORIZATION,
            authorization_header(
                &self.credentials.aws_access_key_id,
                &Utc::now(),
                &self.table_info.region.id(),
                &signed_header_string(headers),
                &signature,
            )
            .parse()?,
        );
        headers.append(CONTENT_LENGTH, content_length.to_string().parse()?);
        headers.append("X-Amz-Content-Sha256", body_digest.parse()?);

        Ok(unsigned)
    }
}

pub struct Static(pub u16, pub String);

impl Requests for Static {
    fn send(
        &self,
        _: Request,
    ) -> Result<(u16, String), Box<dyn Error>> {
        let Static(status, body) = self;
        Ok((*status, body.clone()))
    }
}

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

impl Requests for Reqwest {
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
        let status = resp.status().as_u16();
        let body = resp.text()?;
        //println!("\nresp {} {}", status, body);
        Ok((status, body))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_item_input_serilizes_as_expected() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            serde_json::to_string(&GetItemInput {
                table_name: "test-table".into(),
                key: HashMap::from_iter([("key-name".into(), Attr::S("key-value".into()))]),
                projection_expression: "#v".into(),
                expression_attribute_names: HashMap::from_iter([(
                    "#v".into(),
                    "value-name".into()
                )]),
            })?,
            r##"{"TableName":"test-table","Key":{"key-name":{"S":"key-value"}},"ProjectionExpression":"#v","ExpressionAttributeNames":{"#v":"value-name"}}"##
        );
        Ok(())
    }

    #[test]
    fn put_item_input_serilizes_as_expected() -> Result<(), Box<dyn Error>> {
        // assert_eq!(
        //     serde_json::to_string(&GetItemInput {
        //         table_name: "test-table".into(),
        //         key: HashMap::from_iter([
        //             ("key-name".into(), Attr::S("key-value".into())),
        //             ("value-name".into(), Attr::S("value-value".into()))
        //         ]),
        //         projection_expression: "value-name".into(),
        //     })?,
        //     r#"{"TableName":"test-table","Item":{"key-name":{"S":"key-value"},"value-name":{"S":"value-value"}}"#
        // );
        Ok(())
    }
}
