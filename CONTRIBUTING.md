# Contribution

Pull requests very welcome! Once the dust from initial development settles, we'll have a better TODO list. 

**All contributors agree to license their work under Apache 2.0.**

## Git Hooks

Set your `.git/hooks/*` to `sh .githooks/check`.

## Rust

### Install and test

After [installing rust](https://www.rust-lang.org/tools/install), run:

```shell
cd rust/osm2lanes
cargo test
cargo install trunk
trunk serve
```

Before sending a PR, please run `cargo +nightly fmt` to format the code.
Note that while the crate targets the current stable Rust
the project requires the nightly toolchain for formatting.
You can install it by doing `rustup toolchain install nightly`;
this won't change your default toolchain for other work from stable.

### Run

```shell
osm2lanes ${INPUT_FILE} ${OUTPUT_FILE}
```

### Dev

```shell
cargo install trunk
trunk serve
```

- The web demo is updated with every push on `main`, [see Workflow](./.github/workflows/web.yml)
- The html website is part of the rest implementation at [`/rust/osm2lanes-web` ](./rust/osm2lanes-web)

### JS bindings

You can call the Rust library in JavaScript through WASM bindings.

```shell
cd rust/osm2lanes-npm
wasm-pack build --dev --target web
cd ../../web
ln -s ../rust/osm2lanes-npm/pkg/ .
./serve_locally.py
```

Then open the link in your web browser and click a road.
