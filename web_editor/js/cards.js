export function makeLaneCard(lane) {
  var node = document.createElement("div");
  node.setAttribute("class", "card");
  node.setAttribute("title", JSON.stringify(lane, null, 2));
  node.setAttribute("style", `background: ${backgroundColor(lane)};`);
  node.laneJSON = lane;

  node.innerHTML = `<div align="center">` + typeIcon(lane) + `</div>`;
  node.innerHTML += `<div align="center">` + directionIcon(lane) + `</div>`;
  node.innerHTML += `<div align="center">` + width(lane) + `</div>`;

  return node;
}

function typeIcon(lane) {
  if (lane.type == "travel" && lane.designated == "bicycle") {
    return icon("bicycle");
  }
  if (lane.type == "travel" && lane.designated == "bus") {
    return icon("bus");
  }
  if (lane.type == "travel" && lane.designated == "motor_vehicle") {
    return icon("car");
  }
  if (lane.type == "parking" && lane.designated == "motor_vehicle") {
    return icon("parking");
  }

  return `<b>${lane.type}, ${lane.designated}</b>`;
}

function directionIcon(lane) {
  // TODO Attach an onclick handler. Probably create the below using the DOM instead of innerHTML.
  if (lane.direction == "forward") {
    return `<img src="assets/forwards.svg" class="clickable-icon" />`;
  }
  if (lane.direction == "backward") {
    return `<img src="assets/backwards.svg" class="clickable-icon" />`;
  }
  if (lane.direction == "both") {
    return icon("both_ways");
  }
  // Just an empty space
  return "";
}

function backgroundColor(lane) {
  if (lane.type == "travel" && lane.designated == "bicycle") {
    return "#0F7D4B";
  }
  return "grey";
}

function width(lane) {
  if (lane.width) {
    return `${lane.width}m`;
  }
  return "";
}

function icon(name) {
  return `<img src="assets/${name}.svg" class="icon" />`;
}
