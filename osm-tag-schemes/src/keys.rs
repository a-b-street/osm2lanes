use osm_tags::TagKey;

pub const NAME: TagKey = TagKey::from_static("name");
pub const REF: TagKey = TagKey::from_static("ref");

pub const HIGHWAY: TagKey = TagKey::from_static("highway");
pub const CONSTRUCTION: TagKey = TagKey::from_static("construction");
pub const PROPOSED: TagKey = TagKey::from_static("proposed");
pub const LIFECYCLE: [TagKey; 3] = [HIGHWAY, CONSTRUCTION, PROPOSED];

pub const ONEWAY: TagKey = TagKey::from_static("oneway");

pub const LIT: TagKey = TagKey::from_static("lit");

pub const TRACK_TYPE: TagKey = TagKey::from_static("tracktype");
pub const SMOOTHNESS: TagKey = TagKey::from_static("smoothness");

pub const LANES: TagKey = TagKey::from_static("lanes");
// Unstable: const evaluation https://github.com/rust-lang/rust/issues/90080
pub const LANES_FORWARD: TagKey = TagKey::from_static("lanes:forward");
pub const LANES_BACKWARD: TagKey = TagKey::from_static("lanes:backward");
