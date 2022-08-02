import { dummyData } from "./dummy_data.js";
import init, {
  js_way_to_lanes,
  js_lanes_to_tags,
} from "./osm2lanes-npm/osm2lanes_npm.js";
await init();

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

export class LaneEditor {
  constructor(way, road_wrapper, locale) {
    const road = road_wrapper.Ok.road;

    this.way = way;
    // Clone these
    this.current_road = JSON.parse(JSON.stringify(road));
    this.current_locale = JSON.parse(JSON.stringify(locale));
    console.log("LaneEditor got osm2lanes output");
  }

  static async create() {
    const way = BigInt(document.getElementById("osm_way_id").value);
    // Faster dev workflow: if the way ID is the default, use baked-in data instead of waiting on Overpass.
    var road_wrapper, locale;
    if (way == 427757048) {
      [road_wrapper, locale] = dummyData();
    } else {
      console.log(`Fetching ${way}...`);
      [road_wrapper, locale] = await js_way_to_lanes(way);
    }
    return new LaneEditor(way, road_wrapper, locale);
  }

  render() {
    console.log("LaneEditor rendering");
    // TODO Fully clean up the old cards, including whatever sortable thing is attached there?
    const cards = document.getElementById("cards");
    cards.replaceChildren();

    // Create a card per lane
    for (const lane of this.current_road.lanes) {
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

  generateTags() {
    // Mutate the shared state
    this.current_road.lanes = [];
    for (const card of document.getElementById("cards").children) {
      this.current_road.lanes.push(card.laneJSON);
    }
    const tags = js_lanes_to_tags(this.current_road, this.current_locale);

    const output = document.getElementById("output_tags");
    output.value = tags;
  }
}

function makeLaneCard(lane) {
  var node = document.createElement("div");
  node.setAttribute("class", "card");
  node.setAttribute("title", JSON.stringify(lane, null, 2));
  node.laneJSON = lane;

  node.innerHTML = lane.type;

  // TODO I want if-let
  {
    let x = lane.designated;
    if (x) {
      node.innerHTML += `, ${x}`;
    }
  }

  {
    let x = lane.direction;
    if (x == "forward") {
      node.innerHTML += ", ^";
    } else if (x == "backward") {
      node.innerHTML += ", v";
    } else if (x == "both") {
      node.innerHTML += ", |";
    }
  }

  {
    let x = lane.width;
    if (x) {
      node.innerHTML += `, width = ${x}m`;
    }
  }

  return node;
}
