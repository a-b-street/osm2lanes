#!/bin/bash

# To be run by someone with push rights to gh-pages
# TODO: automate this...

set -e
set -u
set -o pipefail
set -x

BRANCH="$(git rev-parse --abbrev-ref HEAD)"
if [[ "$BRANCH" != "master" ]]; then
  echo 'Aborting script';
  exit 1;
fi
if [ -z "$1" ]
  then
    echo "No argument supplied"
fi
trunk --config Release.toml build
git switch gh-pages
git rm index.html
git rm index-*.js
git rm index-*.wasm
git rm main-*.css
cp dist/* ./
git add index.html
git add index-*.js
git add index-*.wasm
git add main-*.css
git commit -am "$1"
git push origin HEAD
git switch master
