<div align="center">
  📦 🤏
</div>

<h1 align="center">
  tiny dynamo
</h1>

<p align="center">
   A tinier, simpler, key-value focused interface for AWS DynamoDB
</p>

<div align="center">
  <a href="https://github.com/softprops/tiny-dynamo/actions">
		<img src="https://github.com/softprops/tiny-dynamo/workflows/Main/badge.svg"/>
	</a>
</div>

### Tiny what now?

> Amazon DynamoDB is a key-value and document database that delivers single-digit millisecond performance at any scale.

This quote comes directly from the [Amazon DynamoDB docs](https://aws.amazon.com/dynamodb/). This combinaton has some implications on its client APIs that are less than ideal for very simple key-value applications. These interfaces can be overly complicated and sometimes daunting for the uninitiated to say the least.

Tiny Dynamo aims to leverge the useful parts of DynamoDB, the performance and scalability, but expose a much smaller and simpler API that you might expect from a key-value database.

### Usage

```rust ,no_run
use std::{env, error::Error};
use tiny_dynamo::{reqwest_transport::Reqwest, Credentials, TableInfo, DB};

fn main() -> Result<(), Box<dyn Error>> {
    let db = DB::new(
        Credentials::new(
            env::var("AWS_ACCESS_KEY_ID")?,
            env::var("AWS_SECRET_ACCESS_KEY")?,
        ),
        TableInfo::new(
            "key-attr-name",
            "value-attr-name",
            "table-name",
            "us-east-1".parse()?,
            None
        ),
        Reqwest::new(),
    );

    println!("{:#?}", db.set("foo", "bar")?);
    println!("{:#?}", db.get("foo")?);

    Ok(())
}
```

A few notable differences when comparing Tiny Dynamo to traditional DynamoDB clients is that this client assumes a single table, a very common case for most DynamodbDB applications, so you configure your client with that table name so you don't need to redundantly provide it with each request.

You will also find the interface is reduced to `get(key)` `set(key,value)`. This is intentional as this client is primarily focused on being a more comfortable fit for simple key-value applications.

## Features

### Tiny

Tiny Dynamo avoids packing carry-on luggage for anything you don't explicitly need for a simple key-value application. This includes an entire sdk and a transient line of dependencies. This allows it to fit more easily into smaller spaces and to deliver on Rust's zero cost promise of not paying for what you don't use.

### Simpler Data Modeling

A common laborious activity with DynamoDB based applications figuring our your application's data model first and then translating that to a DynamoDB's key space design and catalog of item attribute types. This is fine and expected for applications that require more advanced query access patterns. For simple key-value applications, this is just tax. Tiny DynamoDB assumes a key-value data model.

### Just the data plane

You can think of the DynamoDB API in terms of two planes: The _data_ plane*cation cases, and the *control\* plane, a set of APIs for provisioning the resources that will store your data. Combining these makes its surface area arbitrarily larger that it needs to be. Tiny Dynamo focuses on exposing just the data plane to retain a smaller surface area to learn.

### Sans I/O

Tiny Dynamo takes a [sans I/O](https://sans-io.readthedocs.io/) library approach. It defines a `Transport` trait which allows for any I/O library to implement how requests are transfered over the wire by provides none without an explict cargo feature toggled on

Below are the current available cargo features

#### `reqwest`

the `reqwest` feature provides a `reqwest_transport::Reqwest` backend for sending requests, currently using a blocking client. An async feature is planned for the future

```toml
[dependencies]
tiny_dynamo = { version = "0.1", features = ["reqwest"]}
```

#### `fastly`

The `fastly` feature provides a `fastly_transport::Fastly` backend for sending requests suitable for Fastlys Compute@Edge platform

```toml
[dependencies]
tiny_dynamo = { version = "0.1", features = ["fastly"]}
```

### BYOIO

If you would like to bring your own IO implementation you can define an implementation for a custom type

```rust
use tiny_dynamo::{Request, Transport};
use std::error::Error;

struct CustomIO;

impl Transport for CustomIO {
  fn send(&self, signed: Request) -> Result<(u16, String), Box<dyn Error>> {
    Ok(
      (200,"...".into())
    )
  }
}
```

Doug Tangren (softprops) 2021
