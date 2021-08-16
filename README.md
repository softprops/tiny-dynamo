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

This quote comes directly from the [Amazon DynamoDB docs](https://aws.amazon.com/dynamodb/). This has some implications that are less than idea for simple key value applications. It can be overly complicated and sometimes daunting an to say the least.

Tiny Dynamo aims to leverge the useful parts but exposing a much simpler get/set api you might expect from a key value interface.

```rust
// storing a value
db.set("foo", "bar")?;

// geting a value.
db.get("foo")?;
```

## Features

### Tiny

Tiny Dynamo avoids packing carry-on luggage for anything you don't explicitly need for a simple key value application. This includes an entire sdk and transient line of dependencies. This allows it to fit more easily into smaller spaces and to deliver on Rust's zero cost promise of not paying for what you don't use.

### Simpler Data Modeling

A common laborious activity with DynamoDB applications is to figure our your application's data model first then translate that to a DynamoDBee key space design and item attributes. This is expected for applications that require more advanced access patterns. For simple key value applications, this is just tax. Tiny DynamoDB assumes key value data model. How you serilize your value is up to you.

### Just the data plane

You can think of the DynamoDB API in terms of two planes: The data plane, where you all of your time in 99% of application cases, and the control plane, an api for provisioning the resources that will store your data. Combining these makes its surface area arbitrarily larger that it needs to be. Tiny Dynamo focuses on exposing just the data plane to retain a smaller surface area to learn.

### Sans I/O

Tiny Dynamo takes a [sans I/O](https://sans-io.readthedocs.io/) library approach. By default it defines a `Requests` trait which allows for any I/O library to implement how requests are transfered over the wire by provides none without an implicit feature toggled on

#### Reqwest

the `reqwest` feature provides a `reqwest_requests::Reqwest` backend for sending requests. Currently using a blocking client. An async feature is planned for the future

```toml
[dependencies]
tiny_dynamo = { version = "0.1", features = ["reqwest"]}
```

#### Fastly

The `fastly` feature provides a `fastly_requests::Fastly` backend for sending requests suitable for Fastlys Compute@Edge platform

```toml
[dependencies]
tiny_dynamo = { version = "0.1", features = ["fastly"]}
```

Doug Tangren (softprops) 2021
