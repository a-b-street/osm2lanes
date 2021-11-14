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
    """Lane direction relative to OpenStreetMap way direction."""

    FORWARD = "forward"
    BACKWARD = "backward"


class LaneType(Enum):
    """Lane designation."""

    SIDEWALK = "sidewalk"
    CYCLEWAY = "cycleway"
    DRIVEWAY = "driveway"
    PARKING_LANE = "parking_lane"
    C = "C"
    S = "S"


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

        main_lanes: list[Lane] = []
        sidewalk_right: list[Lane] = []
        sidewalk_left: list[Lane] = []
        cycleway_right: list[Lane] = []
        cycleway_left: list[Lane] = []

        if "lanes" in self.tags:
            if self.tags.get("oneway") == "yes":
                main_lanes = [Lane(LaneType.DRIVEWAY, Direction.FORWARD)] * int(
                    self.tags["lanes"]
                )

        if self.tags.get("sidewalk") == "both":
            sidewalk_left = [Lane(LaneType.SIDEWALK, Direction.BACKWARD)]
            sidewalk_right = [Lane(LaneType.SIDEWALK, Direction.FORWARD)]

        if self.tags.get("cycleway:left") == "lane":
            cycleway_left = [Lane(LaneType.CYCLEWAY, Direction.FORWARD)]

        return (
            sidewalk_left
            + cycleway_left
            + main_lanes
            + cycleway_right
            + sidewalk_right
        )
