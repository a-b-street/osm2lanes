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

    SHARED_LEFT_TURN = "shared_left_turn"
    SHOULDER = "shoulder"


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

    def to_structure(self) -> dict[str, str]:
        """Serialize lane specification into structure."""
        return {"type": self.type_.value, "direction": self.direction.value}


@dataclass
class Road:
    """OpenStreetMap way or relation described road part."""

    tags: Tags

    # DrivingSide bidirectional traffic practice in the region where the road is
    # located.
    driving_side: DrivingSide

    def add_lane(self, lanes: list[Lane], lane: Lane, side: str) -> list[Lane]:
        """Add lanes to the result list."""
        if side == "left":
            return [lane] + lanes
        else:
            return lanes + [lane]

    def add_both_lanes(self, lanes: list[Lane], type_: LaneType) -> list[Lane]:
        """Add left and right lanes."""
        return (
            [Lane(type_, self.get_direction("left"))]
            + lanes
            + [Lane(type_, self.get_direction("right"))]
        )

    def get_direction(self, side: str, is_inverted: bool = False) -> Direction:
        """
        Compute lane direction based on road side and bidirectional traffic
        practice.

        :param side: side of the road
        :param is_inverted: whether the result should be inverted
        """
        if (
            side == "right"
            and self.driving_side == DrivingSide.RIGHT
            or side == "left"
            and self.driving_side == DrivingSide.LEFT
        ):
            return Direction.BACKWARD if is_inverted else Direction.FORWARD

        return Direction.FORWARD if is_inverted else Direction.BACKWARD

    def parse(self) -> list[Lane]:
        """Process road tags."""
        """
        Parse road features described by tags and generate list of lane
        specifications from left to right.
        
        :return: list of lane specifications
        """
        sides: set[str] = {"left", "right"}
        parking_values: set[str] = {"parallel", "diagonal"}
        track_values: set[str] = {"track", "opposite_track"}

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
            lanes = self.add_both_lanes(lanes, LaneType.SIDEWALK)
        elif self.tags.get("sidewalk") == "none":
            lanes = self.add_both_lanes(lanes, LaneType.SHOULDER)
        else:
            for side in sides:
                if self.tags.get("sidewalk") == side:
                    lane = Lane(LaneType.SIDEWALK, self.get_direction(side))
                    lanes = self.add_lane(lanes, lane, side)

        return lanes