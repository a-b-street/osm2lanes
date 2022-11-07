import { makeLaneCard } from "./cards.js";
import { dummyData } from "./dummy_data.js";
import init, {
  js_way_to_lanes,
  js_lanes_to_tags,
} from "./osm2lanes-npm/osm2lanes_npm.js";

await init();
document.getElementById("start-editing").onclick = async function () {
  try {
    window.app = await LaneEditor.create();
    window.app.render();
  } catch (err) {
    window.alert(`Error: ${err}`);
  }
};

export class LaneEditor {
  constructor(way, road, locale, tags) {
    this.way = way;
    this.road = road;
    this.locale = locale;
    this.originalTags = tags;

    // Remove these immediately. If they're rendered invisibly, they mess up indices.
    this.road.lanes = this.road.lanes.filter(
      (lane) => lane.type != "separator"
    );

    // All of the state is immutable, except for road. The source of truth for
    // the current lanes is there; the rendered DOM view is a function of
    // it.

    this.#setupButtons();
  }

  static async create() {
    const way = BigInt(document.getElementById("osm_way_id").value);
    // Faster dev workflow: if the way ID is the default, use baked-in data instead of waiting on Overpass.
    var road_wrapper, locale, tags;
    if (way == 427757048) {
      [road_wrapper, locale, tags] = dummyData();
    } else {
      // TODO Disable the button, show status
      console.log(`Fetching ${way}...`);
      [road_wrapper, locale, tags] = await js_way_to_lanes(way);
    }
    return new LaneEditor(way, road_wrapper.Ok.road, locale, tags);
  }

  render() {
    const cards = document.getElementById("cards");
    cards.replaceChildren();

    // Create a card per lane
    var i = 0;
    for (const lane of this.road.lanes) {
      cards.appendChild(makeLaneCard(lane, i, this));
      i += 1;
    }
  }

  #diffTags() {
    const currentTags = js_lanes_to_tags(this.road, this.locale);

    var output = "<table>";
    for (const [key, origValue] of Object.entries(this.originalTags)) {
      const newValue = currentTags[key];
      var color = "";
      if (newValue == null) {
        color = "background: red";
      } else if (origValue != newValue) {
        color = "background: yellow;";
      }
      output += `<tr style="${color}"><td>${key}</td><td>${origValue}</td><td>${
        newValue || ""
      }</td></tr>`;
    }
    for (const [key, newValue] of Object.entries(currentTags)) {
      if (!this.originalTags[key]) {
        output += `<tr style="background: green"><td>${key}</td><td></td><td>${newValue}</td></tr>`;
      }
    }
    output += "</table>";

    document.getElementById("diff-tags-table").innerHTML = output;
  }

  #setupButtons() {
    document.getElementById("diff-tags").onclick = () => {
      try {
        this.#diffTags();
      } catch (err) {
        window.alert(`Error: ${err}`);
      }
    };

    document.getElementById("new-driving").onclick = () => {
      this.road.lanes.push({
        type: "travel",
        direction: "backward",
        designated: "motor_vehicle",
      });
      this.render();
    };
    document.getElementById("new-bicycle").onclick = () => {
      this.road.lanes.push({
        type: "travel",
        direction: "backward",
        designated: "bicycle",
      });
      this.render();
    };
    document.getElementById("new-parking").onclick = () => {
      this.road.lanes.push({
        type: "parking",
        direction: "backward",
        designated: "motor_vehicle",
      });
      this.render();
    };
    document.getElementById("new-bus").onclick = () => {
      this.road.lanes.push({
        type: "travel",
        direction: "backward",
        designated: "bus",
      });
      this.render();
    };
    document.getElementById("new-sidewalk").onclick = () => {
      this.road.lanes.push({
        type: "travel",
        designated: "foot",
      });
      this.render();
    };
  }
}
