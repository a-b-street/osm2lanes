use std::cell::RefCell;
use std::rc::Rc;

use osm2lanes::locale::{Country, DrivingSide};
use web_sys::{Event, FocusEvent, HtmlInputElement, HtmlSelectElement, KeyboardEvent, MouseEvent};
use yew::{html, Callback, Component, Context, Html, NodeRef, Properties, TargetCast};

use crate::{Msg, State};

#[derive(Properties, PartialEq)]
pub struct Props {
    pub callback_msg: Callback<Msg>,
    pub state: Rc<RefCell<State>>,
}

#[derive(Default)]
pub struct Control {
    textarea_input_ref: NodeRef,
    textarea_output_ref: NodeRef,
}

impl Component for Control {
    type Properties = Props;
    type Message = Msg;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            ..Default::default()
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        ctx.props().callback_msg.emit(msg);
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

        let driving_side_onchange = ctx.link().callback(|_e: Event| Msg::ToggleDrivingSide);

        let country_onchange = ctx.link().callback(|e: Event| {
            let selected: String = e.target_unchecked_into::<HtmlSelectElement>().value();
            let selected = Country::from_alpha2(selected);
            Msg::CountrySet(selected)
        });

        let way_id_onclick = ctx.link().callback(|_e: MouseEvent| Msg::WayFetch);

        let textarea_input_onblur = ctx.link().callback(|input: FocusEvent| {
            Msg::TagsSet(input.target_unchecked_into::<HtmlInputElement>().value())
        });
        let textarea_input_onkeypress = ctx.link().callback(|input: KeyboardEvent| {
            Msg::TagsSet(input.target_unchecked_into::<HtmlInputElement>().value())
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
                    <input class="row-item" type="text" id="way" name="way" size="12" ref={state.way_ref.clone()}/>
                    <button class="row-item" onclick={way_id_onclick}>
                        {"Fetch"}
                    </button>
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
}
