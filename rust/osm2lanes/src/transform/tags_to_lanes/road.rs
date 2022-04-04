use std::str::FromStr;

use Highway::*;
use Hunch::*;

use crate::road::LaneDirection;
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

type Problem = String;
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

/// A value of different certainties. Hunches combine together into another hunch in predefined ways.
enum Hunch<T> {
    // Unknown,
    Tagged(T),
    Implied(T),
    Assumed(T),
    Guessed(T),
}
impl<T> Hunch<T> {
    fn get_ref(&self) -> &T {
        match self {
            Tagged(v) | Implied(v) | Assumed(v) | Guessed(v) => &v,
        }
    }

    fn with<U>(&self, val: U) -> Hunch<U> {
        match self {
            Tagged(_) => Tagged(val),
            Implied(_) => Implied(val),
            Assumed(_) => Assumed(val),
            Guessed(_) => Guessed(val),
        }
    }
    /// Returns a hunch of the value, with an variation appropriate to combining information from
    /// two other hunches.
    fn combine<U, V>(val: T, a: &Hunch<U>, b: &Hunch<V>) -> Hunch<T> {
        match (a, b) {
            (Guessed(_), _) | (_, Guessed(_)) => Guessed(val),
            (Assumed(_), _) | (_, Assumed(_)) => Assumed(val),
            (Implied(_), _) | (_, Implied(_)) => Implied(val),
            (Tagged(_), _) | (_, Tagged(_)) => Tagged(val),
        }
    }
    fn downgrade<S: HunchStrength>(self, sources: &Vec<&S>) -> Self {
        // TODO should implement the sortable trait to make this easier:
        let mut strength = &self;
        for s in sources {
            match (strength, s.strength()) {
                (Guessed(_), _) | (_, Guessed(_)) => Guessed(()),
                (Assumed(_), _) | (_, Assumed(_)) => Assumed(()),
                (Implied(_), _) | (_, Implied(_)) => Implied(()),
                (Tagged(_), _) | (_, Tagged(_)) => Tagged(()),
            }
        }
        strength.with(self.into_inner())
    }
    // fn combine<I, U where I: Iter<&Hunch<U>>>(val: T, hunches: I) {
    //
    // }
    /// Returns the stronger of two hunches (preferring self), emitting an error if there is a conflict.
    fn reconcile(self: &Hunch<T>, b: &Hunch<T>, on_conflict: fn()) -> Hunch<T> {
        match (self, b) {
            (Implied(i), Tagged(t)) | (Tagged(t), Implied(i)) if t != i => {
                on_conflict();
                Implied(i) // TODO return self instead?
            }
            (Tagged(v), _) | (_, Tagged(v)) => Ok(Tagged(v)),
            (Implied(v), _) | (_, Implied(v)) => Ok(Implied(v)),
            (Assumed(v), _) | (_, Assumed(v)) => Ok(Assumed(v)),
            (Guessed(v), _) | (_, Guessed(v)) => Ok(Guessed(v)),
        }
    }
}

trait HunchStrength {
    fn strength(&self) -> Hunch<()>;
}
impl HunchStrength for Hunch<_> {
    fn strength(&self) -> Hunch<()> {
        match self {
            Tagged(_) => Tagged(()),
            Implied(_) => Implied(()),
            Assumed(_) => Assumed(()),
            Guessed(_) => Guessed(()),
        }
    }
}
impl HunchStrength for Theory<_> {
    fn strength(&self) -> Hunch<()> {
        match self.hunch() {
            Some(h) => h.strength(),
            None => Guessed(()),
        }
    }
}

// trait HasProblems { fn iter<Problem>... }
// type TheoryHistory = Vec<Problem>;
struct Theory<'a, T> {
    history: Vec<(Hunch<T>, Vec<&'a Theory<'a, T>>)>,
    // history: Vec<(Hunch<T>, &impl(HasProblems + HunchStrength))>
    problems: Option<Vec<Problem>>,
}
impl<'a, T> Theory<'a, T> {
    fn hunch(self) -> Option<&'a Hunch<T>> {
        match self.history {
            [] => None,
            [.., (h, _)] => Some(h),
        }
    }

    fn update(
        &mut self,
        new_hunch: Hunch<T>,
        supporting_theories: Vec<&Theory<T>>,
        transform_err: Option<fn() -> Problem>,
    ) /*-> Result<(), ()>*/
    {
        self.history.push((
            if let Some(existing_hunch) = self.hunch() {
                new_hunch.reconcile(existing_hunch, || {
                    self.problems.push(match transform_err {
                        Some(f) => f(),
                        None => "conflict",
                    })
                })
            } else {
                new_hunch
            },
            supporting_theories,
        ))
    }

    fn new(new_hunch: Hunch<T>, supporting_theories: Vec<&Theory<T>>) -> Self {
        let mut s = Self::default();
        s.update(new_hunch, supporting_theories, None);
        s
    }

    fn default() -> Self {
        Self {
            history: Vec::default(),
            problems: None,
        }
    }
    fn from(hunch: Hunch<T>) -> Self {
        Self {
            history: vec![(hunch, Vec::default())],
            problems: None,
        }
    }
}

impl<'a, T: FromStr> Theory<'a, T> {
    // Convenience
    fn update_with_tag(&mut self, tag_str: Option<&str>) {
        if let Some(tag) = tag_str {
            match tag.parse() {
                Ok(val) => self.update(Tagged(val), Vec::default(), None),
                Err(e) => self.problems.push(e.description()),
            }
        }
    }
}

/// A set of Hunches that describe canonically the state of a schema.
trait Scheme {
    /// Get a scheme with all the base Assumptions.
    fn default_for_road(road: &Road) -> Self;
    /// Set tagged values and update conflicts.
    fn set_tags(&self, tags: &Tags);
    /// Spruce it up with Guesses, make it look plausible, given what we know.
    fn guess(&self, road: &Road);

    // fn problems(&self) -> Option<Vec<Problem>>;
}

enum Oneway {
    Yes,
    Reverse,
    No,
}

struct Lanes<'a> {
    lanes: Theory<'a, Count>,
    forward: Theory<'a, Count>,
    backward: Theory<'a, Count>,
    bothways: Theory<'a, Count>,
    // directions: Theory<'a, Vec<LaneDirection>>,
}

struct Busway<'a> {
    forward: Theory<'a, Option<LaneDirection>>,
    backward: Theory<'a, Option<LaneDirection>>,
}

impl Scheme for Lanes {
    // type Err = RoadMsg;
    fn default() -> Self {
        Self {
            lanes: Theory::default(),
            forward: Theory::default(),
            backward: Theory::default(),
            bothways: Theory::default(),
        }
    }

    fn new(forward: Theory<Count>, backward: Theory<Count>, bothways: Theory<Count>) -> Self {
        Self {
            lanes: match (forward.some(), backward.some(), bothways.some()) {
                (Some(fwd), Some(back), both) => Theory::new(
                    Implied(fwd + back + both.unwrap_or(0))
                        .downgrade(&vec![&forward, &backward, &bothways]),
                    vec![&forward, &backward, &bothways],
                ),
                _ => Theory::default(),
            },
            forward,
            backward,
            bothways,
            // directions,
        }
    }
    fn new_with_lanes(lanes: Theory<Count>) -> Self {
        if let Some(l) = lanes_theory.hunch() {
            let half = l.get_ref() / 2;
            let middle = l.get_ref() % 2;
            Self {
                lanes,
                forward: Theory::new(Assumed(half).downgrade(&vec![lanes]), vec![&lanes_theory]),
                backward: Theory::new(Assumed(half).downgrade(&vec![lanes]), vec![&lanes_theory]),
                bothways: Theory::new(Assumed(middle).downgrade(&vec![lanes]), vec![&lanes_theory]),
                // directions: ...
            }
        } else {
            Self {
                lanes: lanes_theory,
                forward: Theory::default(),
                backward: Theory::default(),
                bothways: Theory::default(),
            }
        }
    }

    const LANES: TagKey = TagKey::from("lanes");
    const DIRECTION: TagKey = TagKey::from("direction");
    fn set_tags(&mut self, tags: &Tags) -> &mut Self {
        // let tags_token: usize = tags.start_building_subset();
        self.lanes.update_with_tag(tags.get(LANES));
        self.forward.update_with_tag(tags.get(LANES + "forward"));
        self.backward.update_with_tag(tags.get(LANES + "backward"));
        self.bothways.update_with_tag(tags.get(LANES + "bothways"));
        // let used_tags: Tags = tags.withdraw_subset(tags_token);

        // forward, backward and bothways must add up to lanes.
        // let inputs = self.lanes.update(
        //     [&self.forward, &self.backward, &self.bothways],
        //     |fwd, back, both| Implied(fwd + back + both),
        //     "total lane count disagrees with direction specific lane counts",
        // );
        let inputs = vec![&self.forward, &self.backward, &self.bothways];
        if let (Some(fwd), Some(back), Some(both)) = (&self.forward, &self.backward, &self.bothways)
        {
            self.lanes.update(
                Implied(fwd + back + both).downgrade(&inputs),
                inputs,
                Some(|| {
                    String::from("lanes does not agree with lanes:{forward,backward,both_ways}")
                }),
            )
        }

        // TODO read and set conflicts on road for related schemas, like in default
        self
    }
    fn guess_for(mut self, road: &Road) -> Self {
        // guess
        // self
        self
    }
    // fn assumed_for(road: &Road) -> Self {
    //     let default_each_way = match road.highway {
    //         Footway | Cycleway | Road => 1,
    //         Street | Highway => 2,
    //         Freeway => 3,
    //     };
    //
    //     // Add up bus lanes to aid in our assumptions.
    //     // TODO Busway probably feeds into LanesRestricted (Bus&PSV) which feeds in here
    //     let busway = road.get::<Busway>();
    //     // to_assumption would map the value, while downgrading to assumption e.g. passing Guess on.
    //     let buslanes_forward: Hunch<Count> = busway.forward.to_assumption(|f| match f {
    //         Some(_) => 1, // TODO what to do with buslanes going against the normal direction?
    //         None => 0,
    //     });
    //     let buslanes_backward: Hunch<Count> = busway.backward.to_assumption(|f| match f {
    //         Some(_) => 1,
    //         None => 0,
    //     });
    //
    //     match road.oneway {
    //         Oneway::Yes => match road.highway {
    //             Footway => Self::new(
    //                 Assume(default_each_way) + buslanes_forward,
    //                 Implied(buslanes_backward),
    //                 Implied(0),
    //             ),
    //             Cycleway | Road | Street | Highway | Freeway => Self::new(
    //                 Assume(default_each_way + buslanes_forward), // AssumeWithBound(_, buslanes_forward)
    //                 Implied(buslanes_backward),
    //                 Implied(0),
    //             ),
    //         },
    //         Oneway::Reverse => match road.highway {
    //             Footway => Self::new(
    //                 Assume(buslanes_forward),
    //                 Assume(default_each_way) + buslanes_backward,
    //                 Assume(0),
    //             ), // ??
    //             Cycleway | Road | Street | Highway | Freeway => Self::new(
    //                 Implied::from(buslanes_forward),
    //                 Assume(default_each_way) + buslanes_backward,
    //                 Implied(0),
    //             ),
    //         },
    //
    //         Oneway::No => match road.highway {
    //             Footway => Self::new(Assume(0), Assume(0), Assume(default_each_way)),
    //             Cycleway | Road | Street | Highway | Freeway => Self::new(
    //                 Assume(default_each_way + buslanes_forward),
    //                 Assume(default_each_way + buslanes_backward),
    //                 Assume(0),
    //             ),
    //         },
    //     }
    // }
}

// impl Busway {
//     fn new(forward: Hunch<Option<LaneDirection>>, backward: Hunch<Option<LaneDirection>>) -> Self {
//         Self { forward, backward }
//     }
//     fn default(_road: &Road) -> Self {
//         Self::new(Assume(None), Assume(None))
//     }
//     fn set_tags(mut self) -> Self {
//         //TODO
//         self
//     }
// }
