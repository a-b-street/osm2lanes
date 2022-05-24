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

#[derive(Properties, PartialEq)]
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
        highlighted_html_for_string(&props.code, &ss, syntax, &ts.themes["base16-ocean.dark"])
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
                            match get_way(&way_id).await {
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
                <Control callback_msg={callback_msg.clone()} state={self.state.clone()}/>
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
                <Canvas callback_error={callback_error} state={self.state.clone()}/>
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

    fn view_lane_type(&self, lane: &Lane) -> Html {
        html! {
            <div class="lane"><span>{lane.as_utf8()}</span></div>
        }
    }
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

fn main() {
    console_log::init_with_level(log::Level::Debug).expect("logging failed");
    log::trace!("Initializing yew...");
    yew::start_app::<App>();
}
