export function makeLaneCard(lane, idx, app) {
  var node = document.createElement("div");
  node.className = "card";
  node.title = JSON.stringify(lane, null, 2);
  node.style = `background: ${backgroundColor(lane)};`;

  node.appendChild(wrapInCenterDiv(typeIcon(lane)));
  const dir = directionIcon(lane);
  if (dir) {
    node.appendChild(wrapInCenterDiv(dir));
  }
  node.appendChild(wrapInCenterDiv(width(lane)));

  var finalRow = document.createElement("div");

  var left = iconObj("left");
  if (idx != 0) {
    left.onclick = () => {
      const array = app.road.lanes;
      [array[idx - 1], array[idx]] = [array[idx], array[idx - 1]];
      app.render();
    };
  } else {
    left.disabled = true;
  }
  finalRow.appendChild(left);
  finalRow.align = "center";

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
    right.disabled = true;
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

  var text = document.createElement("b");
  text.innerText = `${lane.type}, ${lane.designated}`;
  return text;
}

function directionIcon(lane) {
  if (lane.direction == "forward") {
    var obj = iconObj("forwards");
    obj.onclick = () => {
      lane.direction = "backward";
      app.render();
    };
    return obj;
  }
  if (lane.direction == "backward") {
    var obj = iconObj("backwards");
    obj.onclick = () => {
      lane.direction = "forward";
      app.render();
    };
    return obj;
  }
  if (lane.direction == "both") {
    return icon("both_ways");
  }
  return null;
}

function backgroundColor(lane) {
  if (lane.type == "travel" && lane.designated == "bicycle") {
    return "#0F7D4B";
  }
  return "grey";
}

function width(lane) {
  var div = document.createElement("div");
  div.align = "center";
  if (lane.width) {
    div.innerText = `${lane.width}m`;
  }
  return div;
}

function icon(name) {
  var img = document.createElement("img");
  img.src = `assets/${name}.svg`;
  img.className = "icon";
  return img;
}
function iconObj(name) {
  var btn = document.createElement("button");
  btn.type = "button";

  var img = document.createElement("img");
  img.src = `assets/${name}.svg`;

  btn.appendChild(img);

  return btn;
}

function wrapInCenterDiv(obj) {
  var div = document.createElement("div");
  div.align = "center";
  div.appendChild(obj);
  return div;
}
