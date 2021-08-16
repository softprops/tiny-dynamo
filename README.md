<div align="center">
  📦 🤏
</div>

<h1 align="center">
  tiny dynamo
</h1>

<p align="center">
   A tiny, simpler, key-value focused interface for AWS DynamoDB
</p>

<div align="center">
  <a href="https://github.com/softprops/tiny-dynamo/actions">
		<img src="https://github.com/softprops/tiny-dynamo/workflows/Main/badge.svg"/>
	</a>
</div>

## Features

### Sans I/O

Tiny Dynamodb takes a [sans I/O] library approach. By default it defines a `Requests` trait which allows for any I/O library to implement how requests are transfered over the wire by provides none without an implicit feature toggled on

#### Reqwest

the `reqwest` feature provides a `reqwest_requests::Reqwest` backend for sending requests. Currently using a blocking click.

```toml
[dependencies]
tiny_dynamo = { version = "0.1", features = ["reqwest"]}
```

#### Fastly

the `fastly` feature provides a `fastly_requests::Fastly` backend for sending requests suitable for Fastlys Compute@Edge platform

```toml
[dependencies]
tiny_dynamo = { version = "0.1", features = ["fastly"]}
```

Doug Tangren (softprops) 2021
