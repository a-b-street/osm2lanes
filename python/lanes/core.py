"""
Lane tag parsing.
"""
from dataclasses import dataclass
from enum import Enum

Tags = dict[str, str]


class DrivingSide(Enum):
    """Bidirectional traffic practice."""

    RIGHT = "right"
    LEFT = "left"


class Direction(Enum):
    """Lane direction relative to way direction."""

    FORWARD = "forward"
    BACKWARD = "backward"


class LaneType(Enum):
    """Lane designation."""

    SIDEWALK = "sidewalk"
    CYCLEWAY = "cycleway"
    DRIVEWAY = "driveway"


@dataclass
class Lane:
    """Lane specification."""

    type_: LaneType
    direction: Direction

    @classmethod
    def from_structure(cls, structure: dict[str, str]) -> "Lane":
        """Parse lane specification from structure."""
        return cls(
            LaneType(structure["type"]), Direction(structure["direction"])
        )


@dataclass
class Road:
    """OpenStreetMap way or relation described road part."""

    tags: Tags

    def parse(self) -> list[Lane]:
        """Process road tags."""
        if "lanes" in self.tags:
            return [Lane(LaneType.DRIVEWAY, Direction.FORWARD)] * int(
                self.tags["lanes"]
            )

        return []
