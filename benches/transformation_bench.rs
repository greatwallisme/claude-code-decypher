use claude_code_decypher::{
    parser::Parser,
    transformer::{codegen::CodeGenerator, Transformer},
};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use oxc_allocator::Allocator;

fn benchmark_beautification(c: &mut Criterion) {
    let code = "var x=1;var y=2;function f(){return x+y}".repeat(100);

    c.bench_function("beautify_code", |b| {
        let allocator = Allocator::default();
        let parser = Parser::new(code.clone());
        let parse_result = parser.parse(&allocator).unwrap();

        b.iter(|| {
            let generator = CodeGenerator::new(&allocator, parse_result.program());
            black_box(generator.generate().unwrap())
        })
    });
}

fn benchmark_variable_renaming(c: &mut Criterion) {
    let code = "var QB9=1;var IB9=2;function test(){return QB9+IB9}".repeat(50);

    c.bench_function("variable_renaming", |b| {
        let allocator = Allocator::default();
        let parser = Parser::new(code.clone());
        let parse_result = parser.parse(&allocator).unwrap();

        b.iter(|| {
            let transformer = Transformer::new(parse_result.program());
            black_box(transformer.generate_rename_map().unwrap())
        })
    });
}

fn benchmark_module_splitting(c: &mut Criterion) {
    let code = r#"
        const tool = "bash";
        const api = "https://api.anthropic.com";
        function process() {}
    "#.repeat(50);

    c.bench_function("module_splitting", |b| {
        let allocator = Allocator::default();
        let parser = Parser::new(code.clone());
        let parse_result = parser.parse(&allocator).unwrap();

        b.iter(|| {
            let transformer = Transformer::new(parse_result.program());
            black_box(transformer.split_into_modules(
                claude_code_decypher::transformer::split::SplitStrategy::Hybrid
            ).unwrap())
        })
    });
}

criterion_group!(
    benches,
    benchmark_beautification,
    benchmark_variable_renaming,
    benchmark_module_splitting
);
criterion_main!(benches);
