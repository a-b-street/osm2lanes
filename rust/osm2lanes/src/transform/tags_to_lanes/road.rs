use crate::tag::TagKey;
use crate::transform::RoadMsg::Unimplemented;
use crate::transform::{RoadFromTags, RoadMsg, RoadWarnings, Tags};
use crate::Locale;

pub fn tags_to_lanes(tags: &Tags, locale: &Locale) -> Result<RoadFromTags, RoadError> {
    let mut road: Road = Road::new(locale);

    road.set_tags::<Oneway>(tags);
    road.set_tags::<Busway>(&tags);
    road.set_tags::<Lanes>(&tags);

    // if (guessing_mode)
    {
        road.guess::<Oneway>();
        road.guess::<Busway>();
        road.guess::<Lanes>();
    }

    road.calculate_lanes_ltr().into()
}

/// A collection of schemas. We have a Hunch about each schema because we need at least an option,
/// so why not track it as a Hunch?
struct Road {
    busway: Hunch<Busway>,
    lanes: Hunch<Lanes>,
    // map_of_schema_to_hunches
}

impl Road {
    /// Sets the Hunch for the schema to a Tagged* one, with tags applied.
    fn set_tags<S: Scheme>(&mut self, tags: &Tags) {
        let x = self.get_hunch::<S>();
        self.set_hunch::<S>(x.tagged_to_be(x.ok().set_tags(&tags)))
    }
    fn get<T: Scheme>(&self) -> T {}
    fn get_hunch<T: Scheme>(&self) -> Hunch<T> {} // Not Unknown
    fn set_hunch<T: Scheme>(&self, scheme: Hunch<T>) {}

    // pub fn busway(&self) -> &mut Busway {
    //     match self.busway.some() {
    //         None => {
    //             self.busway = Assume(Busway::default(self));
    //             self.busway.to_mut_ref();
    //             &mut b
    //         }
    //         Some(b) => &mut b,
    //     }
    // }
}

type Maybe<T> = Option<T>;
type Problem = RoadMsg;
type Count = usize;
type Qty = f64;
enum Highway {
    // Piste, // Track, // Bridleway,
    Footway,
    Cycleway,
    Service,
    Road,
    Street,
    Highway,
    Freeway,
}
use Highway::*;
use crate::road::LaneDirection;

/// A value of different certainties. Hunches combine together into another hunch in predefined ways.
enum Hunch<T> {
    Unknown,
    UnknownWithProblem(Problem),
    Tagged(T),
    GuessWithProblem(T, Problem, Tagged(T)),
    Implied(T), // Implied(T, &Tags), ImpliedWithConflict(Implied, Problem, Tagged<T>)
    Assume(T),
    GuessWithBound(T, T),
    Guess(T),
}
// I'm not happy with the error tracking, maybe that comes up a level, Hunches lose *WithProblem,
// and we go full history, with as follows, which implements the combining logic:
// struct HunchWithSources<T> {
//     hunch: Hunch<T>,
//     sources: Vec<Hunch<T>>,
//     problem: Vec<Problem>,
// }
use Hunch::*;


/// A set of Hunches that describe canonically the state of a schema.
trait Scheme {
    /// Get a scheme with all the base Assumptions.
    fn default_for_road(road: &Road) -> Self;
    /// Set tagged values and update conflicts.
    fn set_tags(mut self, tags: &Tags);
    /// Spruce it up with Guesses, make it look plausible, given what we know.
    fn guess(mut self, road: &Road);

    // fn problems(&self) -> Option<Vec<Problem>>;
}

enum Oneway {
    Yes,
    Reverse,
    No,
}

struct Lanes {
    lanes: Hunch<Count>,
    forward: Hunch<Count>,
    backward: Hunch<Count>,
    bothways: Hunch<Count>,
    directions: Hunch<Vec<LaneDirection>>,
}

struct Busway {
    forward: Hunch<Maybe<LaneDirection>>,
    backward: Hunch<Maybe<LaneDirection>>,
}

impl Schema for Lanes {
    // type Err = RoadMsg;
    fn new(forward: Hunch<Count>, backward: Hunch<Count>, bothways: Hunch<Count>) -> Self {
        Self {
            lanes: Hunch::Implied(&forward) + &backward + &bothways,
            forward,
            backward,
            bothways,
            directions,
        }
    }
    fn new_with_lanes(lanes: Hunch<Count>) -> Self {
        let half = lanes.copy() / 2;
        let middle = lanes.copy() % 2;
        Self {
            lanes,
            forward: Guess::from(half),
            backward: Guess::from(half),
            bothways: Guess::from(middle).withBound(0),
            directions: Hunch::None,
        }
    }

    const LANES: TagKey = TagKey::from("lanes");
    const DIRECTION: TagKey = TagKey::from("direction");
    fn set_tags(mut self, tags: &Tags) -> Self {
        let tags_token: usize = tags.start_building_subset();
        self.forward
            .tagged_to_be(tags.get_and_parse(LANES + "forward"));
        self.backward
            .tagged_to_be(tags.get_and_parse(LANES + "backward"));
        self.bothways
            .tagged_to_be(tags.get_and_parse(LANES + "both_ways"));
        self.lanes.tagged_to_be(tags.get_and_parse(LANES));
        let used_tags: Tags = tags.withdraw_subset(tags_token);

        lanes.implied_to_be(&self.forward + &self.backward + &self.bothways, || {
            RoadMsg::conflict("lane counts dont agree", used_tags)
        });

        // TODO read and set conflicts on road for related schemas, like in default

        // let mut directions = Unknown; // TODO WithProblem(RoadMsg::unimplimented("", tags.subset(&[DIRECTION + LANES]));

        self
    }
    fn guess(mut self, road: &Road) -> Self {
        // guess
        // self
        self
    }
    fn default(road: &Road) -> Self {
        let default_each_way = match road.highway {
            Footway | Cycleway | Road => 1,
            Street | Highway => 2,
            Freeway => 3,
        };

        // Add up bus lanes to aid in our assumptions.
        // TODO Busway probably feeds into LanesRestricted (Bus&PSV) which feeds in here
        let busway = road.get::<Busway>();
        // to_assumption would map the value, while downgrading to assumption e.g. passing Guess on.
        let buslanes_forward: Hunch<Count> = busway.forward.to_assumption(|f| match f {
            Some(_) => 1, // TODO what to do with buslanes going against the normal direction?
            None => 0,
        });
        let buslanes_backward: Hunch<Count> = busway.backward.to_assumption(|f| match f {
            Some(_) => 1,
            None => 0,
        });

        match road.oneway {
            Oneway::Yes => match road.highway {
                Footway => Self::new(
                    Assume(default_each_way) + buslanes_forward,
                    Implied(buslanes_backward),
                    Implied(0),
                ),
                Cycleway | Road | Street | Highway | Freeway => Self::new(
                    Assume(default_each_way + buslanes_forward), // AssumeWithBound(_, buslanes_forward)
                    Implied(buslanes_backward),
                    Implied(0),
                ),
            },
            Oneway::Reverse => match road.highway {
                Footway => Self::new(Assume(buslanes_forward), Assume(default_each_way) + buslanes_backward, Assume(0)), // ??
                Cycleway | Road | Street | Highway | Freeway => {
                    Self::new(Implied::from(buslanes_forward), Assume(default_each_way) + buslanes_backward, Implied(0))
                }
            },

            Oneway::No => match road.highway {
                Footway => Self::new(Assume(0), Assume(0), Assume(default_each_way)),
                Cycleway | Road | Street | Highway | Freeway => Self::new(
                    Assume(default_each_way + buslanes_forward),
                    Assume(default_each_way + buslanes_backward),
                    Assume(0),
                ),
            },
        }
    }
}

impl Busway {
    fn new(forward: Hunch<Maybe<LaneDirection>>, backward: Hunch<Maybe<LaneDirection>>) -> Self {
        Self { forward, backward }
    }
    fn default(_road: &Road) -> Self {
        Self::new(Assume(None), Assume(None))
    }
    fn set_tags(mut self) -> Self {
        //TODO
        self
    }
}

impl Hunch<T> {
    fn some(&self) -> Maybe<&T> {
        match self {
            Unknown | UnknownWithProblem(_) => None,
            Assume(t)
            // | AssumeWithBound(t, _)
            | Guess(t)
            | GuessWithBound(t, _)
            | Implied(t)
            | Tagged(t)
            | GuessWithProblem(t, _, _) => Some(t),
        }
    }
    // fn useful -> Result<T, E>;

    fn tagged_to_be(&mut self, val: T) -> Self {
        // Function like this one should implement the meat of working with hunches.

        // I have no idea what level of error tracking is appropriate or useful.
        // Maybe a streamlined error propagation approach is desirable.
        // And how much information about implications, guesses, etc should be tracked?

        match self {
            Unknown | Assume(_) | Guess(_) | Tagged(_) | Implied(val) => Tagged(val),
            Implied(t) => ImpliedWithConflict(
                t,
                RoadMsg::conflict_str("tag conflicts with implied value"),
                Tagged(val),
            ),

            GuessWithBound(_, t) => {
                if t > val {
                    // Getting carried away with error propagation
                    TaggedWithProblem(val, RoadMsg::conflict_str("tag conflicts with an atleast"))
                } else {
                    Tagged(val)
                }
            }
            GuessWithProblem(_, e, t) => TaggedWithProblem(val, e, t),
            // ...
        }
    }
}

impl std::ops::Add<&Hunch<T>> for Hunch<T> {
    type Output = Self;
    // TODO generic for ops?
    fn add(self: &Self, rhs: &Hunch<T>) -> Self {
        match (self.some(), rhs.some()) {
            (None, None) => Unknown,
            (Some(a), None) | (None, Some(a)) => a,
            (Some(l), Some(r)) => {
                let a = l + r;
                match (self, rhs) {
                    (GuessWithProblem(_, _, _), _)
                    | (_, GuessWithProblem(_, _, _))
                    | (Guess(_), _)
                    | (_, Guess(_)) => Guess(a),
                    (Assume(_), _) | (_, Assume(_)) => Assume(a),
                    (GuessWithBound(_, l), GuessWithBound(_, r)) => GuessWithBound(a, l + r),
                    (GuessWithBound(_, b), _) | (_, GuessWithBound(_, b)) => GuessWithBound(a, b),
                    (Implied(_), _) | (_, Implied(_)) => Implied(a),
                    (Tagged(t), Tagged(T)) => Implied,
                }
            }
        }
    }
}
