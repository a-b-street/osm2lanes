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
    NO_CYCLEWAY = "no_cycleway"
    NO_SIDEWALK = "no_sidewalk"


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

        lanes: list[Lane] = []

        # Driveways

        if "lanes" in self.tags:

            number: int = int(self.tags["lanes"])
            forward_driveway: Lane = Lane(LaneType.DRIVEWAY, Direction.FORWARD)
            backward_driveway: Lane = Lane(
                LaneType.DRIVEWAY, Direction.BACKWARD
            )

            if self.tags.get("oneway") == "yes":
                lanes = [forward_driveway] * number
            else:
                half: int = int(number / 2.0)
                lanes = [backward_driveway] * half + [forward_driveway] * (
                    number - half
                )

        def add(new_lanes: list[Lane]) -> list[Lane]:
            if direction == "left":
                return new_lanes + lanes
            if direction == "right":
                return lanes + new_lanes

        def get_direction() -> Direction:
            return (
                Direction.FORWARD if direction == "left" else Direction.BACKWARD
            )

        # Cycleways

        for direction in "left", "right":
            if self.tags.get(f"cycleway:{direction}") == "lane":
                lanes = add([Lane(LaneType.CYCLEWAY, get_direction())])
            elif self.tags.get(f"cycleway:{direction}") == "track":
                lanes = add(
                    [
                        Lane(LaneType.CYCLEWAY, Direction.BACKWARD),
                        Lane(LaneType.CYCLEWAY, Direction.FORWARD),
                    ],
                )

        # Sidewalks

        if self.tags.get("sidewalk") == "both":
            lanes = [Lane(LaneType.SIDEWALK, Direction.BACKWARD)] + lanes
            lanes += [Lane(LaneType.SIDEWALK, Direction.FORWARD)]
        elif self.tags.get("sidewalk") == "none":
            lanes = [Lane(LaneType.NO_SIDEWALK, Direction.BACKWARD)] + lanes
            lanes += [Lane(LaneType.NO_SIDEWALK, Direction.FORWARD)]

        return lanes
