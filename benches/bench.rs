use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tiny_dynamo::{Const, Credentials, Request, Table, DB};

fn get_item(db: DB) -> Result<Request, Box<dyn std::error::Error>> {
    db.get_item_req("test")
}

fn put_item(db: DB) -> Result<Request, Box<dyn std::error::Error>> {
    db.put_item_req("test", "value")
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("get_item", |b| {
        b.iter(|| {
            get_item(black_box(DB::new(
                Credentials::new("test", "test"),
                Table::new(
                    "test",
                    "key",
                    "value",
                    "us-east-1".parse()?,
                    Some("http://localhost:8000".into()),
                ),
                Const(200, "".into()),
            )))
        })
    });

    c.bench_function("put_item", |b| {
        b.iter(|| {
            put_item(black_box(DB::new(
                Credentials::new("test", "test"),
                Table::new(
                    "test",
                    "key",
                    "value",
                    "us-east-1".parse()?,
                    Some("http://localhost:8000".into()),
                ),
                Const(200, "".into()),
            )))
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
