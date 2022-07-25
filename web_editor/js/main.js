// TODO The hash changes, this is very brittle. See
// https://github.com/thedodd/trunk/issues/230 or stop using trunk.
import { dummyData } from "./dummy_data.js";
import init, {
  js_way_to_lanes,
  js_lanes_to_tags,
} from "../osm2lanes-npm-34a13471983c341c.js";
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

// This state is set up by start_editing
var current_road = null;
var current_locale = null;

async function start_editing() {
  const way = BigInt(document.getElementById("osm_way_id").value);
  // Faster dev workflow: if the way ID is the default, use baked-in data instead of waiting on Overpass.
  var road_wrapper, locale;
  if (way == 804788513) {
    [road_wrapper, locale] = dummyData();
  } else {
    console.log(`Fetching ${way}...`);
    [road_wrapper, locale] = await js_way_to_lanes(way);
  }
  const road = road_wrapper["Ok"]["road"];
  console.log(`Got osm2lanes output, creating cards`);

  current_road = JSON.parse(JSON.stringify(road));
  current_locale = JSON.parse(JSON.stringify(locale));

  // TODO Fully clean up the old cards, including whatever sortable thing is attached there?
  const cards = document.getElementById("cards");
  cards.replaceChildren();

  // Create a card per lane
  for (const lane of road["lanes"]) {
    if (lane["type"] == "separator") {
      continue;
    }
    cards.appendChild(make_lane_card(lane));
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
        card = make_lane_card({
          type: "travel",
          direction: "backward",
          designated: "motor_vehicle",
        });
      } else if (type == "bicycle") {
        card = make_lane_card({
          type: "travel",
          direction: "backward",
          designated: "bicycle",
        });
      } else if (type == "parking") {
        card = make_lane_card({
          type: "parking",
          direction: "backward",
          designated: "motor_vehicle",
        });
      }
      evt.item.replaceWith(card);
    },
  });
}
window.start_editing = start_editing;

function make_lane_card(lane) {
  var node = document.createElement("div");
  node.setAttribute("class", "card");
  node.setAttribute("title", JSON.stringify(lane, null, 2));
  node.laneJSON = lane;

  node.innerHTML = lane["type"];

  // TODO I want if-let
  {
    let x = lane["designated"];
    if (x) {
      node.innerHTML += `, ${x}`;
    }
  }

  {
    let x = lane["direction"];
    if (x == "forward") {
      node.innerHTML += ", ^";
    } else if (x == "backward") {
      node.innerHTML += ", v";
    } else if (x == "both") {
      node.innerHTML += ", |";
    }
  }

  {
    let x = lane["width"];
    if (x) {
      node.innerHTML += `, width = ${x}m`;
    }
  }

  return node;
}

function generate_tags() {
  // Mutate the shared state
  current_road["lanes"] = [];
  for (const card of document.getElementById("cards").children) {
    current_road["lanes"].push(card.laneJSON);
  }
  const tags = js_lanes_to_tags(current_road, current_locale);

  const output = document.getElementById("output_tags");
  output.value = tags;
}
window.generate_tags = generate_tags;
