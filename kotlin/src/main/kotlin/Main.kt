/**
 * Lane tag parsing.
 */
import kotlinx.serialization.*

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

class Road(val tags: HashMap<String, String>) {
    fun parse(): List<Lane> {
        var mainLanes = arrayListOf<Lane>()
        var sidewalkRight = arrayListOf<Lane>()
        var sidewalkLeft = arrayListOf<Lane>()
        var cyclewayRight = arrayListOf<Lane>()
        var cyclewayLeft = arrayListOf<Lane>()

        if (tags.containsKey("lanes")) {
            val laneNumber: String? = tags["lanes"]

            if (laneNumber != null) {
                val number = laneNumber.toInt()

                mainLanes = arrayListOf()

                if (tags["oneway"] == "yes") {
                    for (i in 1..number) {
                        mainLanes.add(Lane(LaneType.DRIVEWAY, Direction.FORWARD))
                    }
                } else {
                    val half = number / 2

                    for (i in 1..half) {
                        mainLanes.add(Lane(LaneType.DRIVEWAY, Direction.BACKWARD))
                    }
                    for (i in half + 1..number) {
                        mainLanes.add(Lane(LaneType.DRIVEWAY, Direction.FORWARD))
                    }
                }
            }
        }
        if (tags["sidewalk"] == "both") {
            sidewalkLeft = arrayListOf(Lane(LaneType.SIDEWALK, Direction.BACKWARD))
            sidewalkRight = arrayListOf(Lane(LaneType.SIDEWALK, Direction.FORWARD))
        } else if (tags["sidewalk"] == "none") {
            sidewalkLeft = arrayListOf(Lane(LaneType.NO_SIDEWALK, Direction.BACKWARD))
            sidewalkRight = arrayListOf(Lane(LaneType.NO_SIDEWALK, Direction.FORWARD))
        }
        if (tags["cycleway:left"] == "lane") {
            cyclewayLeft = arrayListOf(Lane(LaneType.CYCLEWAY, Direction.FORWARD))
        } else if (tags["cycleway:left"] == "track") {
            cyclewayLeft = arrayListOf(Lane(LaneType.CYCLEWAY, Direction.BACKWARD),
                Lane(LaneType.CYCLEWAY, Direction.FORWARD))
        }
        if (tags["cycleway:right"] == "lane") {
            cyclewayRight = arrayListOf(Lane(LaneType.CYCLEWAY, Direction.BACKWARD))
        } else if (tags["cycleway:right"] == "track") {
            cyclewayRight = arrayListOf(Lane(LaneType.CYCLEWAY, Direction.BACKWARD),
                Lane(LaneType.CYCLEWAY, Direction.FORWARD))
        }
        return sidewalkLeft + cyclewayLeft + mainLanes + cyclewayRight + sidewalkRight
    }
}
