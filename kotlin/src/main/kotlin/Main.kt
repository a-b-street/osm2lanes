import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.decodeFromString
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.encodeToJsonElement
import java.io.File
import kotlin.collections.ArrayList
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

    @SerialName("forward")
    FORWARD,

    @SerialName("backward")
    BACKWARD,

    @SerialName("both")
    BOTH,
}

/** Lane type. */
@Serializable
enum class LaneType {

    @SerialName("travel")
    TRAVEL,

    @SerialName("parking")
    PARKING,

    @SerialName("shoulder")
    SHOULDER,

    @SerialName("separator")
    SEPARATOR,

    @SerialName("construction")
    CONSTRUCTION,
}

/** Lane designated. */
@Serializable
enum class LaneDesignated {

    @SerialName("any")
    ANY,

    @SerialName("foot")
    FOOT,

    @SerialName("bicycle")
    BICYCLE,

    @SerialName("motor_vehicle")
    MOTOR,

    @SerialName("bus")
    BUS,

    @SerialName("psv")
    PSV,
}

/** Lane specification. */
@Serializable
data class Lane(val type: LaneType, val direction: Direction? = null, val designated: LaneDesignated? = null) {
    override fun toString(): String {
        return "${type}_${direction}_$designated"
    }
}

/**
 * Lane usage by buses according to `lanes:pvs=*` scheme.  See OpenStreetMap wiki pages
 * [Key:lanes:psv](https://wiki.openstreetmap.org/wiki/Key:lanes:psv) and
 * [Bus Lanes](https://wiki.openstreetmap.org/wiki/Bus_lanes)
 */
enum class BusUsage {

    /**
     * Buses and other vehicles may use this lane.  Tag value `yes`.
     */
    YES,

    /**
     * Buses cannot use this lane.  Tag value `no`.
     */
    NO,

    /**
     * The lane is bus-only, explicitly designated.  Tag value `designated`.
     */
    DESIGNATED,

    /**
     * Value is not specified.
     */
    UNKNOWN,

    /**
     * Specified value is unknown.  Any tag value other than `yes`, `no`, or `designated`.
     */
    WRONG_VALUE,
}

enum class TurnUsage {
    LEFT,
    MERGE_TO_LEFT,
    MERGE_TO_RIGHT,
    REVERSE,
    RIGHT,
    SHARP_LEFT,
    SHARP_RIGHT,
    SLIGHT_LEFT,
    SLIGHT_RIGHT,
    THROUGH,
}

data class LaneUsage(
    var is_vehicle: Boolean = false,
    var is_psv: BusUsage = BusUsage.UNKNOWN,
    var turns: ArrayList<TurnUsage> = arrayListOf(),
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
    private fun addBothLanes(lanes: ArrayList<Lane>, type: LaneType, designated: LaneDesignated?) {
        if (type == LaneType.SHOULDER || (type == LaneType.TRAVEL && designated == LaneDesignated.FOOT)) {
            lanes.add(0, Lane(type, null, designated))
            lanes.add(Lane(type, null, designated))
        } else {
            lanes.add(0, Lane(type, getDirection("left"), designated))
            lanes.add(Lane(type, getDirection("right"), designated))
        }
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

    /**
     * E.g. `no|no|designated`.
     *
     * @param representation lane usage text representations separated with `|` sign
     * @param laneUsage lane usage to update
     */
    private fun parseBusLanes(representation: String, laneUsage: List<LaneUsage>) {
        representation.split("|").forEachIndexed { i, description ->
            try {
                laneUsage[i].is_psv = BusUsage.valueOf(description)
            } catch (e: IllegalArgumentException) {
                laneUsage[i].is_psv = BusUsage.WRONG_VALUE
            }
        }
    }

    private fun parseTurnLanes(representation: String, laneUsage: List<LaneUsage>) {
        representation.split("|").forEachIndexed { i, laneDescription ->
            laneDescription.split(";").forEach { description ->
                laneUsage[i].turns.add(TurnUsage.valueOf(description.uppercase()))
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
            (1..travelLaneNumber).forEach { _ -> lanes.add(Lane(LaneType.TRAVEL, Direction.FORWARD, LaneDesignated.MOTOR)) }
        else {
            val half =
                if (drivingSide == DrivingSide.RIGHT) travelLaneNumber / 2 else ceil(travelLaneNumber / 2.0).toInt()

            (1..half).forEach { _ -> lanes.add(Lane(LaneType.TRAVEL, getDirection("left"), LaneDesignated.MOTOR)) }

            if (tags["centre_turn_lane"] == "yes")
                lanes.add(Lane(LaneType.TRAVEL, Direction.BOTH, LaneDesignated.MOTOR))

            (half + 1..travelLaneNumber).forEach { _ ->
                lanes.add(Lane(LaneType.TRAVEL, getDirection("right"), LaneDesignated.MOTOR))
            }
        }

        // Cycleways

        for (side in sides)
            if (tags["cycleway:$side"] == "lane")
            // If road is oneway, cycleways should follow the travel lane direction.
                addLane(lanes, Lane(LaneType.TRAVEL, if (oneway) Direction.FORWARD else getDirection(side), LaneDesignated.BICYCLE), side)
            else if (trackValues.contains(tags["cycleway:$side"])) {
                addLane(lanes, Lane(LaneType.TRAVEL, Direction.BOTH, LaneDesignated.BICYCLE), side)
            }

        // Bus lanes

        if (tags["busway"] == "lane" || tags["busway:both"] == "lane")
            addBothLanes(lanes, LaneType.TRAVEL, LaneDesignated.BUS)
        for (side in sides)
            if (tags["busway:$side"] == "lane")
                addLane(lanes, Lane(LaneType.TRAVEL, getDirection(side), LaneDesignated.BUS), side)

        // Parking lanes

        if (tags["parking:lane:both"] == "parallel")
            addBothLanes(lanes, LaneType.PARKING, LaneDesignated.MOTOR)

        for (side in sides)
            if (parkingValues.contains(tags["parking:lane:$side"]))
                addLane(lanes, Lane(LaneType.PARKING, getDirection(side), LaneDesignated.MOTOR), side)

        // Sidewalks

        if (tags["sidewalk"] == "both")
            addBothLanes(lanes, LaneType.TRAVEL, LaneDesignated.FOOT)
        else if (tags["sidewalk"] == "none")
            addBothLanes(lanes, LaneType.SHOULDER, null)
        else
            for (side in sides)
                if (tags["sidewalk"] == side)
                    addLane(lanes, Lane(LaneType.TRAVEL, null, LaneDesignated.FOOT), side)

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
