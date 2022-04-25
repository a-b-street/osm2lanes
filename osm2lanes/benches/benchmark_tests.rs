use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use osm2lanes::locale::Locale;
use osm2lanes::test::get_tests;
use osm2lanes::transform::{tags_to_lanes, TagsToLanesConfig};

pub fn benchmark_tests(c: &mut Criterion) {
    let tests = get_tests();
    let mut group = c.benchmark_group("tests");
    for test in &tests {
        let locale = Locale::builder()
            .driving_side(test.driving_side)
            .iso_3166_option(test.iso_3166_2.as_deref())
            .build();
        let config = TagsToLanesConfig::new(
            !test.test_has_warnings(),
            test.test_include_separators() && test.expected_has_separators(),
        );
        group.measurement_time(std::time::Duration::from_millis(1000));
        group.warm_up_time(std::time::Duration::from_millis(500));
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
