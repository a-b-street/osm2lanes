use osm2lanes::test::{get_tests, TestCase};
use serde::{Deserialize, Serialize};
use yew_agent::{Agent, AgentLink, HandlerId, Public};

pub struct ExampleLoader {
    link: AgentLink<Self>,
}

#[derive(Serialize, Deserialize)]
pub struct ExampleLoaderOutput(pub Vec<TestCase>);

impl Agent for ExampleLoader {
    type Input = ();
    type Message = ();
    type Output = ExampleLoaderOutput;
    type Reach = Public<Self>;

    fn create(link: AgentLink<Self>) -> Self {
        Self { link }
    }

    fn update(&mut self, _msg: Self::Message) {
        // no messaging
    }

    fn handle_input(&mut self, _msg: Self::Input, id: HandlerId) {
        let tests = get_tests();
        let examples = tests
            .into_iter()
            .filter(|t| t.example().is_some())
            .collect();
        self.link.respond(id, ExampleLoaderOutput(examples));
    }

    fn name_of_resource() -> &'static str {
        "worker.js"
    }
}
