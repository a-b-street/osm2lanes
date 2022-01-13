osm2lanes
=========

See [discussion](https://github.com/a-b-street/abstreet/discussions/789) for
context.  This repo is currently just for starting this experiment.  No license
chosen yet.

Structure
---------

  * `data`
    * `tests.json`—tests, initially converted from Rust file.
      `spec-lanes.json`—json specification.
  * `kotlin`—[Kotlin implementation](#kotlin).
  * `python`—[Python 3.9 implementation](#python).
  * `rust`—[Rust implementation](#rust).

Example
-------

Input: The tags of an OpenStreetMap way.
JSON formatted example:

```json
{
    "lanes": "2",
    "oneway": "yes",
    "sidewalk": "both",
    "cycleway:left": "lane"
}
```

Output: A list of lanes from left to write according to the [specification](./data/spec-lanes.json).
JSON formatted example:

```json
[
    {"type": "travel_lane", "direction": "backward"},
    {"type": "travel_lane", "direction": "backward"},
    {"type": "travel_lane", "direction": "forward"},
    {"type": "travel_lane", "direction": "forward"},
    {"type": "sidewalk", "direction": "forward"}
]
```

Lane Definition
---------------

It is important to determine a strict definition for a lane.
An OpenStreetMap way, usually a `highway=*`, can in some cases be divided into lanes,
where each lane extends from the start to the end of the way.
A lane's direction of travel is not necessarily the same as the direction of the way.
A lane represents a continuous area with no formal separation of vehicles.
Lanes are often separated by something visible like painted lines or raised curbs,
but these are not necessary for lanes to exist.


### Example 1 ###

A road with 5 marked lanes.

```json
[
    {"type": "travel_lane", "direction": "backward"},
    {"type": "travel_lane", "direction": "backward"},
    {"type": "travel_lane", "direction": "forward"},
    {"type": "travel_lane", "direction": "forward"},
    {"type": "travel_lane", "direction": "forward"},
]
```

![Example 1](https://upload.wikimedia.org/wikipedia/commons/thumb/0/0e/Gr%C3%BCnbergstra%C3%9Fe_2.JPG/240px-Gr%C3%BCnbergstra%C3%9Fe_2.JPG)

Note: there is no sidewalk or verge for pedestrians to walk here.

### Example 2 ###

A road with 2 marked lanes.

```json
[
    {"type": "shoulder", "direction": "both"},
    {"type": "travel_lane", "direction": "forward"},
    {"type": "travel_lane", "direction": "backward"},
    {"type": "shoulder", "direction": "both"},
]
```

![Example 2](https://upload.wikimedia.org/wikipedia/commons/f/f0/A537_Cat_and_Fiddle_Road_-_geograph.org.uk_-_175899.jpg)

Note:

- there is are shoulders where pedestrians could walk in both directions.
- this is in the UK, so traffic drives on the left.
- the direction is the predominant direction,
  overtaking vehicles could travel in the opposite direction.

### Example 3 ###

A narrow road with one lane.

```json
[
    {"type": "shoulder", "direction": "both"},
    {"type": "travel_lane", "direction": "both"},
    {"type": "shoulder", "direction": "both"},
]
```

![Example 3](https://upload.wikimedia.org/wikipedia/commons/5/58/Back_Road_In_Ireland.jpg)

TODO: shoulders?

### Discussion Example ###

A road with no marked lanes.

![No Lanes India](https://upload.wikimedia.org/wikipedia/commons/thumb/5/5a/Bijupara-Khalari_Road_-_Jharkhand_1648.JPG/1920px-Bijupara-Khalari_Road_-_Jharkhand_1648.JPG)

![No Lanes Delhi](https://upload.wikimedia.org/wikipedia/commons/thumb/a/a8/Ratan_Lal_Market%2C_Kaseru_Walan%2C_Paharganj%2C_New_Delhi%2C_Delhi%2C_India_-_panoramio_%281%29.jpg/1280px-Ratan_Lal_Market%2C_Kaseru_Walan%2C_Paharganj%2C_New_Delhi%2C_Delhi%2C_India_-_panoramio_%281%29.jpg)

![No Lanes Dutch](https://upload.wikimedia.org/wikipedia/commons/e/e8/Fietsstraat.jpg)

Option 1: Mark the entire road as a single lane in both directions

- Vehicles fill the space as they need to
- There are no dominant road positions
- In some situations (e.g. crossing large vehicles on narrow roads),
  vehicles will invert the side they drive on

Option 2: Two (or more) directional lanes with no road markings

- Vehicles tend to drive in a consistent road position

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
