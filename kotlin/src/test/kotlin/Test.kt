/**
 * Test lane tag parsing.
 */
import kotlinx.serialization.Serializable
import kotlinx.serialization.decodeFromString
import kotlinx.serialization.json.Json
import java.io.File
import kotlin.test.Test
import kotlin.test.assertEquals

const val TEST_FILE_PATH = "../data/tests.json"

@Serializable
internal data class TestCase(
    val skip: Boolean = false,
    val comment: String = "",
    val way_id: Int,
    val tags: HashMap<String, String>,
    val driving_side: DrivingSide,
    val output: ArrayList<Lane>,
)

/** Lane generation tests. */
internal class LaneTest {
    @Test
    fun testLanes() {
        val testSuite = Json.decodeFromString<ArrayList<TestCase>>(File(TEST_FILE_PATH).readText(Charsets.UTF_8))

        for (testCase in testSuite) {
            if (!testCase.skip) {
                val parsed = Road(testCase.tags, testCase.driving_side).parse()
                assertEquals(testCase.output, parsed, testCase.driving_side.toString())
            }
        }
    }
}
