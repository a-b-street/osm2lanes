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

  var editRow = document.createElement("div");
  editRow.style = "text-align: center";

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
  editRow.appendChild(left);

  var trash = iconObj("delete");
  trash.onclick = () => {
    app.road.lanes.splice(idx, 1);
    app.render();
  };
  editRow.appendChild(trash);

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
  editRow.appendChild(right);

  node.append(editRow);

  if (idx != app.road.lanes.length - 1) {
    var add = iconObj("add");
    add.className = "insert-lane";
    add.onclick = () => {
      app.road.lanes.splice(idx + 1, 0, {
        type: "travel",
        direction: "backward",
        designated: "motor_vehicle",
      });
      app.render();
    };
    node.append(add);
  }

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
  if (lane.type == "travel" && lane.designated == "foot") {
    return icon("pedestrian");
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
  div.style = "text-align: center";
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
  div.style = "text-align: center";
  div.appendChild(obj);
  return div;
}
