/**
 * Lane tag parsing.
 */
import kotlinx.serialization.*
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.encodeToJsonElement
import java.io.File

/** Bidirectional traffic practice. */
@Serializable
enum class DrivingSide(val side: String) {
    @SerialName("right") RIGHT("right"),
    @SerialName("left") LEFT("left"),
}

/** Lane direction relative to OpenStreetMap way direction. */
@Serializable
enum class Direction(val direction: String) {
    @SerialName("forward") FORWARD("forward"),
    @SerialName("backward") BACKWARD("backward"),
}

/** Lane designation. */
@Serializable
enum class LaneType(val type: String) {
    @SerialName("sidewalk") SIDEWALK("sidewalk"),
    @SerialName("cycleway") CYCLEWAY("cycleway"),
    @SerialName("driveway") DRIVEWAY("driveway"),
    @SerialName("parking_lane") PARKING_LANE("parking_lane"),
    @SerialName("no_cycleway") NO_CYCLEWAY("no_cycleway"),
    @SerialName("no_sidewalk") NO_SIDEWALK("no_sidewalk"),
}

/** Lane specification. */
@Serializable
data class Lane(val type: LaneType, val direction: Direction)

/** OpenStreetMap way or relation described road part. */
class Road(private val tags: Map<String, String>) {

    fun parse(): List<Lane> {
        val lanes = arrayListOf<Lane>()

        // Driveways

        if (tags.containsKey("lanes")) {
            val laneNumber: String? = tags["lanes"]

            if (laneNumber != null) {
                val number = laneNumber.toInt()

                if (tags["oneway"] == "yes") {
                    for (i in 1..number) {
                        lanes.add(Lane(LaneType.DRIVEWAY, Direction.FORWARD))
                    }
                } else {
                    val half = number / 2

                    for (i in 1..half) {
                        lanes.add(Lane(LaneType.DRIVEWAY, Direction.BACKWARD))
                    }
                    for (i in half + 1..number) {
                        lanes.add(Lane(LaneType.DRIVEWAY, Direction.FORWARD))
                    }
                }
            }
        }

        // Cycleways

        if (tags["cycleway:left"] == "lane") {
            lanes.add(0, Lane(LaneType.CYCLEWAY, Direction.FORWARD))
        } else if (tags["cycleway:left"] == "track") {
            lanes.add(0, Lane(LaneType.CYCLEWAY, Direction.FORWARD))
            lanes.add(0, Lane(LaneType.CYCLEWAY, Direction.BACKWARD))
        }
        if (tags["cycleway:right"] == "lane") {
            lanes.add(Lane(LaneType.CYCLEWAY, Direction.BACKWARD))
        } else if (tags["cycleway:right"] == "track") {
            lanes.add(Lane(LaneType.CYCLEWAY, Direction.BACKWARD))
            lanes.add(Lane(LaneType.CYCLEWAY, Direction.FORWARD))
        }

        // Sidewalks

        if (tags["sidewalk"] == "both") {
            lanes.add(0, Lane(LaneType.SIDEWALK, Direction.BACKWARD))
            lanes.add(Lane(LaneType.SIDEWALK, Direction.FORWARD))
        } else if (tags["sidewalk"] == "none") {
            lanes.add(0, Lane(LaneType.NO_SIDEWALK, Direction.BACKWARD))
            lanes.add(Lane(LaneType.NO_SIDEWALK, Direction.FORWARD))
        }
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
    val lanes = Road(Json.decodeFromString(File(args[0]).readText(Charsets.UTF_8))).parse()
    File(args[1]).writeText(JsonArray(lanes.map{Json.encodeToJsonElement(it)}).toString())
}