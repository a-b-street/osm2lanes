/**
 * Test lane tag parsing.
 */
import kotlinx.serialization.Serializable
import kotlinx.serialization.decodeFromString
import kotlinx.serialization.json.Json
import java.io.File

const val TEST_FILE_PATH = "../data/tests.json"

@Serializable
data class TestCase(
    val skip: Boolean,
    val comment: String = "",
    val way: String,
    val tags: HashMap<String, String>,
    val driving_side: DrivingSide,
    val output: ArrayList<Lane>,
)

fun main() {
    val testSuite = Json.decodeFromString<ArrayList<TestCase>>(File(TEST_FILE_PATH).readText(Charsets.UTF_8))

    for (testCase in testSuite) {
        if (testCase.skip) {
            continue
        }
        val parsed = Road(testCase.tags).parse()
        if (parsed != testCase.output) {
            println(parsed)
            println(testCase.output)
        } else {
            println("OK")
        }
    }
}
