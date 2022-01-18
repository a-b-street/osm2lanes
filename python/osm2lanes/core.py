"""
Lane tag parsing.
"""
import math
from dataclasses import dataclass
from enum import Enum
from typing import Optional

Tags = dict[str, str]


class DrivingSide(Enum):
    """Bidirectional traffic practice."""

    # Vehicles travel on the right side of a road.
    RIGHT = "right"

    # Vehicles travel on the left side of a road.
    LEFT = "left"


class Direction(Enum):
    """
    Lane direction relative to OpenStreetMap way direction.

    See OpenStreetMap wiki page
    https://wiki.openstreetmap.org/wiki/Forward_%26_backward,_left_%26_right.
    """

    FORWARD = "forward"
    BACKWARD = "backward"
    BOTH = "both"
    NONE = "none"

    def __str__(self):
        if self == Direction.FORWARD:
            return "↑"
        if self == Direction.BACKWARD:
            return "↓"
        if self == Direction.BOTH:
            return "↕"
        if self == Direction.NONE:
            return "—"

    def __repr__(self):
        return str(self)


class LaneType(Enum):
    """Lane designation."""

    TRAVEL = "travel"
    PARKING = "parking"
    SHOULDER = "shoulder"
    SEPARATOR = "separator"
    CONSTRUCTION = "construction"

    def __str__(self):
        return self.value


class LaneDesignation(Enum):
    """Lane designation."""

    ANY = "any"
    FOOT = "foot"
    BICYCLE = "bicycle"
    MOTOR = "motor_vehicle"
    BUS = "bus"
    TAXI = "taxi"

    def __str__(self):
        return self.value


class BufferType(Enum):
    """The amount of space between the lanes."""

    # Painted stripes
    STRIPES = "stripes"

    # Flex posts, wands, cones, car ticklers, bollards, other "weak" forms of
    # protection. Usually possible to weave through them.
    FLEX_POSTS = "flex_posts"

    # Sturdier planters, with gaps
    PLANTERS = "planters"

    # Solid barrier, no gaps.
    JERSEY_BARRIER = "jersey_barrier"

    # A raised curb
    CURB = "curb"


@dataclass
class Lane:
    """Lane specification."""

    type_: LaneType
    direction: Optional[Direction]
    designation: Optional[LaneDesignation]

    @classmethod
    def from_structure(cls, structure: dict[str, str]) -> "Lane":
        """Parse lane specification from structure."""
        return cls(
            LaneType(structure["type"]),
            Direction(structure["direction"])
            if structure.get("direction")
            else None,
            LaneDesignation(structure["designated"])
            if structure.get("designated")
            else None,
        )

    def to_structure(self) -> dict[str, str]:
        """Serialize lane specification into structure."""
        ret = {"type": self.type_.value}
        if self.direction:
            ret["direction"] = self.direction.value
        if self.designation:
            ret["designated"] = self.designation.value
        return ret

    def __repr__(self):
        return f"{self.type_} {self.direction} {self.designation}"


@dataclass
class Road:
    """OpenStreetMap way or relation described road part."""

    # Tags associative array describing road features, see
    # https://wiki.openstreetmap.org/wiki/Tags
    tags: Tags

    # DrivingSide bidirectional traffic practice in the region where the road is
    # located.
    driving_side: DrivingSide

    @staticmethod
    def add_lane(lanes: list[Lane], lane: Lane, side: str) -> list[Lane]:
        """Add lanes to the result list."""
        if side == "left":
            return [lane] + lanes

        return lanes + [lane]

    def add_both_lanes(
        self,
        lanes: list[Lane],
        type_: LaneType,
        designation: Optional[LaneDesignation],
    ) -> list[Lane]:
        """Add left and right lanes."""
        if type_ == LaneType.SHOULDER or (
            type_ == LaneType.TRAVEL and designation == LaneDesignation.FOOT
        ):
            return (
                [Lane(type_, None, designation)]
                + lanes
                + [Lane(type_, None, designation)]
            )
        return (
            [Lane(type_, self.get_direction("left"), designation)]
            + lanes
            + [Lane(type_, self.get_direction("right"), designation)]
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

    def get_extra_lanes(self) -> int:
        """
        Compute the number of special lanes (e.g. bus-only lanes).
        """
        lane_count: int = 0

        if (
            self.tags.get("busway") == "lane"
            or self.tags.get("busway:both") == "lane"
        ):
            lane_count += 2

        if (
            self.tags.get("busway:right") == "lane"
            or self.tags.get("busway:left") == "lane"
        ):
            lane_count += 1

        return lane_count

    def parse(self) -> list[Lane]:
        """
        Parse road features described by tags and generate list of lane
        specifications from left to right.

        :return: list of lane specifications
        """
        sides: set[str] = {"left", "right"}
        parking_values: set[str] = {"parallel", "diagonal"}
        track_values: set[str] = {"track", "opposite_track"}

        lanes: list[Lane]

        # Driveways

        oneway: bool = self.tags.get("oneway") == "yes"

        travel_lane_number: int
        total_lane_number = self.tags.get("lanes")

        # If lane number is not specified, we assume that there are two
        # lanes: one forward and one backward (if it is not a oneway road).
        travel_lane_number = (
            int(total_lane_number) - self.get_extra_lanes()
            if total_lane_number
            else 2
        )

        if travel_lane_number == 1 and not oneway:
            travel_lane_number = 2

        if oneway:
            lanes = [
                Lane(LaneType.TRAVEL, Direction.FORWARD, LaneDesignation.MOTOR)
            ] * travel_lane_number
        else:
            half: int = (
                int(travel_lane_number / 2.0)
                if self.driving_side == DrivingSide.RIGHT
                else math.ceil(travel_lane_number / 2.0)
            )
            lanes = [
                Lane(
                    LaneType.TRAVEL,
                    self.get_direction("left"),
                    LaneDesignation.MOTOR,
                )
            ] * half
            if self.tags.get("centre_turn_lane") == "yes":
                lanes += [
                    Lane(LaneType.TRAVEL, Direction.BOTH, LaneDesignation.MOTOR)
                ]
            lanes += [
                Lane(
                    LaneType.TRAVEL,
                    self.get_direction("right"),
                    LaneDesignation.MOTOR,
                )
            ] * (travel_lane_number - half)

        # Cycleways

        lane: Lane

        for side in sides:
            if self.tags.get(f"cycleway:{side}") == "lane":
                lane = Lane(
                    LaneType.TRAVEL,
                    Direction.FORWARD if oneway else self.get_direction(side),
                    LaneDesignation.BICYCLE,
                )
                lanes = self.add_lane(lanes, lane, side)
            elif self.tags.get(f"cycleway:{side}") in track_values:
                lane = Lane(
                    LaneType.TRAVEL, Direction.BOTH, LaneDesignation.BICYCLE
                )
                lanes = self.add_lane(lanes, lane, side)

        # Bus lanes

        if (
            self.tags.get("busway") == "lane"
            or self.tags.get("busway:both") == "lane"
        ):
            lanes = self.add_both_lanes(
                lanes, LaneType.TRAVEL, LaneDesignation.BUS
            )
        for side in sides:
            if self.tags.get(f"busway:{side}") == "lane":
                lane = Lane(
                    LaneType.TRAVEL,
                    self.get_direction(side),
                    LaneDesignation.BUS,
                )
                lanes = self.add_lane(lanes, lane, side)

        # Parking lanes

        if self.tags.get("parking:lane:both") == "parallel":
            lanes = self.add_both_lanes(
                lanes, LaneType.PARKING, LaneDesignation.MOTOR
            )

        for side in sides:
            if self.tags.get(f"parking:lane:{side}") in parking_values:
                lane = Lane(
                    LaneType.PARKING,
                    self.get_direction(side),
                    LaneDesignation.MOTOR,
                )
                lanes = self.add_lane(lanes, lane, side)

        # Sidewalks

        is_rural = (
            self.tags.get("sidewalk") is None
            and (
                self.tags.get("lanes") is None or self.tags.get("lanes") == "2"
            )
            and self.tags.get("oneway") is None
        )
        if self.tags.get("sidewalk") == "both":
            lanes = self.add_both_lanes(
                lanes, LaneType.TRAVEL, LaneDesignation.FOOT
            )
        elif self.tags.get("sidewalk") == "none" or is_rural:
            lanes = self.add_both_lanes(lanes, LaneType.SHOULDER, None)
        else:
            for side in sides:
                if self.tags.get("sidewalk") == side:
                    lane = Lane(LaneType.TRAVEL, None, LaneDesignation.FOOT)
                    lanes = self.add_lane(lanes, lane, side)

        return lanes
