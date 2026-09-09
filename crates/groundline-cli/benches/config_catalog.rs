use divan::{Bencher, black_box};
use groundline_cli::config_catalog::Catalog;
use serde_json::json;

#[global_allocator]
static ALLOC: divan::AllocProfiler = divan::AllocProfiler::system();

fn main() {
    divan::main();
}

fn fixture(count: usize) -> Vec<u8> {
    serde_json::to_vec(&json!({"models": (0..count).map(|index| json!({
        "slug": format!("model-{index}"), "default_reasoning_level": "xhigh",
        "supported_reasoning_levels": [{"effort":"xhigh"}],
        "base_instructions": "synthetic".repeat(1024)
    })).collect::<Vec<_>>()}))
    .unwrap()
}

// The previous repair flow parsed and validated the same input for both audits.
#[divan::bench(args = [8, 128, 512])]
fn parse_per_audit(bencher: Bencher, count: usize) {
    let bytes = fixture(count);
    bencher.bench_local(|| {
        for _ in 0..2 {
            let catalog = Catalog::parse(black_box(&bytes)).unwrap();
            black_box(catalog.supports_effort("model-7", "xhigh"));
        }
    });
}

#[divan::bench(args = [8, 128, 512])]
fn reuse_for_both_audits(bencher: Bencher, count: usize) {
    let bytes = fixture(count);
    bencher.bench_local(|| {
        let catalog = Catalog::parse(black_box(&bytes)).unwrap();
        for _ in 0..2 {
            black_box(catalog.supports_effort("model-7", "xhigh"));
        }
    });
}
