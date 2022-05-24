use osm2lanes_web::agent::ExampleLoader;
use yew_agent::Threaded;

fn main() {
    console_error_panic_hook::set_once();
    ExampleLoader::register();
}
