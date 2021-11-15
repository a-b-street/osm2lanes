"""
osm2lanes entry point.
"""
import argparse
import json
import sys
from pathlib import Path

from osm2lanes.core import Road


def run() -> None:
    """
    Command-line interface.

    Read OpenStreetMap tags from input JSON file and write lane specifications
    into output JSON file.
    """
    parser: argparse.ArgumentParser = argparse.ArgumentParser()
    parser.add_argument("input", help="input JSON file path")
    parser.add_argument("output", help="output JSON file path")

    arguments: argparse.Namespace = parser.parse_args(sys.argv[1:])

    with Path(arguments.input).open(encoding="utf-8") as input_file:
        tags: dict[str, str] = json.load(input_file)

    with Path(arguments.output).open("w+", encoding="utf-8") as output_file:
        json.dump(
            [lane.to_structure() for lane in Road(tags).parse()], output_file
        )
