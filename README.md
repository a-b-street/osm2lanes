osm2lanes
=========

See [discussion](https://github.com/a-b-street/abstreet/discussions/789) for
context.  This repo is currently just for starting this experiment.  No license
chosen yet.

Structure
---------

  * `data`
    * `tests.json`—tests, initially converted from Rust file
      `map_model/src/make/initial/lane_specs.rs` of A/B Street project.  Code is
      under Apache-2.0 License.
  * `kotlin`—Kotlin implementation.
  * `python`—Python 3.9 implementation.

Python
------

### Install ###

```shell
pip install .
```

### Run ###

```shell
osm2lanes ${INPUT_JSON_FILE} ${OUTPUT_JSON_FILE}
```

### Test ###

```shell
pytest
```

Kotlin
------

### Install ###

Run Gradle `jar` task.

### Run ###

```shell
java -jar kotlin/build/libs/osm2lanes.jar ${INPUT_JSON_FILE} ${OUTPUT_JSON_FILE}
```

### Test ###

Run Gradle `test` task.