use std::str::FromStr;

use piet_web::WebRenderContext;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use web_sys::{window, HtmlCanvasElement, HtmlInputElement};
use yew::prelude::*;

mod draw;

use osm2lanes::{lanes_to_tags, tags_to_lanes, LanesToTagsConfig, TagsToLanesConfig};
use osm2lanes::{DrivingSide, Locale};
use osm2lanes::{Lane, LanePrintable, Lanes, Tags};

// Use `wee_alloc` as the global allocator.
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

type ShouldRender = bool;

#[derive(Debug, Serialize, Deserialize)]
pub struct State {
    pub locale: Locale,
    /// The editable input, line and equal separated tags
    pub edit_tags: String,
    /// The input normalised
    pub normalized_tags: Option<String>,
    /// Lanes to visualise
    pub lanes: Vec<Lane>,
    /// Message for user
    pub message: Option<String>,
}

#[derive(Debug)]
pub enum Msg {
    Submit(String),
    Focus,
    ToggleDrivingSide,
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
        let edit_tags = "highway=secondary\ncycleway:right=track\nlanes=6\nlanes:backward=2\nlanes:bus=1\nsidewalk=right".to_owned();
        let state = if let Ok((Lanes { lanes, warnings }, norm_tags)) =
            Self::calculate(&edit_tags, &locale)
        {
            State {
                locale,
                edit_tags,
                normalized_tags: Some(norm_tags.to_string()),
                lanes,
                message: Some(warnings.to_string()),
            }
        } else {
            unreachable!();
        };
        Self { focus_ref, state }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> ShouldRender {
        log::trace!("Message: {:?}", msg);
        match msg {
            Msg::Submit(value) => {
                self.state.edit_tags = value;
                self.update_tags();
                true
            }
            Msg::Focus => {
                if let Some(input) = self.focus_ref.cast::<HtmlInputElement>() {
                    input.focus().unwrap();
                }
                true
            }
            Msg::ToggleDrivingSide => {
                self.state.locale.driving_side = self.state.locale.driving_side.opposite();
                self.update_tags();
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let edit = move |input: HtmlInputElement| Msg::Submit(input.value());

        let onmouseover = ctx.link().callback(|_| Msg::Focus);

        let onblur = ctx
            .link()
            .callback(move |e: FocusEvent| edit(e.target_unchecked_into()));

        let onkeypress = ctx.link().batch_callback(move |e: KeyboardEvent| {
            (e.key() == "Enter").then(|| edit(e.target_unchecked_into()))
        });

        let onchange = ctx.link().callback(|_e: Event| Msg::ToggleDrivingSide);

        html! {
            <div>
                <h1>{"osm2lanes"}</h1>
                <section class="row">
                    <div class="row-item">
                        <p>{"↑↓ LHT"}</p>
                    </div>
                    <label class="row-item switch">
                        <input
                            type="checkbox"
                            checked={self.state.locale.driving_side == DrivingSide::Right}
                            {onchange}
                        />
                        <span class="slider"></span>
                    </label>
                    <div class="row-item">
                        <p>{"RHT ↓↑"}</p>
                    </div>
                </section>
                <section class="row">
                    <div class="row-item">
                        <textarea
                            rows={(self.state.edit_tags.lines().count() + 1).to_string()}
                            cols="48"
                            ref={self.focus_ref.clone()}
                            value={self.state.edit_tags.clone()}
                            {onmouseover}
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
                            </section>
                        }
                    } else {
                        html!{}
                    }
                }
                <hr/>
                <section>
                    <div class="lanes">
                        {
                            for self.state.lanes.iter().map(|lane| self.view_lane_type(lane))
                        }
                    </div>
                    <div class="lanes">
                        {
                            for self.state.lanes.iter().map(|lane| self.view_lane_direction(lane))
                        }
                    </div>
                </section>
                <hr/>
                <canvas id="canvas" width="960px" height="480px"></canvas>
            </div>
        }
    }

    fn rendered(&mut self, _ctx: &Context<Self>, _first_render: bool) {
        self.draw_canvas()
    }
}

impl App {
    fn calculate(
        value: &str,
        locale: &Locale,
    ) -> Result<(Lanes, Tags), Result<(Lanes, String), String>> {
        log::trace!("Calculate: {}", value);
        match Tags::from_str(value) {
            Ok(tags) => match tags_to_lanes(&tags, locale, &TagsToLanesConfig::default()) {
                Ok(lanes) => {
                    match lanes_to_tags(&lanes.lanes, locale, &LanesToTagsConfig::default()) {
                        Ok(tags) => Ok((lanes, tags)),
                        Err(e) => Err(Ok((lanes, e.to_string()))),
                    }
                }
                Err(e) => Err(Err(e.to_string())),
            },
            Err(_) => Err(Err("parsing tags failed".to_owned())),
        }
    }

    fn update_tags(&mut self) {
        let value = &self.state.edit_tags;
        log::trace!("Update Tags: {}", value);
        let calculate = Self::calculate(value, &self.state.locale);
        log::trace!("Update: {:?}", calculate);
        match calculate {
            Ok((Lanes { lanes, warnings }, norm_tags)) => {
                self.state.lanes = lanes;
                self.state.normalized_tags = Some(norm_tags.to_string());
                if warnings.is_empty() {
                    self.state.message = None;
                } else {
                    self.state.message = Some(warnings.to_string());
                }
            }
            Err(Ok((Lanes { lanes, warnings }, norm_err))) => {
                self.state.lanes = lanes;
                self.state.normalized_tags = None;
                if warnings.is_empty() {
                    self.state.message = Some(format!("Normalisation Error: {}", norm_err));
                } else {
                    self.state.message =
                        Some(format!("{}\nNormalisation Error: {}", warnings, norm_err));
                }
            }
            Err(Err(lanes_err)) => {
                self.state.lanes = Vec::new();
                self.state.normalized_tags = None;
                self.state.message = Some(format!("Conversion Error: {}", lanes_err));
            }
        }
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
    fn draw_canvas(&self) {
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

        draw::lanes(&mut rc, (canvas_width, canvas_height), &self.state.lanes).unwrap();
    }
}

fn main() {
    console_log::init_with_level(log::Level::Trace).expect("logging failed");
    log::trace!("Initializing yew...");
    yew::start_app::<App>();
}
