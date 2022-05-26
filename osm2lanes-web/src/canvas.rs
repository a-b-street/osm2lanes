use std::cell::RefCell;
use std::rc::Rc;

use osm2lanes::locale::Locale;
use osm2lanes::road::Road;
use piet::Error as PietError;
use piet_web::WebRenderContext;
use wasm_bindgen::JsCast;
use web_sys::{window, HtmlCanvasElement};
use yew::{html, Callback, Component, Context, Properties};

use crate::{draw, State};

#[derive(Debug)]
pub(crate) enum RenderError {
    Piet(PietError),
    _UnknownLane,
    _UnknownSeparator,
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
            Self::_UnknownLane => write!(f, "error rendering unknown lane"),
            Self::_UnknownSeparator => write!(f, "error rendering unknown separator"),
            Self::Piet(p) => write!(f, "{}", p),
        }
    }
}

pub(crate) enum Msg {}

#[derive(Properties, PartialEq)]
pub(crate) struct Props {
    pub(crate) callback_error: Callback<String>,
    pub(crate) state: Rc<RefCell<State>>,
}

// TODO: make this a functional component
pub(crate) struct Canvas;

impl Component for Canvas {
    type Message = Msg;
    type Properties = Props;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {}
    }

    fn view(&self, _ctx: &Context<Self>) -> yew::Html {
        html! {
            <canvas id="canvas" width="960px" height="480px"></canvas>
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, _first_render: bool) {
        if let Err(e) = Self::draw_canvas(
            ctx.props().state.borrow().road.as_ref(),
            &ctx.props().state.borrow().locale,
        ) {
            ctx.props().callback_error.emit(format!("Error: {}", e));
        }
    }
}

impl Canvas {
    #[allow(clippy::as_conversions)]
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_sign_loss)]
    fn draw_canvas(road: Option<&Road>, locale: &Locale) -> Result<(), RenderError> {
        if let Some(road) = road {
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
            let canvas_width = (f64::from(canvas.offset_width()) * dpr) as u32;
            let canvas_height = (f64::from(canvas.offset_height()) * dpr) as u32;
            canvas.set_width(canvas_width);
            canvas.set_height(canvas_height);
            context.scale(dpr, dpr).unwrap();
            let mut rc = WebRenderContext::new(context, window);

            draw::lanes(&mut rc, (canvas_width, canvas_height), road, locale)?;
        }
        Ok(())
    }
}
