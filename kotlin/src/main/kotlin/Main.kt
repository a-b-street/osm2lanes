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
    C("C"),
    S("S"),
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
            if (tags["oneway"] == "yes") {
                val laneNumber: String? = tags["lanes"]

                if (laneNumber != null) {
                    mainLanes = arrayListOf()

                    for (i in 1..laneNumber.toInt()) {
                        mainLanes.add(Lane(LaneType.DRIVEWAY, Direction.FORWARD))
                    }
                }
            }
        }
        if (tags["sidewalk"] == "both") {
            sidewalkLeft = arrayListOf(Lane(LaneType.SIDEWALK, Direction.BACKWARD))
            sidewalkRight = arrayListOf(Lane(LaneType.SIDEWALK, Direction.FORWARD))
        }
        if (tags["cycleway:left"] == "lane") {
            cyclewayLeft = arrayListOf(Lane(LaneType.CYCLEWAY, Direction.FORWARD))
        }
        return sidewalkLeft + cyclewayLeft + mainLanes + cyclewayRight + sidewalkRight
    }
}
