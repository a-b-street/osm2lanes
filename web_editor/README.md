# osm2lanes editor

This is an early prototype of a tool to edit lane tagging in OSM with a
cross-section view, like [Streetmix](streetmix.net).

To build it, [install
wasm-pack](https://rustwasm.github.io/wasm-pack/installer/), then run
`./serve_locally.sh`. You can edit HTML, CSS, and JS and just refresh the page
immediately. If you modify the Rust code, you must re-run `serve_locally.sh`,
which will recompile.

## Design notes

- Render cards with proportional width
- Be able to diff OSM tags with a nice green/red/yellow table
- Think through interactions for new lanes
- How should we represent uncertainty in assumed defaults like width, coloring?
- Reference: https://github.com/openstreetmap/iD/issues/387
