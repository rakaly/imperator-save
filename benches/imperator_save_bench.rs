use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use imperator_save::ImperatorExtractor;

const HEADER: &'static [u8] = include_bytes!("../tests/fixtures/header");

pub fn binary_header_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("binary-header");
    group.throughput(Throughput::Bytes(HEADER.len() as u64));
    group.bench_function("owned", |b| {
        b.iter(|| {
            ImperatorExtractor::builder()
                .extract_header_owned(HEADER)
                .unwrap()
        });
    });
    group.bench_function("borrowed", |b| {
        b.iter(|| {
            ImperatorExtractor::builder()
                .extract_header_borrowed(HEADER)
                .unwrap()
        });
    });
    group.finish();
}

criterion_group!(benches, binary_header_benchmark);
criterion_main!(benches);
