from setuptools import setup

setup(
    name="osm2lanes",
    entry_points={
        "console_scripts": ["osm2lanes=osm2lanes.__main__:run"],
    },
    python_requires=">=3.9",
)