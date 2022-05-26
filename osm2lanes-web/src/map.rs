use geo::{LineString, Point};
use leaflet::{LatLng, Map, MouseEvent, Path, Polyline, TileLayer};
use osm2lanes::locale::Locale;
use osm2lanes::overpass::{get_nearby, Error as OverpassError};
use osm_tags::Tags;
use serde::Serialize;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Element, HtmlElement};
use yew::prelude::*;
use yew::Html;

use crate::Msg as WebMsg;

#[allow(clippy::large_enum_variant)]
pub(crate) enum Msg {
    MapClick(Point<f64>),
    MapUpdate(String, Tags, Locale, LineString<f64>),
    Error(String),
}

#[derive(Properties, Clone, PartialEq)]
pub(crate) struct Props {
    pub(crate) callback_msg: Callback<WebMsg>,
}

pub(crate) struct MapComponent {
    container: HtmlElement,
    map: Map,
    point: Point<f64>,
    path: Option<Path>,
    _map_click_closure: Closure<dyn Fn(MouseEvent)>,
    is_searching: bool,
}

impl MapComponent {
    const MAP_ID: &'static str = "map";

    fn render_map(&self) -> Html {
        // creating the container here doesn't work
        // modifying the container here breaks things
        // regardless, it is unclear if this clone is OK
        Html::VRef(self.container.clone().into())
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MapOptions {
    scroll_wheel_zoom: bool,
    // https://github.com/elmarquis/Leaflet.GestureHandling
    gesture_handling: bool,
}
const MAP_OPTIONS: MapOptions = MapOptions {
    scroll_wheel_zoom: false,
    gesture_handling: true,
};

impl Component for MapComponent {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        let container: Element = gloo_utils::document().create_element("div").unwrap();
        container.set_id(Self::MAP_ID);
        let container: HtmlElement = container.dyn_into().unwrap();

        let map = Map::new_with_element(&container, &JsValue::from_serde(&MAP_OPTIONS).unwrap());

        let map_click_callback = ctx.link().callback(Msg::MapClick);
        let map_click_closure = Closure::<dyn Fn(MouseEvent)>::wrap(Box::new(move |click_event| {
            let lat_lng = click_event.latlng();
            map_click_callback.emit(Point::new(lat_lng.lat(), lat_lng.lng()));
        }));
        map.on("click", map_click_closure.as_ref());

        Self {
            container,
            map,
            point: Point::new(40.0_f64, 10.0_f64),
            path: None,
            // to avoid dropping the closure and invalidating the callback
            _map_click_closure: map_click_closure,
            is_searching: false,
        }
    }

    fn rendered(&mut self, _ctx: &Context<Self>, first_render: bool) {
        if first_render {
            self.map
                .setView(&LatLng::new(self.point.x(), self.point.y()), 4.0);
            log::debug!("add osm tile layer");
            add_tile_layer(&self.map);
        }
    }

    #[allow(clippy::todo)]
    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        self.is_searching = false;
        match msg {
            Msg::MapClick(point) => {
                if self.is_searching {
                    log::debug!("map search click ignored, search ongoing");
                    false
                } else {
                    log::debug!("map search click");
                    ctx.link().send_future(async move {
                        match get_nearby(point).await {
                            Ok((id, tags, geometry, locale)) => {
                                Msg::MapUpdate(id.to_string(), tags, locale, geometry)
                            },
                            Err(OverpassError::Empty) => Msg::Error(String::from("no ways found")),
                            Err(e) => Msg::Error(e.to_string()),
                        }
                    });
                    self.is_searching = true;
                    true
                }
            },
            Msg::MapUpdate(id, tags, locale, geometry) => {
                log::debug!("map search complete");
                if let Some(path) = self.path.take() {
                    path.remove();
                }

                let polyline = Polyline::new(
                    geometry
                        .into_iter()
                        .map(|coordinate| LatLng::new(coordinate.x, coordinate.y).into())
                        .collect(),
                );
                let bounds = polyline.getBounds();
                let path = Path::from(polyline);
                path.addTo(&self.map);
                self.path = Some(path);
                self.map.fitBounds(&bounds);
                ctx.props()
                    .callback_msg
                    .emit(WebMsg::TagsLocaleSet { id, tags, locale });
                true
            },
            Msg::Error(e) => {
                log::debug!("map error: {e}");
                ctx.props().callback_msg.emit(WebMsg::Error(e));
                true
            },
        }
    }

    fn changed(&mut self, _ctx: &Context<Self>) -> bool {
        false
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        log::debug!("map redraw");
        html! {
            <section class="map">
                {self.render_map()}
            </section>
        }
    }
}

fn add_tile_layer(map: &Map) {
    TileLayer::new(
        "https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png",
        &JsValue::NULL,
    )
    .addTo(map);
}
