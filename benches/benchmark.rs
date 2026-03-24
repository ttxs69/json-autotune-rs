use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};

fn gen_small() -> String { r#"{"name":"Alice","age":30,"active":true}"#.into() }

fn gen_medium() -> String {
    let items: Vec<String> = (0..100).map(|i| format!(r#"{{"id":{},"name":"User{}"}}"#, i, i)).collect();
    format!(r#"{{"users":[{}]}}"#, items.join(","))
}

fn gen_large() -> String {
    let items: Vec<String> = (0..1000).map(|i| format!(r#"{{"id":{},"data":[1,2,3]}}"#, i)).collect();
    format!(r#"{{"items":[{}]}}"#, items.join(","))
}

fn bench(c: &mut Criterion) {
    let small = gen_small();
    let medium = gen_medium();
    let large = gen_large();

    c.bench_function("json-autotune/small", |b| {
        b.iter(|| json_autotune::parse(black_box(&small)).unwrap())
    });
    c.bench_function("serde_json/small", |b| {
        b.iter(|| serde_json::from_str::<serde_json::Value>(black_box(&small)).unwrap())
    });

    c.bench_function("json-autotune/medium", |b| {
        b.iter(|| json_autotune::parse(black_box(&medium)).unwrap())
    });
    c.bench_function("serde_json/medium", |b| {
        b.iter(|| serde_json::from_str::<serde_json::Value>(black_box(&medium)).unwrap())
    });

    let mut g = c.benchmark_group("large");
    g.throughput(Throughput::Bytes(large.len() as u64));
    g.bench_function("json-autotune", |b| b.iter(|| json_autotune::parse(black_box(&large)).unwrap()));
    g.bench_function("serde_json", |b| b.iter(|| serde_json::from_str::<serde_json::Value>(black_box(&large)).unwrap()));
    g.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);