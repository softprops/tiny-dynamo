use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tiny_dynamo::{Credentials, Request, Static, TableInfo, DB};

fn get_item(db: DB) -> Result<Request, Box<dyn std::error::Error>> {
    db.get_item_req("test")
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("get_item", |b| {
        b.iter(|| {
            get_item(black_box(DB {
                credentials: Credentials {
                    aws_access_key_id: "test".into(),
                    aws_secret_access_key: "test".into(),
                },
                table_info: TableInfo {
                    key_name: "key".into(),
                    value_name: "value".into(),
                    table_name: "test".into(),
                    region: "us-east-1".into(),
                    endpoint: Some("http://localhost:8000".into()),
                },
                requests: Box::new(Static(200, "".into())),
            }))
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
