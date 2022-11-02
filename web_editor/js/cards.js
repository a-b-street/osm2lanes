export function makeLaneCard(lane, idx, app) {
  var node = document.createElement("div");
  node.className = "card";
  node.title = JSON.stringify(lane, null, 2);
  node.style = `background: ${backgroundColor(lane)};`;

  node.innerHTML = `<div align="center">${typeIcon(lane)}</div>`;
  node.innerHTML += `<div align="center">${directionIcon(lane)}</div>`;
  node.innerHTML += `<div align="center">${width(lane)}</div>`;

  var finalRow = document.createElement("div");

  var left = iconObj("left");
  if (idx != 0) {
    left.onclick = () => {
      const array = app.road.lanes;
      [array[idx - 1], array[idx]] = [array[idx], array[idx - 1]];
      app.render();
    };
  } else {
    // TODO Show greyed out
  }
  finalRow.appendChild(left);

  var trash = iconObj("delete");
  trash.onclick = () => {
    app.road.lanes.splice(idx, 1);
    app.render();
  };
  finalRow.appendChild(trash);

  var right = iconObj("right");
  if (idx != app.road.lanes.length - 1) {
    right.onclick = () => {
      const array = app.road.lanes;
      [array[idx + 1], array[idx]] = [array[idx], array[idx + 1]];
      app.render();
    };
  } else {
    // TODO Show greyed out
  }
  finalRow.appendChild(right);

  node.append(finalRow);

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
  if (lane.direction == "forward") {
    return `<img src="assets/forwards.svg" />`;
  }
  if (lane.direction == "backward") {
    return `<img src="assets/backwards.svg" />`;
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
function iconObj(name) {
  var obj = document.createElement("img");
  obj.src = `assets/${name}.svg`;
  obj.className = "clickable";
  return obj;
}
