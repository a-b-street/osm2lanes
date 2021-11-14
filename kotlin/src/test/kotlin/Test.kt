import kotlinx.serialization.Serializable
import kotlinx.serialization.decodeFromString
import kotlinx.serialization.json.Json
import java.io.File

const val TEST_FILE_PATH = "../data/tests.json"

@Serializable
data class TestCase(
    val comment: String = "",
    val way: String,
    val tags: HashMap<String, String>,
    val driving_side: DrivingSide,
    val output: ArrayList<Lane>,
)

fun main(args: Array<String>) {
    val jsonString: String = File(TEST_FILE_PATH).readText(Charsets.UTF_8)
    val testSuite = Json.decodeFromString<ArrayList<TestCase>>(jsonString)

    for (testCase in testSuite) {
        val road = Road(testCase.tags)
        println(road.parse() == testCase.output)
    }
}
