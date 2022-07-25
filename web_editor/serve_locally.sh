#!/bin/bash

wasm-pack build --dev --target web ../osm2lanes-npm
python3 -m http.server
