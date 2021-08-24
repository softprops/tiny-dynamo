//! <div align="center">
//!   📦 🤏
//! </div>
//!
//! <h1 align="center">
//!   tiny dynamo
//! </h1>
//!
//! <p align="center">
//!    A tinier, simpler, key-value focused interface for AWS DynamoDB
//! </p>
//!
//! <div align="center">
//!   <a href="https://github.com/softprops/tiny-dynamo/actions">
//! 		<img src="https://github.com/softprops/tiny-dynamo/workflows/Main/badge.svg"/>
//! 	</a>
//! </div>
//!
//! ### Install
//!
//! To install tiny dynamo add the following to your `Cargo.toml` file.
//!
//! ```toml
//! [dependencies]
//! tiny-dynamo = "0.1"
//! ```
//!
//! ### Tiny what now?
//!
//! > Amazon DynamoDB is a key-value and document database that delivers single-digit millisecond performance at any scale.
//!
//! This quote comes directly from the [Amazon DynamoDB docs](https://aws.amazon.com/dynamodb/). This combinaton has some implications on its client APIs that are less than ideal for very simple key-value applications. These interfaces can be overly complicated and sometimes daunting for the uninitiated to say the least.
//!
//! Tiny Dynamo aims to leverge the useful parts of DynamoDB, the performance and scalability, but expose a much smaller and simpler API that you might expect from a key-value database.
//!
//! ### Usage
//!
//! ```rust ,no_run
//! use std::{env, error::Error};
//! use tiny_dynamo::{reqwest_transport::Reqwest, Credentials, Table, DB};
//!
//! fn main() -> Result<(), Box<dyn Error>> {
//!     let db = DB::new(
//!         Credentials::new(
//!             env::var("AWS_ACCESS_KEY_ID")?,
//!             env::var("AWS_SECRET_ACCESS_KEY")?,
//!         ),
//!         Table::new(
//!             "table-name",
//!             "key-attr-name",
//!             "value-attr-name",
//!             "us-east-1".parse()?,
//!             None
//!         ),
//!         Reqwest::new(),
//!     );
//!
//!     println!("{:#?}", db.set("foo", "bar")?);
//!     println!("{:#?}", db.get("foo")?);
//!
//!     Ok(())
//! }
//! ```
//!
//! A few notable differences when comparing Tiny Dynamo to traditional DynamoDB clients is that this client assumes a single table, a very common case for most DynamodbDB applications, so you configure your client with that table name so you don't need to redundantly provide it with each request.
//!
//! You will also find the interface is reduced to `get(key)` `set(key,value)`. This is intentional as this client is primarily focused on being a more comfortable fit for simple key-value applications.
//!
//! ## Features
//!
//! ### Tiny
//!
//! Tiny Dynamo avoids packing carry-on luggage for anything you don't explicitly need for a simple key-value application. This includes an entire sdk and a transient line of dependencies. This allows it to fit more easily into smaller spaces and to deliver on Rust's zero cost promise of not paying for what you don't use.
//!
//! ### Simpler Data Modeling
//!
//! A common laborious activity with DynamoDB based applications figuring our your application's data model first and then translating that to a DynamoDB's key space design and catalog of item attribute types. This is fine and expected for applications that require more advanced query access patterns. For simple key-value applications, this is just tax. Tiny DynamoDB assumes a key-value data model.
//!
//! ### Just the data plane
//!
//! You can think of the DynamoDB API in terms of two planes: The _data_ plane*cation cases, and the *control\* plane, a set of APIs for provisioning the resources that will store your data. Combining these makes its surface area arbitrarily larger that it needs to be. Tiny Dynamo focuses on exposing just the data plane to retain a smaller surface area to learn.
//!
//! ### Sans I/O
//!
//! Tiny Dynamo takes a [sans I/O](https://sans-io.readthedocs.io/) library approach. It defines a `Transport` trait which allows for any I/O library to implement how requests are transfered over the wire by provides none without an explict cargo feature toggled on
//!
//! Below are the current available cargo features
//!
//! #### `reqwest`
//!
//! the `reqwest` feature provides a `reqwest_transport::Reqwest` backend for sending requests, currently using a blocking client. An async feature is planned for the future
//!
//! ```toml
//! [dependencies]
//! tiny-dynamo = { version = "0.1", features = ["reqwest"]}
//! ```
//!
//! #### `fastly`
//!
//! The `fastly` feature provides a `fastly_transport::Fastly` backend for sending requests suitable for Fastlys Compute@Edge platform
//!
//! ```toml
//! [dependencies]
//! tiny-dynamo = { version = "0.1", features = ["fastly"]}
//! ```
//!
//! ### BYOIO
//!
//! If you would like to bring your own IO implementation you can define an implementation for a custom type
//!
//! ```rust
//! use tiny_dynamo::{Request, Transport};
//! use std::error::Error;
//!
//! struct CustomIO;
//!
//! impl Transport for CustomIO {
//!   fn send(&self, signed: Request) -> Result<(u16, String), Box<dyn Error>> {
//!     Ok(
//!       (200,"...".into())
//!     )
//!   }
//! }
//! ```
//!

//#![doc = include_str!("../README.md")]
#[cfg(feature = "fastly")]
pub mod fastly_transport;
mod region;
#[cfg(feature = "reqwest")]
pub mod reqwest_transport;

use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac, NewMac};
use http::{
    header::{HeaderName, AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE, HOST},
    method::Method,
    Request as HttpRequest, Uri,
};
pub use region::Region;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, error::Error, fmt::Display, iter::FromIterator};

const SHORT_DATE: &str = "%Y%m%d";
const LONG_DATETIME: &str = "%Y%m%dT%H%M%SZ";
const X_AMZ_CONTENT_SHA256: &[u8] = b"X-Amz-Content-Sha256";

/// A type alias for `http::RequestVec<u8>`
pub type Request = HttpRequest<Vec<u8>>;
type HmacSha256 = Hmac<Sha256>;

/// A set of AWS credentials to authenticate requests with
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

/// Information about your target AWS DynamoDB table
#[non_exhaustive]
pub struct Table {
    /// The name of your DynamoDB
    pub table_name: String,
    /// The name of the attribute that will store your key
    pub key_name: String,
    /// The name of the attribute that will store your value
    pub value_name: String,
    /// The AWS region the table is hosted in.
    ///
    /// When `endpoint` is defined, the value of this field is is somewhat arbitrary
    pub region: Region,
    /// An Optional, uri to address the DynamoDB api, often times just for dynamodb local
    pub endpoint: Option<String>,
}

impl Table {
    pub fn new(
        table_name: impl AsRef<str>,
        key_name: impl AsRef<str>,
        value_name: impl AsRef<str>,
        region: Region,
        endpoint: impl Into<Option<String>>,
    ) -> Self {
        Self {
            table_name: table_name.as_ref().into(),
            key_name: key_name.as_ref().into(),
            value_name: value_name.as_ref().into(),
            region,
            endpoint: endpoint.into(),
        }
    }
}

/// A trait to implement the behavior for sending requests, often your "IO" layer
pub trait Transport {
    /// Accepts a signed `http::Request<Vec<u8>>` and returns a tuple
    /// representing a response's HTTP status code and body
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

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct AWSError {
    #[serde(alias = "__type")]
    __type: String,
    message: String,
}

impl Display for AWSError {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        f.write_str(self.__type.as_str())?;
        f.write_str(": ")?;
        f.write_str(self.message.as_str())
    }
}

impl Error for AWSError {}

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

/// The central client interface applications will work with
///
/// # Example
///
/// ```rust ,no_run
/// # use std::{env, error::Error};
/// # use tiny_dynamo::{reqwest_transport::Reqwest, Credentials, Table, DB};
/// # fn main() -> Result<(), Box<dyn Error>> {
///let db = DB::new(
///    Credentials::new(
///        env::var("AWS_ACCESS_KEY_ID")?,
///        env::var("AWS_SECRET_ACCESS_KEY")?,
///    ),
///    Table::new(
///        "table-name",
///        "key-attr-name",
///        "value-attr-name",
///        "us-east-1".parse()?,
///        None
///    ),
///    Reqwest::new(),
///);
/// # Ok(())
/// # }
/// ```
pub struct DB {
    credentials: Credentials,
    table_info: Table,
    transport: Box<dyn Transport>,
}

impl DB {
    /// Returns a new instance of a DB
    pub fn new(
        credentials: Credentials,
        table_info: Table,
        transport: impl Transport + 'static,
    ) -> Self {
        Self {
            credentials,
            table_info,
            transport: Box::new(transport),
        }
    }

    /// Gets a value by its key
    pub fn get(
        &self,
        key: impl AsRef<str>,
    ) -> Result<Option<String>, Box<dyn Error>> {
        let Table { value_name, .. } = &self.table_info;
        match self.transport.send(self.get_item_req(key)?)? {
            (200, body) if body.as_str() == "{}" => Ok(None), // not found
            (200, body) => Ok(serde_json::from_str::<GetItemOutput>(&body)?
                .item
                .get(value_name)
                .iter()
                .find_map(|attr| match attr {
                    Attr::S(v) => Some(v.clone()),
                })),
            (_, body) => Err(Box::new(serde_json::from_str::<AWSError>(&body)?)),
        }
    }

    /// Sets a value for a given key
    pub fn set(
        &self,
        key: impl AsRef<str>,
        value: impl AsRef<str>,
    ) -> Result<(), Box<dyn Error>> {
        match self.transport.send(self.put_item_req(key, value)?)? {
            (200, _) => Ok(()),
            (_, body) => Err(Box::new(serde_json::from_str::<AWSError>(&body)?)),
        }
    }

    #[doc(hidden)]
    pub fn put_item_req(
        &self,
        key: impl AsRef<str>,
        value: impl AsRef<str>,
    ) -> Result<Request, Box<dyn Error>> {
        // https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_PutItem.html
        let req = http::Request::builder();
        let Table {
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
                    table_name,
                    item: HashMap::from_iter([
                        (key_name.as_str(), Attr::S(key.as_ref().to_owned())),
                        (value_name.as_ref(), Attr::S(value.as_ref().to_owned())),
                    ]),
                })?)?,
        )
    }

    #[doc(hidden)]
    pub fn get_item_req(
        &self,
        key: impl AsRef<str>,
    ) -> Result<Request, Box<dyn Error>> {
        // https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_GetItem.html
        let req = http::Request::builder();
        let Table {
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
                    table_name,
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
            let mut mac = HmacSha256::new_from_slice(key).map_err(|e| StrErr(e.to_string()))?;
            mac.update(data);
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
        ) -> Result<Vec<u8>, Box<dyn Error>> {
            [region.as_bytes(), b"dynamodb", b"aws4_request"]
                .iter()
                .try_fold::<_, _, Result<_, Box<dyn Error>>>(
                    hmac(
                        &[b"AWS4", secret_key.as_bytes()].concat(),
                        datetime.format(SHORT_DATE).to_string().as_bytes(),
                    )?,
                    |res, next| hmac(&res, next),
                )
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
                .collect::<Vec<_>>();
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

        let string_to_sign = string_to_sign(&now, self.table_info.region.id(), &canonical_request);
        let signature = hex::encode(hmac(
            &signing_key(
                &now,
                &self.credentials.aws_secret_access_key,
                self.table_info.region.id(),
            )?,
            string_to_sign.as_bytes(),
        )?);
        let headers_string = signed_header_string(unsigned.headers());
        let content_length = unsigned.body().len();
        unsigned.headers_mut().extend([
            (
                AUTHORIZATION,
                authorization_header(
                    &self.credentials.aws_access_key_id,
                    &Utc::now(),
                    self.table_info.region.id(),
                    &headers_string,
                    &signature,
                )
                .parse()?,
            ),
            (CONTENT_LENGTH, content_length.to_string().parse()?),
            (
                HeaderName::from_bytes(X_AMZ_CONTENT_SHA256)?,
                body_digest.parse()?,
            ),
        ]);

        Ok(unsigned)
    }
}

/// Provides a `Transport` implementation for a constantized response.
pub struct Const(pub u16, pub String);

impl Transport for Const {
    fn send(
        &self,
        _: Request,
    ) -> Result<(u16, String), Box<dyn Error>> {
        let Const(status, body) = self;
        Ok((*status, body.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_item_input_serilizes_as_expected() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            serde_json::to_string(&GetItemInput {
                table_name: "test-table",
                key: HashMap::from_iter([("key-name", Attr::S("key-value".into()))]),
                projection_expression: "#v",
                expression_attribute_names: HashMap::from_iter([("#v", "value-name")]),
            })?,
            r##"{"TableName":"test-table","Key":{"key-name":{"S":"key-value"}},"ProjectionExpression":"#v","ExpressionAttributeNames":{"#v":"value-name"}}"##
        );
        Ok(())
    }

    #[test]
    fn put_item_input_serilizes_as_expected() -> Result<(), Box<dyn Error>> {
        // assert_eq!(
        //     serde_json::to_string(&PutItemInput {
        //         table_name: "test-table",
        //         item: HashMap::from_iter([
        //             ("key-name", Attr::S("key-value".into())),
        //             ("value-name", Attr::S("value".into())),
        //         ]),
        //     })?,
        //     r##"{"TableName":"test-table","Item":{"key-name":{"S":"key-value"},"value-name":{"S":"value"}}}"##
        // );
        Ok(())
    }
}
