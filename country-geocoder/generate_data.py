#!/usr/bin/python3

import json
import requests
import yaml

resp = requests.get("https://github.com/streetcomplete/countrymetadata/raw/master/data/isLeftHandTraffic.yml")
leftHandedCountryCodes = yaml.safe_load(resp.content)

# The smaller (600KB) "admin 0 countries" file from http://geojson.xyz
resp = requests.get("https://d2ad6b4ur7yvpq.cloudfront.net/naturalearth-3.3.0/ne_110m_admin_0_countries.geojson")
geojson = resp.json()

for feature in geojson["features"]:
    # Only keep ISO 3166-1 alpha-2 country code
    iso_a2 = feature["properties"]["iso_a2"]
    del feature["properties"]
    feature["properties"] = {
        "iso_a2": iso_a2,
        "left_handed": iso_a2 in leftHandedCountryCodes,
    }

    # Turn the Polygons into MultiPolygons, so the Rust can expect one thing
    if feature["geometry"]["type"] == "Polygon":
        feature["geometry"]["type"] = "MultiPolygon"
        feature["geometry"]["coordinates"] = [feature["geometry"]["coordinates"]]

with open("data.geojson", "w") as f:
    json.dump(geojson, f)
