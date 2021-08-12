use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac, NewMac};
use http::{Request as HttpRequest, Uri};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, error::Error, fmt::Display, iter::FromIterator};
type HmacSha256 = Hmac<Sha256>;

const SHORT_DATE: &str = "%Y%m%d";
const LONG_DATETIME: &str = "%Y%m%dT%H%M%SZ";

pub type Request = HttpRequest<Vec<u8>>;

pub struct Credentials {
    pub aws_access_key_id: String,
    pub aws_secret_access_key: String,
}

pub struct TableInfo {
    pub table_name: String,
    pub key_name: String,
    pub value_name: String,
    pub region: String,
    pub endpoint: Option<String>,
}

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
struct PutItemInput {
    table_name: String,
    item: HashMap<String, Attr>,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct GetItemInput {
    table_name: String,
    key: HashMap<String, Attr>,
    projection_expression: String,
    expression_attribute_names: HashMap<String, String>,
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

    fn put_item_req(
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
            .clone()
            .unwrap_or_else(|| {
                format!(
                    "https://dynamodb.{region}.{domain}",
                    region = region,
                    domain = "amazonaws.com"
                )
            })
            .parse()?;
        self.sign(
            req.method("POST")
                .uri(&uri)
                .header("Host", uri.authority().expect("expected host").as_str())
                .header("Content-Type", "application/x-amz-json-1.0")
                .header("X-Amz-Target", "DynamoDB_20120810.PutItem")
                .body(serde_json::to_vec(&PutItemInput {
                    table_name: table_name.into(),
                    item: HashMap::from_iter([
                        (key_name.into(), Attr::S(key.as_ref().to_owned())),
                        (value_name.into(), Attr::S(value.as_ref().to_owned())),
                    ]),
                })?)?,
        )
    }

    fn get_item_req(
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
            .clone()
            .unwrap_or_else(|| {
                format!(
                    "https://dynamodb.{region}.{domain}",
                    region = region,
                    domain = "amazonaws.com"
                )
            })
            .parse()?;
        self.sign(
            req.method("POST")
                .uri(&uri)
                .header("Host", uri.authority().expect("expected host").as_str())
                .header("Content-Type", "application/x-amz-json-1.0")
                .header("X-Amz-Target", "DynamoDB_20120810.GetItem")
                .body(serde_json::to_vec(&GetItemInput {
                    table_name: table_name.into(),
                    key: HashMap::from_iter([(key_name.into(), Attr::S(key.as_ref().to_owned()))]),
                    // we use #v because https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/ReservedWords.html
                    projection_expression: "#v".into(),
                    expression_attribute_names: HashMap::from_iter([(
                        "#v".into(),
                        value_name.into(),
                    )]),
                })?)?,
        )
    }

    fn sign(
        &self,
        mut unsigned: Request,
    ) -> Result<Request, Box<dyn Error>> {
        // https://github.com/durch/rust-s3/blob/ae166bad53c25c88b9d3784fb816783142400567/s3/src/request_trait.rs#L286

        let sha = {
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
            let mut date_hmac = HmacSha256::new_varkey(format!("AWS4{}", secret_key).as_bytes())
                .map_err(|e| StrErr(e.to_string()))?;
            date_hmac.update(datetime.format(SHORT_DATE).to_string().as_bytes());

            Ok([region, service, "aws4_request"]
                .iter()
                .try_fold::<_, _, Result<_, Box<dyn Error>>>(date_hmac, |res, next| {
                    let mut next_mac = HmacSha256::new_varkey(&res.finalize().into_bytes())
                        .map_err(|e| StrErr(e.to_string()))?;
                    next_mac.update(next.to_string().as_bytes());
                    Ok(next_mac)
                })?
                .finalize()
                .into_bytes()
                .to_vec())
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

        fn canonical_uri_string(uri: &http::Uri) -> String {
            // decode `Url`'s percent-encoding and then reencode it
            // according to AWS's rules
            //let decoded = percent_encoding::percent_decode_str(uri.path()).decode_utf8_lossy();
            //uri_encode(&decoded, false)
            uri.path().to_string()
        }

        fn canonical_query_string(_uri: &http::Uri) -> String {
            // let mut keyvalues = uri
            //     .query()
            //     .map(|(key, value)| uri_encode(&key, true) + "=" + &uri_encode(&value, true))
            //     .collect::<Vec<String>>();
            // keyvalues.sort();
            // keyvalues.join("&")
            "".to_string()
        }

        fn canonical_request(
            method: &str,
            url: &http::Uri,
            headers: &http::HeaderMap,
            body_sha256: &str,
        ) -> String {
            format!(
                "{method}\n{uri}\n{query_string}\n{headers}\n\n{signed_headers}\n{body_sha256}",
                method = method,
                uri = canonical_uri_string(url),
                query_string = canonical_query_string(url),
                headers = canonical_header_string(headers),
                signed_headers = signed_header_string(headers),
                body_sha256 = body_sha256
            )
        }

        // verb url headers, sha256
        let canonical_request = canonical_request(
            unsigned.method().as_str(),
            unsigned.uri(),
            unsigned.headers(),
            sha.as_str(),
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

        let string_to_sign = string_to_sign(&now, &self.table_info.region, &canonical_request);
        let mut hmac = HmacSha256::new_varkey(&signing_key(
            &now,
            &self.credentials.aws_secret_access_key,
            &self.table_info.region,
            "dynamodb",
        )?)
        .map_err(|e| StrErr(e.to_string()))?;
        hmac.update(string_to_sign.as_bytes());
        let signature = hex::encode(hmac.finalize().into_bytes());
        let content_length = unsigned.body().len();
        let headers = unsigned.headers_mut();
        headers.append(
            "Authorization",
            authorization_header(
                &self.credentials.aws_access_key_id,
                &Utc::now(),
                &self.table_info.region,
                &signed_header_string(headers),
                &signature,
            )
            .parse()?,
        );
        headers.append("Content-Length", content_length.to_string().parse()?);
        //headers.append("X-Amz-Content-Sha256", sha.parse()?);

        Ok(unsigned)
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
