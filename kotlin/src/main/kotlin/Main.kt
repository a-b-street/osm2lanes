import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.decodeFromString
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.encodeToJsonElement
import java.io.File
import kotlin.math.ceil

/** Bidirectional traffic practice. */
@Serializable
enum class DrivingSide {
    /**
     * Vehicles travel on the right side of a road.
     */
    @SerialName("right")
    RIGHT,

    /**
     * Vehicles travel on the left side of a road.
     */
    @SerialName("left")
    LEFT,
}

/**
 * Lane direction relative to OpenStreetMap way direction.
 *
 * See OpenStreetMap wiki page
 * [Forward & backward, left & right](https://wiki.openstreetmap.org/wiki/Forward_%26_backward,_left_%26_right).
 */
@Serializable
enum class Direction {
    /**
     * The direction of the lane coincides with the direction of the way.
     */
    @SerialName("forward")
    FORWARD,

    /**
     * The direction of the lane it opposite to the direction of the way.
     */
    @SerialName("backward")
    BACKWARD,
}

/** Lane designation. */
@Serializable
enum class LaneType {
    /**
     * Part of a highway set aside for the use of pedestrians and sometimes also cyclists, separated from the
     * carriageway (or roadway).  See OpenStreetMap wiki page
     * [Sidewalks](https://wiki.openstreetmap.org/wiki/Sidewalks).
     */
    @SerialName("sidewalk")
    SIDEWALK,

    /**
     * Cycling infrastructure that is an inherent part of the road.  See OpenStreetMap wiki page
     * [Key:cycleway](https://wiki.openstreetmap.org/wiki/Key:cycleway).
     */
    @SerialName("cycleway")
    CYCLEWAY,

    /**
     * Traffic lane of a highway suitable for vehicles.
     */
    @SerialName("travel_lane")
    TRAVEL_LANE,

    /**
     * Part of the road designated for parking.
     */
    @SerialName("parking_lane")
    PARKING_LANE,

    /**
     * A shared center turn lane.
     */
    @SerialName("shared_left_turn")
    SHARED_LEFT_TURN,

    /**
     * Some roads without any sidewalks still have pedestrian traffic. This type represents the shoulder of the road,
     * where people are usually forced to walk.
     */
    @SerialName("shoulder")
    SHOULDER,

    /**
     * A bus-only lane.
     */
    @SerialName("bus_lane")
    BUS_LANE,
}

/**
 * The amount of space between the lanes.
 */
@Serializable
enum class BufferType {
    /**
     * Painted stripes
     */
    @SerialName("stripes")
    STRIPES,

    /**
     * Flex posts, wands, cones, car ticklers, bollards, other "weak" forms of protection. Usually possible to weave
     * through them.
     */
    @SerialName("flex_posts")
    FLEX_POSTS,

    /**
     * Sturdier planters, with gaps
     */
    @SerialName("planters")
    PLANTERS,

    /**
     * Solid barrier, no gaps.
     */
    @SerialName("jersey_barrier")
    JERSEY_BARRIER,

    /**
     * A raised curb
     */
    @SerialName("curb")
    CURB,
}

/** Lane specification. */
@Serializable
data class Lane(val type: LaneType, val direction: Direction) {
    override fun toString(): String {
        return "${type}_$direction"
    }
}

enum class BusUsage {
    YES,
    NO,
    DESIGNATED,
    UNKNOWN,
}

data class LaneUsage(
    var is_vehicle: Boolean = false,
    var is_psv: BusUsage = BusUsage.UNKNOWN,
    var is_left_turn: Boolean = false,
    var is_through_turn: Boolean = false,
    var is_right_turn: Boolean = false,
)

/**
 * OpenStreetMap way or relation described road part.
 *
 * @param tags associative array describing road features, see OpenStreetMap wiki page
 *     [Tags](https://wiki.openstreetmap.org/wiki/Tags)
 * @param drivingSide bidirectional traffic practice in the region where the road is located
 */
class Road(private val tags: Map<String, String>, private val drivingSide: DrivingSide) {

    /**
     * Compute lane direction based on road side and bidirectional traffic practice.
     *
     * @param side side of the road
     * @param isInverted whether the result should be inverted
     */
    private fun getDirection(side: String, isInverted: Boolean = false): Direction {
        if (side == "right" && drivingSide == DrivingSide.RIGHT || side == "left" && drivingSide == DrivingSide.LEFT)
            return if (isInverted) Direction.BACKWARD else Direction.FORWARD

        return if (isInverted) Direction.FORWARD else Direction.BACKWARD
    }

    /**
     * Add new lane.
     *
     * @param lanes destination lanes listed from left to right
     * @param lane new lane to add
     * @param side new lane position: left or right
     */
    private fun addLane(lanes: ArrayList<Lane>, lane: Lane, side: String) {
        if (side == "left") lanes.add(0, lane) else lanes.add(lane)
    }

    /**
     * Add left and right lanes.
     *
     * @param lanes destination lanes listed from left to right
     * @param type lane type
     */
    private fun addBothLanes(lanes: ArrayList<Lane>, type: LaneType) {
        lanes.add(0, Lane(type, getDirection("left")))
        lanes.add(Lane(type, getDirection("right")))
    }

    /**
     * Compute the number of special lanes (e.g. bus-only lanes).
     */
    private fun getExtraLanes(): Int {
        var laneCount = 0

        if (tags["busway"] == "lane" || tags["busway:both"] == "lane")
            laneCount += 2
        if (tags["busway:right"] == "lane" || tags["busway:left"] == "lane")
            laneCount += 1

        return laneCount
    }

    private fun parseVehicleLanes(representation: String, laneUsage: List<LaneUsage>) {
        representation.split("|").forEachIndexed { i, description ->
            if (description == "yes")
                laneUsage[i].is_vehicle = true
            else if (description == "no")
                laneUsage[i].is_vehicle = false
        }
    }

    private fun parseBusLanes(representation: String, laneUsage: List<LaneUsage>) {
        representation.split("|").forEachIndexed { i, description ->
            when (description) {
                "designated" -> laneUsage[i].is_psv = BusUsage.DESIGNATED
                "no" -> laneUsage[i].is_psv = BusUsage.NO
                "yes" -> laneUsage[i].is_psv = BusUsage.YES
                else -> laneUsage[i].is_psv = BusUsage.UNKNOWN
            }
        }
    }

    /**
     * Parse road features described by tags and generate list of lane specifications from left to right.
     *
     * @return list of lane specifications
     */
    fun parse(): List<Lane> {

        val sides = setOf("left", "right")
        val parkingValues = setOf("parallel", "diagonal")
        val trackValues = setOf("track", "opposite_track")

        val lanes = arrayListOf<Lane>()

        // Driveways

        val oneway = tags["oneway"] == "yes"

        var travelLaneNumber: Int
        val totalLaneNumber = tags["lanes"]

        // If lane number is not specified, we assume that there are two lanes: one forward and one backward (if it is
        // not a oneway road).
        travelLaneNumber = if (totalLaneNumber != null) totalLaneNumber.toInt() - getExtraLanes() else 2

        if (travelLaneNumber == 1 && !oneway)
            travelLaneNumber = 2

        if (oneway)
            (1..travelLaneNumber).forEach { _ -> lanes.add(Lane(LaneType.TRAVEL_LANE, Direction.FORWARD)) }
        else {
            val half =
                if (drivingSide == DrivingSide.RIGHT) travelLaneNumber / 2 else ceil(travelLaneNumber / 2.0).toInt()

            (1..half).forEach { _ -> lanes.add(Lane(LaneType.TRAVEL_LANE, getDirection("left"))) }

            if (tags["centre_turn_lane"] == "yes")
                lanes.add(Lane(LaneType.SHARED_LEFT_TURN, Direction.FORWARD))

            (half + 1..travelLaneNumber).forEach { _ ->
                lanes.add(Lane(LaneType.TRAVEL_LANE, getDirection("right")))
            }
        }

        // Cycleways

        for (side in sides)
            if (tags["cycleway:$side"] == "lane")
            // If road is oneway, cycleways should follow the travel lane direction.
                addLane(lanes, Lane(LaneType.CYCLEWAY, if (oneway) Direction.FORWARD else getDirection(side)), side)
            else if (trackValues.contains(tags["cycleway:$side"])) {
                addLane(lanes, Lane(LaneType.CYCLEWAY, getDirection(side, true)), side)
                addLane(lanes, Lane(LaneType.CYCLEWAY, getDirection(side)), side)
            }

        // Bus lanes

        if (tags["busway"] == "lane" || tags["busway:both"] == "lane")
            addBothLanes(lanes, LaneType.BUS_LANE)
        for (side in sides)
            if (tags["busway:$side"] == "lane")
                addLane(lanes, Lane(LaneType.BUS_LANE, getDirection(side)), side)

        // Parking lanes

        if (tags["parking:lane:both"] == "parallel")
            addBothLanes(lanes, LaneType.PARKING_LANE)

        for (side in sides)
            if (parkingValues.contains(tags["parking:lane:$side"]))
                addLane(lanes, Lane(LaneType.PARKING_LANE, getDirection(side)), side)

        // Sidewalks

        if (tags["sidewalk"] == "both")
            addBothLanes(lanes, LaneType.SIDEWALK)
        else if (tags["sidewalk"] == "none")
            addBothLanes(lanes, LaneType.SHOULDER)
        else
            for (side in sides)
                if (tags["sidewalk"] == side)
                    addLane(lanes, Lane(LaneType.SIDEWALK, getDirection(side)), side)

        return lanes
    }
}

/**
 * Command-line interface.
 *
 * Read OpenStreetMap tags from input JSON file and write lane specifications into output JSON file.
 *
 * @param args command-line arguments: input JSON file path, output JSON file path
 */
fun main(args: Array<String>) {
    val lanes = Road(Json.decodeFromString(File(args[0]).readText(Charsets.UTF_8)), DrivingSide.RIGHT).parse()
    File(args[1]).writeText(JsonArray(lanes.map { Json.encodeToJsonElement(it) }).toString())
}
