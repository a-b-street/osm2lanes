use gloo_worker::Registrable;
use osm2lanes_web::agent::{ExampleLoader, JsonCodec};

fn main() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Debug).expect("logging failed");
    log::trace!("Initializing worker...");
    ExampleLoader::registrar()
        .encoding::<JsonCodec>()
        .register();
}
