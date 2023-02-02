# country-geocoder

This crate takes a WGS84 coordinate and determines the [two-letter ISO country
code](https://en.wikipedia.org/wiki/ISO_3166-1_alpha-2) where it exists. It
also determines if that country drives on the left or right. It does this by
offline geocoding, using a bundled file.

`data.geojson` is produced by `generate_data.py`. Thanks to
<http://geojson.xyz> for preprocessing public domain Natural Earth data of
country boundaries, and to Tobias Zwick for
[countrymetadata](https://github.com/streetcomplete/countrymetadata) and for
[inspiring this approach](https://github.com/westnordost/countryboundaries).
