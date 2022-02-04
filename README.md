# osm2lanes

This project transforms OpenStreetMap tags to a specification of lanes on a
street. Start with the [web demo](https://a-b-street.github.io/osm2lanes) ([Source](#web-demo)).

See [discussion](https://github.com/a-b-street/abstreet/discussions/789) for further motivation. This repository is undr lots of active churn. Please get in touch before taking a dependency on it; we will clearly communicate a first public release.

## Structure

- `data`
  - `tests.yml`—tests, initially converted from Rust file.
    `spec-lanes.json`—json specification.
- `kotlin`—[Kotlin implementation](#kotlin).
- `python`—[Python 3.9 implementation](#python).
- `rust`—[Rust implementation](#rust).

## Lane Definition

It is important to determine a strict definition for a lane.
An OpenStreetMap way, usually a `highway=*`, can in some cases be divided into lanes,
where each lane extends from the start to the end of the way.
A lane's direction of travel is not necessarily the same as the direction of the way.
A lane represents a continuous area with no formal separation of vehicles.
Lanes are often separated by something visible like painted lines or raised curbs,
but these are not necessary for lanes to exist.

Whilst the type of lanes are usually determined from the OSM tags,
osm2lanes also reports on lane separators and lane widths which are usually assumptions.
These will be a best guess based on regional, national, or other standards.
We always assume the lanes are new, with the latest road markings.
When lane separators may change down a way,
e.g. a dashed line allowing overtaking on straight sections but a double solid line on curves,
the most permissive line arrangement will be assumed (e.g. a single dashed line);
we use a fail-deadly approach to emphasize that
osm2lanes should not be used in road safety applications.

For all examples:

- we omit access, this mimics the access tags for a lane in OSM.
- we omit widths, for simplicity. osm2lanes will first try to determine widths from OSM tags, but it can fallback to making assumptions itself.
- we omit turn markings (for now, TODO)

### Example 1

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

### Example 2

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

### Example 3

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

### Examples 4

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

### Missing Examples

TODO:

- Road markings with an area fill
- More bicycle lane examples
- More bus and taxi lane examples
- Trunk/Motorway/Highway

## Kotlin

### Run with Gradle

```shell
cd kotlin
gradle run --args "${INPUT_FILE} ${OUTPUT_FILE}"
```

### Install and test

Create JAR file with `gradle jar` and test with `gradle test`.

### Run with Java

```shell
java -jar kotlin/build/libs/osm2lanes.jar ${INPUT_FILE} ${OUTPUT_FILE}
```

## Python

### Install and test

```shell
cd python
pip install .
cd ..
pytest
```

### Run

```shell
osm2lanes ${INPUT_FILE} ${OUTPUT_FILE}
```

## Rust

### Install and test

After [installing rust](https://www.rust-lang.org/tools/install), run:

```shell
cd rust/osm2lanes
cargo test
```

Before sending a PR, please run `cargo +nightly fmt` to format the code. Note that while the crate targets the current stable Rust, the project requires the nightly toolchain for formatting. You can install it by doing `rustup toolchain install nightly` -- this won't change the default toolchain from stable.

### Run

```shell
cargo run -- ${INPUT_FILE} ${OUTPUT_FILE}
```

## Web demo

The web demo at https://a-b-street.github.io/osm2lanes provides an easy way to test OSM tags and see the generated results.

### Dev

- The web demo is updated with every push on `main`, [see Workflow](./.github/workflows/web.yml)
- The html website is part of the rest implementation at [`/rust/osm2lanes-web` ](./rust/osm2lanes-web)

## Contributing

Pull requests very welcome! Once the dust from initial development settles, we'll have a better TODO list. All contributors agree to license their work under Apache 2.0.
