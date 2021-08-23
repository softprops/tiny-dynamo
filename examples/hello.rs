use std::{env, error::Error};
use tiny_dynamo::{reqwest_transport::Reqwest, Credentials, Table, DB};

fn main() -> Result<(), Box<dyn Error>> {
    // docker run -p 8000:8000 amazon/dynamodb-local
    // AWS_ACCESS_KEY_ID=foo AWS_SECRET_ACCESS_KEY=foo aws dynamodb create-table --endpoint-url http://localhost:8000 --table-name test --key-schema AttributeName=key,KeyType=HASH --attribute-definitions AttributeName=key,AttributeType=S --provisioned-throughput ReadCapacityUnits=1,WriteCapacityUnits=1
    // https://www.rahulpnath.com/blog/aws_dynamodb_local/
    let db = DB::new(
        Credentials::new(
            env::var("AWS_ACCESS_KEY_ID")?,
            env::var("AWS_SECRET_ACCESS_KEY")?,
        ),
        Table::new(
            env::var("TABLE_NAME").ok().as_deref().unwrap_or("test"),
            env::var("KEY_NAME").ok().as_deref().unwrap_or("key"),
            env::var("VALUE_NAME").ok().as_deref().unwrap_or("value"),
            env::var("AWS_REGION")
                .ok()
                .as_deref()
                .unwrap_or("us-east-1")
                .parse()?,
            Some("http://localhost:8000".into()),
        ),
        Reqwest::new(),
    );
    println!("{:#?}", db.set("foo", "bar")?);
    println!("{:#?}", db.get("foo")?);
    Ok(())
}
