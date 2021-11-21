"""
Test lane tag parsing.
"""
import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import pytest
from osm2lanes.core import DrivingSide, Lane, Road

Tags = dict[str, str]

TEST_FILE_PATH: Path = Path("data/tests.json")

with TEST_FILE_PATH.open(encoding="utf-8") as input_file:
    CONFIGURATIONS: list[dict[str, Any]] = [
        x for x in json.load(input_file) if "skip" not in x or not x["skip"]
    ]


@dataclass
class Case:
    """Lane test."""

    skip: bool
    # The OSM way unique identifier.
    way_id: int
    tags: Tags
    driving_side: DrivingSide
    output: list[Lane]

    @classmethod
    def from_structure(cls, structure: dict[str, Any]) -> "Case":
        """Parse test from configuration."""
        return cls(
            structure["skip"] if "skip" in structure else False,
            structure["way_id"],
            structure["tags"],
            DrivingSide(structure["driving_side"]),
            list(map(Lane.from_structure, structure["output"])),
        )


@pytest.mark.parametrize("test_configuration", CONFIGURATIONS)
def test_lanes(test_configuration: dict[str, Any]) -> None:
    """Test lane specification generation."""
    test = Case.from_structure(test_configuration)
    road: Road = Road(test.tags, test.driving_side)
    output: list[Lane] = road.parse()

    tags = "\n    ".join(f"{k}={v}" for k, v in test.tags.items())

    assert (
        output == test.output
    ), f"\nExpected: {test.output}\nActual:   {output}\nTags:\n    {tags}"
