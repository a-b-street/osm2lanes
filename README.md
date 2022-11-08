# osm2lanes

This project transforms OpenStreetMap tags to a specification of lanes on a
street. Start with the [web demo](https://a-b-street.github.io/osm2lanes) ([Source](#web-demo)).

See [discussion](https://github.com/a-b-street/abstreet/discussions/789) for further motivation. This repository is under lots of active churn.
Please get in touch before taking a dependency on it; we will clearly communicate a first public release.

## Structure

- `data`
  - `tests.yml` - Test cases.
    `spec-lanes.json` - JSON specification.
- `osm-tags` - Tags datatype library
- `osm-tag-schemes` - Tagging schemes library
- `osm2lanes` - Tags to lanes library
- `osm2lanes-web` - Website with lane viewer
- `osm2lanes-npm` - NPM package
- `osm2lanes-cli` - CLI tool

## Design

The primary input is a `Tags` map data structure.
`osm-tag-schemes` and `osm2lanes` are aware of many tagging schemes, so these are parsed from the Tags.
The schemes may be mutually compatible or incompatible, so this must be reconciled.
A road is then built up with lanes, inside-out.
Separators are added once all the lanes are added.

## Contributing

[See the contribution guide.](./CONTRIBUTING.md)

## Using This Library

Bindings to other languages like JVM, C, or Node.js will be worked on.
Let us know what you need by [raising an issue](https://github.com/a-b-street/osm2lanes/issues/new).

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
These will be the best guess based on regional, national, or other standards.
We always assume the lanes are new, with the latest road markings.
When lane separators may change down a way,
e.g. a dashed line allowing overtaking on straight sections but a double solid line on curves,
the most permissive line arrangement will be assumed (e.g. a single dashed line);
we use a fail-deadly approach to emphasize that
osm2lanes should not be used in road safety applications.

For all examples we omit details for simplicity, such as widths, turn markings, access, etc.

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
    {"type": "travel_lane", "direction": "forward"}
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
    {"type": "shoulder", "direction": "both"}
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
    {"type": "shoulder", "direction": "both"}
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

Open Issue: The Dutch example has paving stone rumble strips at the edges which cannot be faithfully represented (TODO).

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

### Parallel ways

Sidewalks, cyclepaths, and roads split into dual carriageways are some examples in OSM where there are multiple parallel ways close together. Human judgment may be to group them into one logical road. `osm2lanes` does not attempt to do this; it just parses one OSM way at a time. See [osm2streets](https://github.com/a-b-street/osm2streets) for higher-level grouping.

## Web demo

The web demo at <https://a-b-street.github.io/osm2lanes> provides an easy way to test OSM tags and see the generated results. <https://a-b-street.github.io/osm2lanes/editor/> is a prototype tool to edit the resulting lanes.
