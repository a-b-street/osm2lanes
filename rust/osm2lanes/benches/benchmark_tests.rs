use std::time::Duration;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use osm2lanes::test::get_tests;
use osm2lanes::{tags_to_lanes, Locale, TagsToLanesConfig};

pub fn benchmark_tests(c: &mut Criterion) {
    let tests = get_tests();
    let mut group = c.benchmark_group("benchmark_tests");
    for test in tests.iter() {
        let locale = Locale::builder()
            .driving_side(test.driving_side)
            .iso_3166_option(test.iso_3166_2.as_deref())
            .build();
        let config = TagsToLanesConfig::new(
            !test.test_ignore_warnings(),
            test.test_include_separators() && test.expected_has_separators(),
        );
        group.measurement_time(Duration::from_millis(1000));
        group.warm_up_time(Duration::from_millis(500));
        group.bench_with_input(BenchmarkId::from_parameter(test), test, |b, test| {
            b.iter(|| {
                assert!(tags_to_lanes(&test.tags, &locale, &config).is_ok());
            });
        });
    }
    group.finish();
}

criterion_group!(benches, benchmark_tests);
criterion_main!(benches);
