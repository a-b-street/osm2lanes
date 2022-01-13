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

![Example 1](https://upload.wikimedia.org/wikipedia/commons/0/0e/Gr%C3%BCnbergstra%C3%9Fe_2.JPG)

```json
[
    {"type": "travel_lane", "direction": "backward"},
    {"type": "separator", "markings": [{"style": "dashed_line"}]},
    {"type": "travel_lane", "direction": "backward"},
    {"type": "separator", "markings": [{"style": "solid_line"}, {"style": "solid_line"}]},
    {"type": "travel_lane", "direction": "forward"},
    {"type": "separator", "markings": [{"style": "dashed_line"}]},
    {"type": "travel_lane", "direction": "forward"},
    {"type": "separator", "markings": [{"style": "broken_line"}]},
    {"type": "travel_lane", "direction": "forward"},
]
```

Note:

- there is no sidewalk nor verge for pedestrians to walk here.
- lane separators are listed.
- we omit widths for simplicity. (TODO: if a guess is needed, e.g. on an average lane width, where is this guess made?)
- we omit turn markings for now. (TODO)

### Example 2 ###

A road with 2 marked lanes.

![Example 2](https://upload.wikimedia.org/wikipedia/commons/f/f0/A537_Cat_and_Fiddle_Road_-_geograph.org.uk_-_175899.jpg)

```json
[
    {"type": "shoulder", "direction": "both"},
    {"type": "separator", "markings": [{"style": "solid_line"}]},
    {"type": "travel_lane", "direction": "forward"},
    {"type": "separator", "markings": [{"style": "dashed_line"}]},
    {"type": "travel_lane", "direction": "backward"},
    {"type": "separator", "markings": [{"style": "solid_line"}]},
    {"type": "shoulder", "direction": "both"},
]
```

Note:

- this road has shoulders consisting of a soft grassy verge where pedestrians could walk in both directions and cars could pull over.
- this is in the UK, so traffic drives on the left.
- the direction of the lane is the predominant direction of travel,
  overtaking vehicles could travel in the opposite direction.

### Example 3 ###

A narrow road with one lane.

![Example 3](https://upload.wikimedia.org/wikipedia/commons/5/58/Back_Road_In_Ireland.jpg)

```json
[
    {"type": "shoulder", "direction": "both"},
    {"type": "travel_lane", "direction": "both"},
    {"type": "shoulder", "direction": "both"},
]
```

Note:

- no road markings.
- shoulders could be optional (TODO)?

### Examples 4 ###

Roads with no marked lanes.

![No Lanes India](https://upload.wikimedia.org/wikipedia/commons/thumb/5/5a/Bijupara-Khalari_Road_-_Jharkhand_1648.JPG/1920px-Bijupara-Khalari_Road_-_Jharkhand_1648.JPG)

![No Lanes Delhi](https://upload.wikimedia.org/wikipedia/commons/thumb/a/a8/Ratan_Lal_Market%2C_Kaseru_Walan%2C_Paharganj%2C_New_Delhi%2C_Delhi%2C_India_-_panoramio_%281%29.jpg/1280px-Ratan_Lal_Market%2C_Kaseru_Walan%2C_Paharganj%2C_New_Delhi%2C_Delhi%2C_India_-_panoramio_%281%29.jpg)

![No Lanes Dutch](https://upload.wikimedia.org/wikipedia/commons/e/e8/Fietsstraat.jpg)

#### Preferred Approach

Mark the continuous and uninterrupted surface as a travel lane in both directions.

Rationale:

- Vehicles fill the space as they need to.
- There are usually no consistent road positions.
- In some situations (e.g. crossing large vehicles on narrow roads),
  vehicles will invert the side they drive on.
- Rendering or routing clients must decide what to do with this information, as any other representation may be misleading.

Open Issue: The dutch example has paving stone rumble strips at the edges which cannot be faithfully represented (TODO).

#### Alternative

Two (or more) directional lanes with no road markings.
Only use this when vehicles consistently drive in lanes even though they are unmarked.
Example: A road with markings is repaved without markings, but road users habitually maintain their lanes.

### Missing Examples ###

TODO:

- Road markings with an area fill
- More bicycle lane examples
- More bus and taxi lane examples
- Trunk/Motorway/Highway

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
