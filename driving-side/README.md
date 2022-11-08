# driving-side

This crate takes a WGS84 coordinate and determines if it's located in a country
that drives on the left or right. It does this by offline geocoding -- just
checking if the point is inside any polygon of a left-sided country or not. 

`data.geojson` is produced by `generate_data.py`. Thanks to
<http://geojson.xyz> for preprocessing public domain Natural Earth data of
country boundaries, and to Tobias Zwick for
[countrymetadata](https://github.com/streetcomplete/countrymetadata) and for
[inspiring this approach](https://github.com/westnordost/countryboundaries).
