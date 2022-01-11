use std::str::FromStr;

use piet_web::WebRenderContext;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use web_sys::{window, HtmlCanvasElement, HtmlInputElement};
use yew::prelude::*;

mod draw;

use osm2lanes::{get_lane_specs_ltr, Config, DrivingSide, LanePrintable, LaneSpec, Tags};

// Use `wee_alloc` as the global allocator.
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

type ShouldRender = bool;

#[derive(Debug, Serialize, Deserialize)]
pub struct State {
    pub edit_value: String,
    pub lanes: Vec<LaneSpec>,
}

pub enum Msg {
    Submit(String),
    Focus,
}

pub struct App {
    focus_ref: NodeRef,
    state: State,
}

const CFG: Config = Config {
    driving_side: DrivingSide::Right,
    inferred_sidewalks: true,
};

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        let focus_ref = NodeRef::default();
        let edit_value = "highway=secondary\ncycleway:right=track\nlanes=6\nlanes:backward=2\nlanes:taxi:backward=1\nlanes:psv=1\nsidewalk=right".to_owned();
        let lanes = get_lane_specs_ltr(Tags::from_str(&edit_value).unwrap(), &CFG);
        let state = State { edit_value, lanes };
        Self { focus_ref, state }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::Submit(value) => {
                log::trace!("Submit: {}", value);
                if let Ok(tags) = Tags::from_str(&value) {
                    self.state.lanes = get_lane_specs_ltr(tags, &CFG);
                }
                self.state.edit_value = value;
                true
            }
            Msg::Focus => {
                if let Some(input) = self.focus_ref.cast::<HtmlInputElement>() {
                    input.focus().unwrap();
                }
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

        html! {
            <div>
                <h1>{"osm2lanes"}</h1>
                <textarea
                    class="edit"
                    type="text"
                    rows={(self.state.edit_value.lines().count() + 1).to_string()}
                    cols="48"
                    ref={self.focus_ref.clone()}
                    value={self.state.edit_value.clone()}
                    {onmouseover}
                    {onblur}
                    {onkeypress}
                />
                <hr/>
                <section>
                    <div class="row">
                        {
                            for self.state.lanes.iter().map(|lane| self.view_lane_type(lane))
                        }
                    </div>
                    <div class="row">
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
    fn view_lane_type(&self, lane: &LaneSpec) -> Html {
        html! {
            <div class="row-item"><span>{lane.lane_type.as_utf8()}</span></div>
        }
    }
    fn view_lane_direction(&self, lane: &LaneSpec) -> Html {
        html! {
            <div class="row-item"><span>{lane.direction.as_utf8()}</span></div>
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
