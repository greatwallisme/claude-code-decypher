use claude_code_decypher::parser::Parser;
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use oxc_allocator::Allocator;

fn benchmark_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("parsing");

    // Small code sample
    let small_code = "var x = 1; function foo() { return x; }".repeat(10);

    group.bench_with_input(
        BenchmarkId::new("small", small_code.len()),
        &small_code,
        |b, code| {
            b.iter(|| {
                let allocator = Allocator::default();
                let parser = Parser::new(code.clone());
                black_box(parser.parse(&allocator).unwrap())
            })
        },
    );

    // Medium code sample
    let medium_code = r#"
        function process(data) {
            return data.map(x => x * 2);
        }
        const result = process([1, 2, 3]);
    "#
    .repeat(100);

    group.bench_with_input(
        BenchmarkId::new("medium", medium_code.len()),
        &medium_code,
        |b, code| {
            b.iter(|| {
                let allocator = Allocator::default();
                let parser = Parser::new(code.clone());
                black_box(parser.parse(&allocator).unwrap())
            })
        },
    );

    group.finish();
}

fn benchmark_large_file(c: &mut Criterion) {
    // Only run if vendors/claude exists
    if std::path::Path::new("./vendors/claude").exists() {
        let source = std::fs::read_to_string("./vendors/claude").unwrap();

        c.bench_function("parse_claude_code_bundle", |b| {
            b.iter(|| {
                let allocator = Allocator::default();
                let parser = Parser::new(source.clone());
                black_box(parser.parse(&allocator).unwrap())
            })
        });
    }
}

criterion_group!(benches, benchmark_parsing, benchmark_large_file);
criterion_main!(benches);
