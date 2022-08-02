import { makeLaneCard } from "./cards.js";
import { dummyData } from "./dummy_data.js";
import init, {
  js_way_to_lanes,
  js_lanes_to_tags,
} from "./osm2lanes-npm/osm2lanes_npm.js";

await init();
setupOnce();

export class LaneEditor {
  constructor(way, road_wrapper, locale, tags) {
    const road = road_wrapper.Ok.road;

    this.way = way;
    // Clone these
    this.currentRoad = JSON.parse(JSON.stringify(road));
    this.currentLocale = JSON.parse(JSON.stringify(locale));
    this.originalTags = tags;
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
    return new LaneEditor(way, road_wrapper, locale, tags);
  }

  render() {
    // TODO Fully clean up the old cards, including whatever sortable thing is attached there?
    const cards = document.getElementById("cards");
    cards.replaceChildren();

    // Create a card per lane
    for (const lane of this.currentRoad.lanes) {
      if (lane.type == "separator") {
        continue;
      }
      cards.appendChild(makeLaneCard(lane));
    }

    new Sortable(cards, {
      group: {
        name: "lanes",
        put: ["toolbox"],
      },
      animation: 150,
      ghostClass: "card-being-dragged",
      onAdd: function (evt) {
        const type = evt.item.getAttribute("value");
        // TODO switch case but without break?
        // TODO Figure out direction based on center line position
        var card;
        if (type == "driving") {
          card = makeLaneCard({
            type: "travel",
            direction: "backward",
            designated: "motor_vehicle",
          });
        } else if (type == "bicycle") {
          card = makeLaneCard({
            type: "travel",
            direction: "backward",
            designated: "bicycle",
          });
        } else if (type == "parking") {
          card = makeLaneCard({
            type: "parking",
            direction: "backward",
            designated: "motor_vehicle",
          });
        }
        evt.item.replaceWith(card);
      },
    });
  }

  diffTags() {
    // Mutate the shared state
    this.currentRoad.lanes = [];
    for (const card of document.getElementById("cards").children) {
      this.currentRoad.lanes.push(card.laneJSON);
    }
    const currentTags = js_lanes_to_tags(this.currentRoad, this.currentLocale);

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
}

function setupOnce() {
  // Setup the toolbox controls
  new Sortable(document.getElementById("toolbox"), {
    group: {
      name: "toolbox",
      pull: "clone",
    },
    sort: false,
    animation: 150,
    ghostClass: "card-being-dragged",
  });

  // TODO Disable the ghostClass of lanes when going here
  new Sortable(document.getElementById("delete"), {
    group: "lanes",
    onAdd: function (evt) {
      var el = evt.item;
      el.parentNode.removeChild(el);
    },
  });

  document.getElementById("start-editing").onclick = async function () {
    try {
      window.app = await LaneEditor.create();
      window.app.render();
    } catch (err) {
      window.alert(`Error: ${err}`);
    }
  };
  document.getElementById("diff-tags").onclick = function () {
    if (window.app) {
      try {
        window.app.diffTags();
      } catch (err) {
        window.alert(`Error: ${err}`);
      }
    }
  };
}
