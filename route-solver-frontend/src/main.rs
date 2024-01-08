use std::{
    cell::RefCell,
    error::Error,
    fmt::{self, Debug, Display, Formatter},
    rc::Rc, ops::Deref,
};

use route_solver_shared::queries::*;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::{JsFuture, future_to_promise, spawn_local};
use web_sys::{HtmlElement, HtmlInputElement, Request, RequestInit, RequestMode, Response, console::log};
use yew::{prelude::*, virtual_dom::Key};
use serde_json::*;

#[derive(Properties, PartialEq)]
struct TextBoxProps {
    text: String,
    type_name: String,
    text_update_handler: Callback<String, ()>
}

#[derive(Properties, PartialEq)]
struct ButtonProps {
    text: String,
    on_click: Callback<()>,
}

#[derive(Properties, PartialEq)]
struct ModalProps {
    id: String,
    main_text: String,
    internal_html: Html,
}

#[derive(Properties, PartialEq)]
struct ModalTriggerProps {
    id: String,
    text: String,
}

#[derive(Properties, PartialEq)]
struct DropDownProps {
    text: String,
    opts: Vec<String>,
    node_ref: NodeRef,
}

struct ItineraryListItems {
    items: Vec<ItineraryListItemProps>,
}

struct TextBox {
    input_value: String
}

enum TextMsg {
    InputChanged(InputEvent),
}

impl Component for TextBox {
    type Message = TextMsg;
    type Properties = TextBoxProps;

    fn create(ctx: &Context<Self>) -> Self {
        TextBox { input_value: "".to_string() }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            TextMsg::InputChanged(e) => {
                let target = e.target();
                let input = target.and_then(|t| t.dyn_into::<HtmlInputElement>().ok());

                if let Some(input) = input {
                    self.input_value = input.value();
                    ctx.props().text_update_handler.emit(self.input_value.clone());
                }
            }
        }
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <input type={"text"} class={"form-control"} placeholder={ctx.props().text.clone()} aria-label={ctx.props().text.clone()} type={ctx.props().type_name.clone()} value={self.input_value.clone()} oninput={ctx.link().callback(|e: InputEvent| TextMsg::InputChanged(e))} />
        }
    }
}

#[function_component(Button)]
fn button(ButtonProps { text, on_click }: &ButtonProps) -> Html {
    let on_click_fn = {
        let on_click = on_click.clone();
        Callback::from(move |_| on_click.emit(()))
    };

    html! {
        <button type={"button"} onclick={on_click_fn} class={"btn btn-primary my-2"}>{ text.clone() }</button>
    }
}

#[function_component(ModalTriggerButton)]
fn modal_trigger(ModalTriggerProps { id, text }: &ModalTriggerProps) -> Html {
    html! {
        <button type={"button"} data-bs-toggle="modal" data-bs-target={ String::from("#") + id } class={"btn btn-primary"}>{ text.clone() }</button>
    }
}

#[function_component(Modal)]
fn modal(
    ModalProps {
        id,
        main_text,
        internal_html,
    }: &ModalProps,
) -> Html {
    html! {
      <div id={ id.clone() } class="modal fade" tabindex="-1">
        <div class="modal-dialog">
          <div class="modal-content">
            <div class="modal-header">
              <h5 class="modal-title">{ main_text }</h5>
              <button type="button" class="btn-close" data-bs-dismiss="modal" aria-label="Close"></button>
            </div>
            <div class="modal-body">
                { internal_html.clone() }
            </div>
            <div class="modal-footer">
              <button type="button" class="btn btn-secondary" data-bs-dismiss="modal">{ String::from("Close") }</button>
            </div>
          </div>
        </div>
      </div>
    }
}

#[function_component(CloseButton)]
fn close_button(ButtonProps { text, on_click }: &ButtonProps) -> Html {
    let on_click_fn = {
        let on_click = on_click.clone();
        Callback::from(move |_| on_click.emit(()))
    };

    html! {
        <button type={"button"} onclick={on_click_fn} class={"btn-close my-2"} aria-label={text.clone()}></button>
    }
}

#[derive(PartialEq, Clone, Properties)]
struct FlyInProps {
    fly_in_update_handler: Callback<(String, String), ()>
}

#[function_component(FlyInComponent)]
fn fly_in(FlyInProps { fly_in_update_handler }: &FlyInProps) -> Html {
    let mut curr_vals = use_state(|| ["".to_string(), "".to_string()]);

    let box_callback_gen = |id: usize| {
        let curr_vals = curr_vals.clone();
        let fly_in_cb = fly_in_update_handler.clone();
        Callback::from(move |new_val: String| {
            let mut new_vals = curr_vals.deref().clone();
            new_vals[id] = new_val;
            fly_in_cb.clone().emit((new_vals[0].clone(), new_vals[1].clone()));
            curr_vals.set(new_vals);
        })
    };

    html! {
        <div class="d-inline-flex">
            <div class="input-group flex-nowrap pe-2">
                <span class="input-group-text" id="addon-wrapping">{ "Start" }</span>
                <TextBox text="Start" type_name="date" text_update_handler={box_callback_gen(0)} />
            </div>
            <div class="input-group flex-nowrap pe-2">
                <span class="input-group-text" id="addon-wrapping">{ "End" }</span>
                <TextBox text="End" type_name="date" text_update_handler={box_callback_gen(1)} />
            </div>
        </div>
    }
}

#[derive(PartialEq, Clone, Properties)]
struct ListItemProps {
    text: String,
    children: Children,
}

#[function_component(ListItem)]
fn list_item(ListItemProps { text, children }: &ListItemProps) -> Html {
    let open = use_state(|| false);
    let onopen = {
        let open = open.clone();
        Callback::from(move |_| open.set(true))
    };
    let onclose = {
        let open = open.clone();
        Callback::from(move |_| open.set(false))
    };

    html! {
        if !*open {
            <div class="col-md-auto">
                <Button text={text.clone()} on_click={onopen} />
            </div>
        } else {
            <div class="col-md-auto my-2">
                { children.clone() }
            </div>
            <div class="col-md-auto my-2">
                <CloseButton text="Close" on_click={onclose} />
            </div>
        }
    }
}

#[function_component(DropDown)]
fn dropdown(
    DropDownProps {
        text,
        opts,
        node_ref,
    }: &DropDownProps,
) -> Html {
    let opts_html = opts.iter().map(|opt| {
        html! {
            <li><a class={"dropdown-item"} href={"#"}>{ opt.clone() }</a></li>
        }
    });

    html! {
        <div class={"dropdown"} ref={ node_ref.clone() } >
          <button class={"btn btn-secondary dropdown-toggle"} type={"button"} data-bs-toggle={"dropdown"} aria-expanded={"false"}>
            { text.clone() }
          </button>
          <ul class={"dropdown-menu"}>
            { for opts_html }
          </ul>
        </div>
    }
}

#[derive(PartialEq, Clone, Default)]
struct ListItemVals {
    airport: String,
    start_dates: (String, String),
    end_dates: (String, String),
    temp_constraints: (String, String),
}

#[derive(Properties, PartialEq, Clone)]
struct ItineraryListItemProps {
    id: usize,
    remove_handler: Callback<usize>,
    vals_updated_handler: Callback<ListItemVals, ()>
}

struct ItineraryRow {
    list_item_vals: ListItemVals
}

enum ItineraryRowMsg {
    FlyInUpdated(usize, String, String),
    AirportUpdated(String)
}

impl Component for ItineraryRow {
    type Properties = ItineraryListItemProps;
    type Message = ItineraryRowMsg;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            list_item_vals: ListItemVals::default()
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            ItineraryRowMsg::FlyInUpdated(idx, start, end) => {
                let mut fly_in_ref = match idx {
                    0 => &mut self.list_item_vals.start_dates,
                    1 => &mut self.list_item_vals.end_dates,
                    _ => panic!("Bad messaging in ItineraryRow element")
                };
                *fly_in_ref = (start, end);
            },
            ItineraryRowMsg::AirportUpdated(text) => self.list_item_vals.airport = text
        }
        ctx.props().vals_updated_handler.emit(self.list_item_vals.clone());
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let remove_handler_passthrough = {
            let remove_handler = ctx.props().remove_handler.clone();
            let id = ctx.props().id;
            Callback::from(move |_| {
                remove_handler.emit(id);
            })
        };

        html! {
            <div id={ format!("itin-row-{}", ctx.props().id) } class={"row my-1 justify-content-start bg-body-secondary p-1 rounded-3"}>
                <div class="container p-2">
                    <div class="row justify-content-start">
                        <div class={"col-md-auto"}>
                            <TextBox text={ "Airport Code" } type_name={"text"} text_update_handler={ ctx.link().callback(|input: String| ItineraryRowMsg::AirportUpdated(input)) } />
                        </div>
                        <div class={"col-md-auto"}>
                            <CloseButton text="Close" on_click={remove_handler_passthrough} />
                        </div>
                    </div>
                    <div class="row justify-content-start">
                        <ListItem text={ "Add fly in dates" }>
                            <FlyInComponent fly_in_update_handler={ctx.link().callback(|input: (String, String)| ItineraryRowMsg::FlyInUpdated(0, input.0, input.1))} />
                        </ListItem>
                    </div>
                    <div class="row justify-content-start">
                        <ListItem text={ "Add fly out dates" }>
                            <FlyInComponent fly_in_update_handler={ctx.link().callback(|input: (String, String)| ItineraryRowMsg::FlyInUpdated(1, input.0, input.1))} />
                        </ListItem>
                    </div>
                    <div class="row justify-content-start">
                        <ListItem text={ "Add other constraints" }>
                            <p>{"Yay constraints"}</p>
                        </ListItem>
                    </div>
                </div>
            </div>
        }
    }
}

struct ItineraryList {
    html_list: Vec<(Html, bool)>,
    curr_count: usize,
    list_item_vals: Vec<ListItemVals>,
}

enum ItineraryListMessage {
    AddChild,
    RemoveChild(usize),
    ChildUpdate(usize, ListItemVals),
    SendPost
}

impl ItineraryList {
    fn get_formatted_text(&self) -> String {
        self.html_list
            .iter()
            .enumerate()
            .filter(|(_, (_, on))| *on)
            .map(|(idx, (_, _))| {
                let airport_code = self.list_item_vals[idx].airport.clone();
                let start_dates = self.list_item_vals[idx].start_dates.clone();
                let end_dates = self.list_item_vals[idx].end_dates.clone();
                let temp_dates = self.list_item_vals[idx].temp_constraints.clone();

                format!(
                    "Airport {} start dates {} {} end dates {} {} temp dates {} {}",
                    airport_code,
                    start_dates.0,
                    start_dates.1,
                    end_dates.0,
                    end_dates.1,
                    temp_dates.0,
                    temp_dates.1
                )
            })
            .reduce(|acc, e| format!("{}\n{}", acc, e))
            .unwrap()
    }
}

impl Component for ItineraryList {
    type Properties = ();
    type Message = ItineraryListMessage;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            html_list: vec![],
            curr_count: 0,
            list_item_vals: vec![]
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        use web_sys::console;

        match msg {
            ItineraryListMessage::AddChild => {
                let link = ctx.link();
                let count = self.curr_count.clone();
                self.list_item_vals
                    .push(ListItemVals::default());
                self.html_list.push((html! { <ItineraryRow id={ self.curr_count.clone() } key={ self.curr_count.clone() } vals_updated_handler={ ctx.link().callback(move |vals: ListItemVals| ItineraryListMessage::ChildUpdate( count, vals)) } remove_handler={ link.callback(move |_| ItineraryListMessage::RemoveChild( count )) } /> }, true));
                self.curr_count += 1;
            }
            ItineraryListMessage::ChildUpdate(idx, vals) => {
                self.list_item_vals[idx] = vals;
            }
            ItineraryListMessage::SendPost => {
                let text = self.get_formatted_text();

                console::log_1(&("Posting: ".to_string() + &text).into());

                let resp_runner = async {
                    let query: JsValue = serde_json::to_string(&EchoQuery { input: text }).unwrap().into();

                    let mut opts = RequestInit::new();
                    opts.method("POST");
                    opts.body(Some(&query));
                    let request = Request::new_with_str_and_init("echo", &opts).unwrap();
                    let _ = request.headers().set("content-type", "application/json");
                    let window = web_sys::window().unwrap();
                    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await.unwrap();
                    let val = resp_value.as_string();

                    val.and_then(|r| Some(console::log_1(&("Response ".to_string() + &r).into())));
                    // Ok(JsValue::from_bddool(true))
                };

                // let js_promise = future_to_promise(resp_runner);
                spawn_local(resp_runner);
            }
            ItineraryListMessage::RemoveChild(idx) => self.html_list.iter_mut().for_each(|x| {
                if x.0.key().unwrap().eq(&Key::from(idx)) {
                    *x = (x.0.clone(), false)
                }
            })
        }

        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();

        let rows = self
            .html_list
            .iter()
            .filter_map(|x| if x.1 { Some(x.0.clone()) } else { None })
            .collect::<Html>();

        html! {
            <>
                { rows }
                <div class="d-flex flex-row">
                    <div class="pe-2">
                        <Button text={"Add new row"} on_click={ link.callback(|_| ItineraryListMessage::AddChild) } />
                    </div>
                    <div>
                        <Button text={"Go!"} on_click={ link.callback(|_| ItineraryListMessage::SendPost) } />
                    </div>
                </div>
            </>
        }
    }
}

fn main() {
    let window = web_sys::window().expect("Can't find window");
    let document = window.document().expect("Can't find document in window");
    let itin_box = document
        .get_element_by_id("rust-box")
        .expect("Can't find rust-box");

    wasm_logger::init(wasm_logger::Config::default());

    yew::Renderer::<ItineraryList>::with_root(itin_box).render();
}
