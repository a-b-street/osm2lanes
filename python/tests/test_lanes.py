"""
Test lane tag parsing.
"""
import yaml
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Optional

import pytest
from osm2lanes.core import DrivingSide, Lane, Road

Tags = dict[str, str]

TEST_FILE_PATH: Path = Path("data/tests.yml")

with TEST_FILE_PATH.open(encoding="utf-8") as input_file:
    CONFIGURATIONS: list[dict[str, Any]] = [
        test
        for test in yaml.safe_load(input_file)
        if not test.get("skip_python")
    ]


@dataclass
class Case:
    """Lane test."""

    skip: bool
    # The OSM way unique identifier.
    description: Optional[str]
    way_id: Optional[int]
    tags: Tags
    driving_side: DrivingSide
    lanes: list[Lane]

    @classmethod
    def from_structure(cls, structure: dict[str, Any]) -> "Case":
        """Parse test from configuration."""
        return cls(
            skip=bool(structure.get("skip_python")),
            way_id=structure.get("way_id"),
            description=structure.get("description"),
            tags=structure["tags"],
            driving_side=DrivingSide(structure["driving_side"]),
            lanes=[
                Lane.from_structure(l)
                for l in structure.get("output") or structure["road"]["lanes"]
                if l["type"] != "separator"
            ],
        )


@pytest.mark.parametrize("test_configuration", CONFIGURATIONS)
def test_lanes(test_configuration: dict[str, Any]) -> None:
    """Test lane specification generation."""
    test = Case.from_structure(test_configuration)
    road: Road = Road(test.tags, test.driving_side)
    output: list[Lane] = road.parse()

    tags = "\n    ".join(f"{k}={v}" for k, v in test.tags.items())

    assert (
        output == test.lanes
    ), f"{test.way_id}: {test.description}\nGot:      {output}\nExpected: {test.lanes}\nTags:\n    {tags}"
