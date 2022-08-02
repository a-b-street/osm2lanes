// Fetching from overpass is slow; to speed up development, use hardcoded output
export function dummyData() {
  const road = {
    Ok: {
      road: {
        name: "10th Avenue East",
        highway: "secondary",
        lit: "yes",
        lanes: [
          {
            type: "shoulder",
          },
          {
            type: "separator",
            semantic: "shoulder",
            markings: [
              {
                style: "solid_line",
                width: 0.2,
                color: "white",
              },
            ],
          },
          {
            type: "parking",
            direction: "backward",
            designated: "motor_vehicle",
          },
          {
            type: "separator",
            semantic: "modal",
            markings: [
              {
                style: "solid_line",
                width: 0.2,
                color: "white",
              },
            ],
          },
          {
            type: "travel",
            direction: "backward",
            designated: "bicycle",
          },
          {
            type: "separator",
            semantic: "modal",
            markings: [
              {
                style: "solid_line",
                width: 0.2,
                color: "white",
              },
            ],
          },
          {
            type: "travel",
            direction: "backward",
            designated: "motor_vehicle",
            width: 3.5,
            max_speed: {
              unit: "mph",
              value: 25.0,
            },
          },
          {
            type: "separator",
            semantic: "centre",
            markings: [
              {
                style: "dotted_line",
                width: 0.2,
                color: "yellow",
              },
            ],
          },
          {
            type: "travel",
            direction: "forward",
            designated: "motor_vehicle",
            width: 3.5,
            max_speed: {
              unit: "mph",
              value: 25.0,
            },
          },
          {
            type: "separator",
            semantic: "centre",
            markings: [
              {
                style: "dotted_line",
                width: 0.2,
                color: "yellow",
              },
            ],
          },
          {
            type: "parking",
            direction: "forward",
            designated: "motor_vehicle",
          },
          {
            type: "separator",
            semantic: "shoulder",
            markings: [
              {
                style: "solid_line",
                width: 0.2,
                color: "white",
              },
            ],
          },
          {
            type: "shoulder",
          },
        ],
      },
    },
  };
  const locale = {
    country: "US",
    iso_3166_2_subdivision: "WA",
    driving_side: "right",
  };
  return [road, locale];
}
