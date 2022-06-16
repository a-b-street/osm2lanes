use gloo_worker::{Codec, HandlerId, Worker, WorkerScope};
use osm2lanes::test::{get_tests, TestCase};
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;

pub struct SerdeWasmBindgen;

impl Codec for SerdeWasmBindgen {
    fn encode<I>(input: I) -> JsValue
    where
        I: Serialize,
    {
        let data = serde_json::to_string(&input).expect("can't serialize worker message");
        log::trace!("message in: {data}");
        JsValue::from_str(&data)
    }

    fn decode<O>(input: JsValue) -> O
    where
        O: for<'de> Deserialize<'de>,
    {
        let data = input.as_string().expect("JsValue string");
        log::trace!("message out: {data}");
        serde_json::from_str(&data).expect("can't deserialize worker message")
    }
}

pub(crate) const NAME: &str = "worker.js";

pub struct ExampleLoader;

#[derive(Serialize, Deserialize)]
pub struct ExampleLoaderOutput(pub Vec<TestCase>);

impl Worker for ExampleLoader {
    type Message = ();
    type Input = ();
    type Output = ExampleLoaderOutput;

    fn create(_scope: &WorkerScope<Self>) -> Self {
        Self
    }

    fn update(&mut self, _scope: &WorkerScope<Self>, _msg: Self::Message) {
        // no messaging
    }

    fn received(&mut self, scope: &WorkerScope<Self>, _msg: Self::Input, id: HandlerId) {
        let tests = get_tests();
        let examples = tests
            .into_iter()
            .filter(|t| t.example().is_some())
            .collect();
        scope.respond(id, ExampleLoaderOutput(examples));
    }
}
