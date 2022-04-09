use std::str::FromStr;

use piet::Error as PietError;
use piet_web::WebRenderContext;
use wasm_bindgen::JsCast;
use web_sys::{window, HtmlCanvasElement, HtmlInputElement, HtmlSelectElement};
use yew::prelude::*;

mod draw;

use osm2lanes::locale::{Country, DrivingSide, Locale};
use osm2lanes::overpass::get_way;
use osm2lanes::road::{Lane, Printable, Road};
use osm2lanes::tag::Tags;
use osm2lanes::transform::{
    lanes_to_tags, tags_to_lanes, LanesToTagsConfig, RoadFromTags, TagsToLanesConfig,
};

// Use `wee_alloc` as the global allocator.
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

type ShouldRender = bool;

#[derive(Debug)]
pub enum RenderError {
    Piet(PietError),
    UnknownLane,
    UnknownSeparator,
}

impl From<PietError> for RenderError {
    fn from(e: PietError) -> Self {
        Self::Piet(e)
    }
}

impl std::error::Error for RenderError {}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownLane => write!(f, "error rendering unknown lane"),
            Self::UnknownSeparator => write!(f, "error rendering unknown separator"),
            Self::Piet(p) => write!(f, "{}", p),
        }
    }
}

#[derive(Debug)]
pub struct State {
    pub locale: Locale,
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
    TagsLocaleSet((Tags, Locale)),
    ToggleDrivingSide,
    CountrySet(Result<Country, &'static str>),
    WayFetch,
    Error(String),
}

pub struct App {
    focus_ref: NodeRef,
    state: State,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        let locale = Locale::builder().build();
        let focus_ref = NodeRef::default();
        let edit_tags = "highway=secondary\ncycleway:right=track\nlanes=6\nlanes:backward=2\nbusway=lane\nsidewalk=right".to_owned();
        let state = State {
            locale,
            edit_tags,
            normalized_tags: None,
            road: None,
            message: None,
            way_ref: NodeRef::default(),
        };
        let mut app = Self { focus_ref, state };
        app.update_tags();
        app
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> ShouldRender {
        log::trace!("Message: {:?}", msg);
        match msg {
            Msg::TagsSet(tags) => {
                self.state.edit_tags = tags;
                self.update_tags();
                true
            },
            Msg::TagsLocaleSet((tags, locale)) => {
                self.state.edit_tags = tags.to_string();
                self.state.locale = locale;
                self.update_tags();
                true
            },
            Msg::ToggleDrivingSide => {
                self.state.locale.driving_side = self.state.locale.driving_side.opposite();
                self.update_tags();
                true
            },
            Msg::CountrySet(Ok(country)) => {
                self.state.locale.country = Some(country);
                self.update_tags();
                true
            },
            Msg::CountrySet(Err(country_err)) => {
                self.state.message = Some(country_err.to_owned());
                true
            },
            Msg::WayFetch => {
                let way_id = self
                    .state
                    .way_ref
                    .cast::<HtmlInputElement>()
                    .unwrap()
                    .value();
                log::debug!("WayFetch {}", way_id);
                match way_id.parse() {
                    Ok(way_id) => {
                        ctx.link().send_future(async move {
                            match get_way(way_id).await {
                                Ok((tags, locale)) => Msg::TagsLocaleSet((tags, locale)),
                                Err(e) => Msg::Error(e.to_string()),
                            }
                        });
                    },
                    Err(e) => self.state.message = Some(format!("Invalid way id: {}", e)),
                }
                true
            },
            Msg::Error(e) => {
                self.state.message = Some(format!("Error: {}", e));
                true
            },
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let edit = move |input: HtmlInputElement| Msg::TagsSet(input.value());

        let onblur = ctx
            .link()
            .callback(move |e: FocusEvent| edit(e.target_unchecked_into()));

        let onkeypress = ctx.link().batch_callback(move |e: KeyboardEvent| {
            (e.key() == "Enter").then(|| edit(e.target_unchecked_into()))
        });

        let driving_side_onchange = ctx.link().callback(|_e: Event| Msg::ToggleDrivingSide);
        let country_onchange = ctx.link().callback(move |e: Event| {
            let selected: String = e.target_unchecked_into::<HtmlSelectElement>().value();
            let selected = Country::from_alpha2(selected);
            Msg::CountrySet(selected)
        });

        let way_id_onclick = ctx.link().callback(|_| Msg::WayFetch);

        let countries = {
            let mut countries: Vec<(&str, bool)> = Country::get_countries()
                .into_iter()
                .map(|country| {
                    (
                        country.alpha2,
                        self.state
                            .locale
                            .country
                            .as_ref()
                            .map_or(false, |locale| locale == &country),
                    )
                })
                .collect();
            countries.sort_unstable_by_key(|c| c.0);
            log::trace!("countries {:?}", countries);
            countries
        };

        html! {
            <div>
                <h1>{"osm2lanes"}</h1>
                <section class="row">
                    <button class="row-item">
                        {"Calculate"}
                    </button>
                    <hr/>
                    <p class="row-item">
                        {"↑↓ LHT"}
                    </p>
                    <label class="row-item switch">
                        <input
                            type="checkbox"
                            checked={self.state.locale.driving_side == DrivingSide::Right}
                            onchange={driving_side_onchange}
                        />
                        <span class="slider"></span>
                    </label>
                    <p class="row-item">
                        {"RHT ↓↑"}
                    </p>
                    <hr/>
                    <select onchange={country_onchange} class={
                        if self.state.locale.iso_3166_2_subdivision.is_some() {"suffixed"} else {""}
                    }>
                    {
                        for countries.into_iter().map(|(country, selected)| html!{
                            <option value={country} selected={selected}>
                                {country}
                            </option>
                        })
                    }
                    </select>
                    {
                        if let Some(code) = &self.state.locale.iso_3166_2_subdivision {
                            html!{
                                <p class="row-item prefixed">
                                    {"-"}{code}
                                </p>
                            }
                        } else {
                            html!{}
                        }
                    }
                    <hr/>
                    <label class="row-item" for="way">{"OSM Way ID"}</label>
                    <input class="row-item" type="text" id="way" name="way" size="12" ref={self.state.way_ref.clone()}/>
                    <button class="row-item" onclick={way_id_onclick}>
                        {"Fetch"}
                    </button>
                </section>
                <section class="row">
                    <div class="row-item">
                        <textarea
                            rows={(self.state.edit_tags.lines().count() + 1).to_string()}
                            cols="48"
                            ref={self.focus_ref.clone()}
                            value={self.state.edit_tags.clone()}
                            {onblur}
                            {onkeypress}
                            autocomplete={"off"}
                            spellcheck={"false"}
                        />
                    </div>
                    <div class="row-item">
                        <p>{"➔"}</p>
                    </div>
                    <div class="row-item">
                        <textarea
                            readonly=true
                            disabled={self.state.normalized_tags.is_none()}
                            rows={
                                if let Some(tags) = &self.state.normalized_tags {
                                    (tags.lines().count() + 1).to_string()
                                } else {
                                    "1".to_owned()
                                }
                            }
                            cols="48"
                            ref={self.focus_ref.clone()}
                            value={
                                if let Some(tags) = &self.state.normalized_tags {
                                    tags.clone()
                                } else {
                                    "".to_owned()
                                }
                            }
                            spellcheck={"false"}
                        />
                    </div>
                </section>
                <hr/>
                {
                    if let Some(message) = &self.state.message {
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
                    if let Some(road) = &self.state.road {
                        html!{
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
                        }
                    } else {
                        html!{}
                    }
                }
                <canvas id="canvas" width="960px" height="480px"></canvas>
            </div>
        }
    }

    fn rendered(&mut self, _ctx: &Context<Self>, _first_render: bool) {
        if let Err(e) = self.draw_canvas() {
            self.state.message = Some(format!("Error: {}", e));
        }
    }
}

impl App {
    fn update_tags(&mut self) {
        let value = &self.state.edit_tags;
        let locale = &self.state.locale;
        log::trace!("Update Tags: {}", value);
        log::trace!("Locale: {:?}", locale);
        match Tags::from_str(value) {
            Ok(tags) => match tags_to_lanes(&tags, locale, &TagsToLanesConfig::default()) {
                Ok(RoadFromTags { road, warnings }) => {
                    match lanes_to_tags(&road.lanes, locale, &LanesToTagsConfig::new(false)) {
                        Ok(tags) => {
                            self.state.road = Some(road);
                            self.state.normalized_tags = Some(tags.to_string());
                            if warnings.is_empty() {
                                self.state.message = None;
                            } else {
                                self.state.message = Some(warnings.to_string());
                            }
                        },
                        Err(error) => {
                            self.state.road = Some(road);
                            self.state.normalized_tags = None;
                            if warnings.is_empty() {
                                self.state.message =
                                    Some(format!("Lanes to Tags Error: {}", error));
                            } else {
                                self.state.message =
                                    Some(format!("{}\nLanes to Tags Error: {}", warnings, error));
                            }
                        },
                    }
                },
                Err(road_error) => {
                    self.state.road = None;
                    self.state.normalized_tags = None;
                    self.state.message = Some(format!("Conversion Error: {}", road_error));
                },
            },
            Err(tags_error) => {
                self.state.road = None;
                self.state.normalized_tags = None;
                self.state.message = Some(format!("Conversion Error: {}", tags_error));
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
    fn draw_canvas(&self) -> Result<(), RenderError> {
        if let Some(road) = &self.state.road {
            let window = window().unwrap();
            let canvas = window
                .document()
                .unwrap()
                .get_element_by_id("canvas")
                .unwrap()
                .dyn_into::<HtmlCanvasElement>()
                .unwrap();
            let context = canvas
                .get_context("2d")
                .unwrap()
                .unwrap()
                .dyn_into::<web_sys::CanvasRenderingContext2d>()
                .unwrap();

            let dpr = window.device_pixel_ratio();
            let canvas_width = (canvas.offset_width() as f64 * dpr) as u32;
            let canvas_height = (canvas.offset_height() as f64 * dpr) as u32;
            canvas.set_width(canvas_width);
            canvas.set_height(canvas_height);
            context.scale(dpr, dpr).unwrap();
            let mut rc = WebRenderContext::new(context, window);

            draw::lanes(
                &mut rc,
                (canvas_width, canvas_height),
                road,
                &self.state.locale,
            )?;
        }
        Ok(())
    }
}

fn main() {
    console_log::init_with_level(log::Level::Trace).expect("logging failed");
    log::trace!("Initializing yew...");
    yew::start_app::<App>();
}
