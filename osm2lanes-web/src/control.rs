use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gloo_timers::callback::Timeout;
use osm2lanes::locale::{Country, DrivingSide, Locale};
use osm2lanes::test::{get_tests, TestCase};
use web_sys::{Event, FocusEvent, HtmlInputElement, HtmlSelectElement, KeyboardEvent, MouseEvent};
use yew::{html, Callback, Component, Context, Html, NodeRef, Properties, TargetCast};

use crate::{Msg as AppMsg, State};

pub enum Msg {
    Up(Box<AppMsg>),
    FirstLazy,
    Example(String),
}

impl From<AppMsg> for Msg {
    fn from(msg: AppMsg) -> Self {
        Self::Up(Box::new(msg))
    }
}

#[derive(Properties, PartialEq)]
pub struct Props {
    pub callback_msg: Callback<AppMsg>,
    pub state: Rc<RefCell<State>>,
}

#[derive(Default)]
pub struct Control {
    textarea_input_ref: NodeRef,
    textarea_output_ref: NodeRef,
    example: Option<String>,
    examples: Option<BTreeMap<String, TestCase>>,
}

impl Component for Control {
    type Properties = Props;
    type Message = Msg;

    fn create(_ctx: &Context<Self>) -> Self {
        Self::default()
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Up(msg) => ctx.props().callback_msg.emit(*msg),
            Msg::FirstLazy => {
                let tests = get_tests();
                let examples: BTreeMap<_, _> = tests
                    .into_iter()
                    .filter_map(|t| {
                        let example_name = t.example().map(|e| e.to_owned());
                        example_name.map(|e| (e, t))
                    })
                    .collect();
                let example = examples.iter().next().unwrap().0.to_owned();
                self.examples = Some(examples);
                ctx.link().send_message(Msg::Example(example));
            },
            Msg::Example(example) => {
                let test_case = self
                    .examples
                    .as_ref()
                    .unwrap()
                    .get(&example)
                    .unwrap()
                    .clone();
                self.example = Some(example);
                ctx.link().send_message(Msg::from(AppMsg::TagsLocaleSet {
                    id: test_case.way_id.unwrap().to_string(),
                    tags: test_case.tags,
                    locale: Locale::builder()
                        .driving_side(test_case.driving_side)
                        .iso_3166_option(test_case.iso_3166_2.as_deref())
                        .build(),
                }))
            },
        }
        // we let the parent do this
        false
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let state = ctx.props().state.borrow();

        let countries = {
            let mut countries: Vec<(&str, bool)> = Country::get_countries()
                .into_iter()
                .map(|country| {
                    (
                        country.alpha2,
                        state
                            .locale
                            .country
                            .as_ref()
                            .map_or(false, |locale| locale == &country),
                    )
                })
                .collect();
            countries.sort_unstable_by_key(|c| c.0);
            log::trace!("countries {:?}", countries);
            countries
        };
        let country_onchange = ctx.link().callback(|e: Event| {
            let selected: String = e.target_unchecked_into::<HtmlSelectElement>().value();
            let selected = Country::from_alpha2(selected);
            Msg::from(AppMsg::CountrySet(selected))
        });

        let driving_side_onchange = ctx
            .link()
            .callback(|_e: Event| Msg::from(AppMsg::ToggleDrivingSide));

        let way_id: String = state
            .id
            .as_ref()
            .cloned()
            .unwrap_or_else(|| String::from(""));
        let way_id_onclick = ctx
            .link()
            .callback(|_e: MouseEvent| Msg::from(AppMsg::WayFetch));

        let textarea_input_onblur = ctx.link().callback(|input: FocusEvent| {
            Msg::from(AppMsg::TagsSet(
                input.target_unchecked_into::<HtmlInputElement>().value(),
            ))
        });
        let textarea_input_onkeypress = ctx.link().callback(|input: KeyboardEvent| {
            Msg::from(AppMsg::TagsSet(
                input.target_unchecked_into::<HtmlInputElement>().value(),
            ))
        });

        let example_onchange = ctx.link().callback(move |e: Event| {
            let example: String = e.target_unchecked_into::<HtmlSelectElement>().value();
            Msg::Example(example)
        });

        html! {
            <>
                <section class="row">
                    <button class="row-item">
                        {"Calculate"}
                    </button>
                    <hr/>
                    <p class="row-item">
                        {"↑↓ LHT"}
                    </p>
                    <label class="row-item switch">
                        <input
                            type="checkbox"
                            checked={state.locale.driving_side == DrivingSide::Right}
                            onchange={driving_side_onchange}
                        />
                        <span class="slider"></span>
                    </label>
                    <p class="row-item">
                        {"RHT ↓↑"}
                    </p>
                    <hr/>
                    <select onchange={country_onchange} class={
                        if state.locale.iso_3166_2_subdivision.is_some() {"suffixed"} else {""}
                    }>
                    {
                        for countries.into_iter().map(|(country, selected)| html!{
                            <option value={country} selected={selected}>
                                {country}
                            </option>
                        })
                    }
                    </select>
                    {
                        if let Some(code) = &state.locale.iso_3166_2_subdivision {
                            html!{
                                <p class="row-item prefixed">
                                    {"-"}{code}
                                </p>
                            }
                        } else {
                            html!{}
                        }
                    }
                    <hr/>
                    <label class="row-item" for="way">{"OSM Way ID"}</label>
                    <input class="row-item" type="text" id="way" name="way" size="12"
                        ref={state.way_ref.clone()}
                        value={way_id}/>
                    <button class="row-item" onclick={way_id_onclick}>
                        {"Fetch"}
                    </button>
                    <hr/>
                    <label class="row-item" for="example">{"Examples"}</label>
                    <select onchange={example_onchange} id="examples">
                    {
                        if let Some(examples) = &self.examples {
                            html!{
                                <>{
                                    for examples.keys().map(|e| html!{
                                        <option value={e.clone()} selected={Some(e) == self.example.as_ref()}>{e}</option>
                                    })
                                }</>
                            }
                        } else {
                            html!{
                                <option>{"LOADING..."}</option>
                            }
                        }
                    }
                    </select>
                </section>
                <section class="row">
                    <div class="row-item">
                        <textarea
                            rows={(state.edit_tags.lines().count() + 1).to_string()}
                            cols="48"
                            ref={self.textarea_input_ref.clone()}
                            value={state.edit_tags.clone()}
                            onblur={textarea_input_onblur}
                            onkeypress={textarea_input_onkeypress}
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
                            disabled={state.normalized_tags.is_none()}
                            rows={
                                if let Some(tags) = &state.normalized_tags {
                                    (tags.lines().count() + 1).to_string()
                                } else {
                                    "1".to_owned()
                                }
                            }
                            cols="48"
                            ref={self.textarea_output_ref.clone()}
                            value={
                                if let Some(tags) = &state.normalized_tags {
                                    tags.clone()
                                } else {
                                    "".to_owned()
                                }
                            }
                            spellcheck={"false"}
                        />
                    </div>
                </section>
            </>
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            let handle = {
                let link = ctx.link().clone();
                Timeout::new(1, move || link.send_message(Msg::FirstLazy))
            };
            handle.forget();
        }
    }
}
