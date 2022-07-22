// Fetching from overpass is slow; to speed up development, use hardcoded output
export function dummyData() {
  return {
    Ok: {
      road: {
        name: "24th Avenue East",
        highway: "secondary",
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
            semantic: "lane",
            markings: [
              {
                style: "dotted_line",
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
                style: "solid_line",
                width: 0.2,
                color: "white",
              },
              {
                style: "no_fill",
                width: 0.1,
                color: null,
              },
              {
                style: "solid_line",
                width: 0.2,
                color: "white",
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
            semantic: "lane",
            markings: [
              {
                style: "dotted_line",
                width: 0.2,
                color: "white",
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
            semantic: "lane",
            markings: [
              {
                style: "dotted_line",
                width: 0.2,
                color: "white",
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
}
