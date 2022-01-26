/**
 * Test lane tag parsing.
 */
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.decodeFromString
import kotlinx.serialization.json.Json
import java.io.File
import kotlin.test.Test
import kotlin.test.assertEquals

const val TEST_FILE_PATH = "../data/tests.json"

@Serializable
internal data class TestCase(
    @SerialName("skip_kotlin")
    val skip: Boolean = false,
    val description: String = "",
    /**
     * The OSM way unique identifier.
     */
    val way_id: Int? = null,
    val tags: HashMap<String, String>,
    val driving_side: DrivingSide,
    val output: ArrayList<Lane>,
)

/** Lane generation tests. */
internal class LaneTest {
    private val json = Json { ignoreUnknownKeys = true }

    @Test
    fun testLanes() {
        val testSuite = json.decodeFromString<ArrayList<TestCase>>(File(TEST_FILE_PATH).readText(Charsets.UTF_8))

        for (testCase in testSuite) {
            if (!testCase.skip) {
                val parsed = Road(testCase.tags, testCase.driving_side).parse()
                assertEquals(
                    testCase.output.filter { it.type != LaneType.SEPARATOR },
                    parsed,
                    "${testCase.way_id ?: testCase.description}\nGot:      ${parsed}\nExpected: ${testCase.output}\n"
                )
            }
        }
    }
}
