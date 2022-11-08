#!/usr/bin/python3

import json
import requests
import yaml

resp = requests.get("https://github.com/streetcomplete/countrymetadata/raw/master/data/isLeftHandTraffic.yml")
countryCodes = yaml.safe_load(resp.content)

# The smaller (600KB) "admin 0 countries" file from http://geojson.xyz
resp = requests.get("https://d2ad6b4ur7yvpq.cloudfront.net/naturalearth-3.3.0/ne_110m_admin_0_countries.geojson")
geojson = resp.json()

features = []
for feature in geojson["features"]:
    if feature["properties"]["iso_a2"] in countryCodes:
        del feature["properties"]
        features.append(feature)
geojson["features"] = features

with open("data.geojson", "w") as f:
    json.dump(geojson, f)
