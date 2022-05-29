use gloo_worker::{HandlerId, Worker, WorkerScope};
use osm2lanes::test::{get_tests, TestCase};
use serde::{Deserialize, Serialize};

pub(crate) const NAME: &str = "worker.js";

pub struct ExampleLoader {
    link: WorkerScope<Self>,
}

#[derive(Serialize, Deserialize)]
pub struct ExampleLoaderOutput(pub Vec<TestCase>);

impl Worker for ExampleLoader {
    type Message = ();
    type Input = ();
    type Output = ExampleLoaderOutput;

    fn create(link: WorkerScope<Self>) -> Self {
        Self { link }
    }

    fn update(&mut self, _msg: Self::Message) {
        // no messaging
    }

    fn received(&mut self, _msg: Self::Input, id: HandlerId) {
        let tests = get_tests();
        let examples = tests
            .into_iter()
            .filter(|t| t.example().is_some())
            .collect();
        self.link.respond(id, ExampleLoaderOutput(examples));
    }
}
