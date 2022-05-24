use osm2lanes_web::App;

fn main() {
    console_log::init_with_level(log::Level::Debug).expect("logging failed");
    log::trace!("Initializing yew...");
    yew::start_app::<App>();
}
