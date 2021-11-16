"""
Lane tag parsing.
"""
import math
from dataclasses import dataclass
from enum import Enum

Tags = dict[str, str]


class DrivingSide(Enum):
    """Bidirectional traffic practice."""

    RIGHT = "right"
    LEFT = "left"


class Direction(Enum):
    """
    Lane direction relative to OpenStreetMap way direction.

    See OpenStreetMap wiki page
    https://wiki.openstreetmap.org/wiki/Forward_%26_backward,_left_%26_right.
    """

    FORWARD = "forward"
    BACKWARD = "backward"


class LaneType(Enum):
    """Lane designation."""

    # Part of a highway set aside for the use of pedestrians and sometimes also
    # cyclists, separated from the carriageway (or roadway).  See
    # https://wiki.openstreetmap.org/wiki/Sidewalks
    SIDEWALK = "sidewalk"

    # Cycling infrastructure that is an inherent part of the road.  See
    # https://wiki.openstreetmap.org/wiki/Key:cycleway
    CYCLEWAY = "cycleway"

    # Traffic lane of a highway suitable for vehicles.
    DRIVEWAY = "driveway"

    # Part of the road designated for parking.
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

    def __str__(self):
        return f"{self.type_}_{self.direction}"


@dataclass
class Road:
    """OpenStreetMap way or relation described road part."""

    # Tags associative array describing road features, see
    # https://wiki.openstreetmap.org/wiki/Tags
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

        # If lane number is not specified, we assume that there are two lanes:
        # one forward and one backward (if it is not a oneway road).
        number: int = int(self.tags["lanes"]) if "lanes" in self.tags else 2

        oneway: bool = self.tags.get("oneway") == "yes"

        if oneway:
            lanes = [Lane(LaneType.DRIVEWAY, Direction.FORWARD)] * number
        else:
            half: int = (
                int(number / 2.0)
                if self.driving_side == DrivingSide.RIGHT
                else math.ceil(number / 2.0)
            )
            lanes = [Lane(LaneType.DRIVEWAY, self.get_direction("left"))] * half
            if self.tags.get("centre_turn_lane") == "yes":
                lanes += [Lane(LaneType.SHARED_LEFT_TURN, Direction.FORWARD)]
            lanes += [Lane(LaneType.DRIVEWAY, self.get_direction("right"))] * (
                number - half
            )

        # Cycleways

        lane: Lane

        for side in sides:
            if self.tags.get(f"cycleway:{side}") == "lane":
                lane = Lane(
                    LaneType.CYCLEWAY,
                    Direction.FORWARD if oneway else self.get_direction(side),
                )
                lanes = self.add_lane(lanes, lane, side)
            elif self.tags.get(f"cycleway:{side}") in track_values:
                lane = Lane(LaneType.CYCLEWAY, self.get_direction(side, True))
                lanes = self.add_lane(lanes, lane, side)

                lane = Lane(LaneType.CYCLEWAY, self.get_direction(side))
                lanes = self.add_lane(lanes, lane, side)

        # Parking lanes

        if self.tags.get("parking:lane:both") == "parallel":
            lanes = self.add_both_lanes(lanes, LaneType.PARKING_LANE)

        for side in sides:
            if self.tags.get(f"parking:lane:{side}") in parking_values:
                lane = Lane(LaneType.PARKING_LANE, self.get_direction(side))
                lanes = self.add_lane(lanes, lane, side)

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
