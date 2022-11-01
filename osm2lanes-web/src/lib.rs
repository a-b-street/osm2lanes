#![warn(explicit_outlives_requirements)]
#![warn(missing_abi)]
#![deny(non_ascii_idents)]
#![warn(trivial_casts)]
#![warn(unreachable_pub)]
#![deny(unsafe_code)]
#![deny(unsafe_op_in_unsafe_fn)]
// #![warn(unused_crate_dependencies)] // https://github.com/rust-lang/rust/issues/57274
#![warn(unused_lifetimes)]
#![warn(unused_qualifications)]
// Clippy
#![warn(clippy::pedantic, clippy::cargo)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::cargo_common_metadata)]
#![warn(
    clippy::allow_attributes_without_reason,
    clippy::as_conversions,
    clippy::clone_on_ref_ptr,
    clippy::create_dir,
    clippy::dbg_macro,
    clippy::decimal_literal_representation,
    clippy::deref_by_slicing,
    clippy::empty_structs_with_brackets,
    clippy::float_cmp_const,
    clippy::fn_to_numeric_cast_any,
    clippy::if_then_some_else_none,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::map_err_ignore,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::single_char_lifetime_names,
    clippy::str_to_string,
    clippy::string_add,
    clippy::string_slice,
    clippy::string_to_string,
    clippy::todo,
    clippy::try_err,
    clippy::unseparated_literal_suffix,
    clippy::use_debug
)]

use std::cell::RefCell;
use std::rc::Rc;
use std::str::FromStr;

use osm2lanes::locale::{Country, Locale};
use osm2lanes::overpass::get_way;
use osm2lanes::road::{Lane, Printable, Road};
use osm2lanes::transform::{
    lanes_to_tags, tags_to_lanes, LanesToTagsConfig, RoadFromTags, TagsToLanesConfig,
};
use osm_tags::Tags;
use syntect::highlighting::ThemeSet;
use syntect::html::highlighted_html_for_string;
use syntect::parsing::SyntaxSet;
use web_sys::HtmlInputElement;
use yew::prelude::*;

pub mod agent;

mod control;
use control::Control;

mod canvas;
use canvas::Canvas;

mod draw;

mod map;
use map::MapComponent;

// Use `wee_alloc` as the global allocator.
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[derive(Properties, PartialEq, Eq)]
pub struct CodeProps {
    pub code: String,
}

#[function_component(CodeHtml)]
pub fn code_html(props: &CodeProps) -> Html {
    let html = {
        let ss = SyntaxSet::load_defaults_newlines();
        let ts = ThemeSet::load_defaults();
        let syntax = ss
            .find_syntax_by_token("json")
            .unwrap_or_else(|| ss.find_syntax_plain_text());
        highlighted_html_for_string(
            &props.code,
            &ss,
            syntax,
            ts.themes.get("base16-ocean.dark").unwrap(),
        )
        .unwrap()
    };
    let div = gloo_utils::document().create_element("code").unwrap();
    div.set_inner_html(&html);
    Html::VRef(div.into())
}

type ShouldRender = bool;

#[derive(Debug, PartialEq)]
pub struct State {
    pub locale: Locale,
    pub id: Option<String>,
    /// The editable input, line and equal separated tags
    pub edit_tags: String,
    /// The input normalised
    pub normalized_tags: Option<String>,
    /// Lanes to visualise
    pub road: Option<Road>,
    /// Message for user
    pub message: Option<String>,
    /// Ref to input for way id
    pub way_ref: NodeRef,
}

#[derive(Debug)]
pub enum Msg {
    TagsSet(String),
    TagsLocaleSet {
        id: String,
        tags: Tags,
        locale: Locale,
    },
    ToggleDrivingSide,
    CountrySet(Result<Country, &'static str>),
    WayFetch,
    Error(String),
}

pub struct App {
    state: Rc<RefCell<State>>,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        let locale = Locale::builder().iso_3166("FR").build();
        let edit_tags = "Loading...".to_owned();
        let state = Rc::new(RefCell::new(State {
            locale,
            id: None,
            edit_tags,
            normalized_tags: None,
            road: None,
            message: None,
            way_ref: NodeRef::default(),
        }));
        Self { state }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> ShouldRender {
        log::trace!("Message: {:?}", msg);
        match msg {
            Msg::TagsSet(tags) => {
                {
                    let mut state = self.state.borrow_mut();
                    state.edit_tags = tags;
                }
                self.update_tags();
                true
            },
            Msg::TagsLocaleSet { id, tags, locale } => {
                {
                    let mut state = self.state.borrow_mut();
                    state.edit_tags = tags.to_string();
                    state.locale = locale;
                    state.id = Some(id);
                }
                self.update_tags();
                true
            },
            Msg::ToggleDrivingSide => {
                {
                    let mut state = self.state.borrow_mut();
                    state.locale.driving_side = state.locale.driving_side.opposite();
                }
                self.update_tags();
                true
            },
            Msg::CountrySet(Ok(country)) => {
                {
                    let mut state = self.state.borrow_mut();
                    state.locale = Locale::builder()
                        .driving_side(state.locale.driving_side)
                        .country(country)
                        .build();
                }
                self.update_tags();
                true
            },
            Msg::CountrySet(Err(country_err)) => {
                let mut state = self.state.borrow_mut();
                state.message = Some(country_err.to_owned());
                true
            },
            Msg::WayFetch => {
                let mut state = self.state.borrow_mut();
                let way_id = state.way_ref.cast::<HtmlInputElement>().unwrap().value();
                log::debug!("WayFetch {}", way_id);
                match way_id.parse() {
                    Ok(way_id) => {
                        ctx.link().send_future(async move {
                            match get_way(way_id).await {
                                Ok((tags, _geom, locale)) => Msg::TagsLocaleSet {
                                    id: way_id.to_string(),
                                    tags,
                                    locale,
                                },
                                Err(e) => Msg::Error(e.to_string()),
                            }
                        });
                    },
                    Err(e) => state.message = Some(format!("Invalid way id: {}", e)),
                }
                true
            },
            Msg::Error(e) => {
                let mut state = self.state.borrow_mut();
                state.message = Some(format!("Error: {}", e));
                true
            },
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let state = self.state.borrow();

        let callback_error = ctx.link().callback(Msg::Error);
        let callback_msg = ctx.link().callback(|msg| msg);

        html! {
            <div>
                <h1>{"osm2lanes"}</h1>
                <Control callback_msg={callback_msg.clone()} state={Rc::clone(&self.state)}/>
                <hr/>
                {
                    if let Some(message) = &state.message {
                        html!{
                            <section>
                                <pre>
                                    {message}
                                </pre>
                                <hr/>
                            </section>
                        }
                    } else {
                        html!{}
                    }
                }
                {
                    if let Some(road) = &state.road {
                        html!{
                            <>
                            <section>
                                <div class="lanes">
                                    {
                                        for road.lanes.iter().map(|lane| self.view_lane_type(lane))
                                    }
                                </div>
                                <div class="lanes">
                                    {
                                        for road.lanes.iter().map(|lane| self.view_lane_direction(lane))
                                    }
                                </div>
                                <hr/>
                            </section>
                            <section>
                                <details>
                                <summary>
                                    {"JSON Output"}
                                </summary>
                                <div class="json">
                                    <CodeHtml code={serde_json::to_string_pretty(&road).unwrap()}/>
                                </div>
                                </details>
                            </section>
                            </>
                        }
                    } else {
                        html!{}
                    }
                }
                <>
                <section>
                    <details>
                    <summary>
                        {"Test case YAML"}
                    </summary>
                    <div class="json">
                        <p>{"You probably need to modify the expected output manually."}</p>
                        <CodeHtml code={
                            let way_id = state
                                .way_ref
                                .cast::<HtmlInputElement>()
                                .and_then(|elem| elem.value().parse::<i64>().ok());
                            generate_test_yaml(state.road.clone(), &state.edit_tags, &state.locale, way_id)
                        }/>
                    </div>
                    </details>
                </section>
                </>
                <Canvas callback_error={callback_error} state={Rc::clone(&self.state)}/>
                <hr/>
                <MapComponent callback_msg={callback_msg.clone()}/>
            </div>
        }
    }
}

impl App {
    fn update_tags(&mut self) {
        let mut state = self.state.borrow_mut();
        let value = &state.edit_tags;
        let locale = &state.locale;
        log::trace!("Update Tags: {}", value);
        log::trace!("Locale: {:?}", locale);
        match Tags::from_str(value) {
            Ok(tags) => match tags_to_lanes(&tags, locale, &TagsToLanesConfig::default()) {
                Ok(RoadFromTags { road, warnings }) => {
                    match lanes_to_tags(&road, locale, &LanesToTagsConfig::new(false)) {
                        Ok(tags) => {
                            state.road = Some(road);
                            state.normalized_tags = Some(tags.to_string());
                            if warnings.is_empty() {
                                state.message = None;
                            } else {
                                state.message = Some(warnings.to_string());
                            }
                        },
                        Err(error) => {
                            state.road = Some(road);
                            state.normalized_tags = None;
                            if warnings.is_empty() {
                                state.message = Some(format!("Lanes to Tags Error: {}", error));
                            } else {
                                state.message =
                                    Some(format!("{}\nLanes to Tags Error: {}", warnings, error));
                            }
                        },
                    }
                },
                Err(road_error) => {
                    state.road = None;
                    state.normalized_tags = None;
                    state.message = Some(format!("Conversion Error: {}", road_error));
                },
            },
            Err(tags_error) => {
                state.road = None;
                state.normalized_tags = None;
                state.message = Some(format!("Conversion Error: {}", tags_error));
            },
        };
    }

    #[allow(clippy::unused_self)]
    fn view_lane_type(&self, lane: &Lane) -> Html {
        html! {
            <div class="lane"><span>{lane.as_utf8()}</span></div>
        }
    }

    #[allow(clippy::unused_self)]
    fn view_lane_direction(&self, lane: &Lane) -> Html {
        html! {
            <div class="lane"><span>{if let Lane::Travel {
                direction: Some(direction),
                ..
            } = lane
            {
                direction.as_utf8()
            } else {
                ' '
            }}</span></div>
        }
    }
}

fn generate_test_yaml(
    road: Option<Road>,
    tags: &str,
    locale: &Locale,
    way_id: Option<i64>,
) -> String {
    use osm2lanes::test::{Expected, TestCase};

    let test = TestCase {
        way_id,
        link: None,
        comment: None,
        description: Some("fill me out".to_string()),
        example: None,
        driving_side: locale.driving_side,
        iso_3166_2: locale.iso_3166_2_subdivision.clone(),
        tags: Tags::from_str(tags).unwrap_or_else(|_| Tags::default()),
        expected: Expected::Road(road.unwrap_or_else(|| Road::empty())),
        rust: None,
    };
    // TODO Strip out road's name, ref, and other things we don't test for?
    // TODO Based on checkboxes, strip out separators
    let raw = serde_yaml::to_string(&test).unwrap();

    // serde_yaml explicitly lists "field: null" for None values. Filter these out, to match the
    // style of the test YAML.
    let mut output = String::new();
    for line in raw.lines() {
        if !line.ends_with(": null") {
            output.push_str(line);
            output.push_str("\n");
        }
    }
    output
}
