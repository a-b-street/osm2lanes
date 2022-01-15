osm2lanes
=========

This project transforms OpenStreetMap tags to a specification of lanes on a
street. Start with the [web demo](https://a-b-street.github.io/osm2lanes).

See [discussion](https://github.com/a-b-street/abstreet/discussions/789) for
context.  This repo is currently just for starting this experiment.  No license
chosen yet.

Structure
---------

  * `data`
    * `tests.json`—tests, initially converted from Rust file
      `map_model/src/make/initial/lane_specs.rs` of A/B Street project.  Code is
      under Apache-2.0 License.
  * `kotlin`—[Kotlin implementation](#kotlin).
  * `python`—[Python 3.9 implementation](#python).
  * `rust`—[Rust implementation](#rust).

Example
-------

Input JSON file with road OpenStreetMap tags:

```json
{
    "lanes": "2",
    "oneway": "yes",
    "sidewalk": "both",
    "cycleway:left": "lane"
}
```

Output lane specifications from left to write:

```json
[
    {"type": "sidewalk", "direction": "backward"},
    {"type": "cycleway", "direction": "forward"},
    {"type": "travel_lane", "direction": "forward"},
    {"type": "travel_lane", "direction": "forward"},
    {"type": "sidewalk", "direction": "forward"}
]
```

Kotlin
------

### Run with Gradle ###

```shell
cd kotlin
gradle run --args "${INPUT_FILE} ${OUTPUT_FILE}"
```

### Install and test ###

Create JAR file with `gradle jar` and test with `gradle test`.

### Run with Java ###

```shell
java -jar kotlin/build/libs/osm2lanes.jar ${INPUT_FILE} ${OUTPUT_FILE}
```

Python
------

### Install and test ###

```shell
cd python
pip install .
cd ..
pytest
```

### Run ###

```shell
osm2lanes ${INPUT_FILE} ${OUTPUT_FILE}
```

Rust
------

### Install and test ###

After [installing rust](https://www.rust-lang.org/tools/install), run:

```shell
cd rust/osm2lanes
cargo test
```

### Run ###

```shell
cargo run -- ${INPUT_FILE} ${OUTPUT_FILE}
```
