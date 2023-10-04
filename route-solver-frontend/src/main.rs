use std::{
    cell::RefCell,
    error::Error,
    fmt::{self, Debug, Display, Formatter},
    rc::Rc,
};

use log::info;
use route_solver_shared::queries::*;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{HtmlElement, HtmlInputElement, Request, RequestInit, RequestMode, Response};
use yew::{prelude::*, virtual_dom::Key};

#[derive(Properties, PartialEq)]
struct TextBoxProps {
    text: String,
    node_ref: NodeRef,
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

#[derive(PartialEq, Clone, Default)]
struct StartEndRefs {
    start_node: NodeRef,
    end_node: NodeRef,
}

impl StartEndRefs {
    fn get_strings(&self) -> (String, String) {
        let start_html = self.start_node.cast::<HtmlInputElement>().unwrap().value();
        let end_html = self.end_node.cast::<HtmlInputElement>().unwrap().value();

        (start_html, end_html)
    }
}

struct ItineraryListItems {
    items: Vec<ItineraryListItemProps>,
}

#[function_component(TextBox)]
fn text_box(TextBoxProps { text, node_ref }: &TextBoxProps) -> Html {
    let my_text_handle = use_state(|| "".to_string());

    let handle_input = Callback::from(move |input_event: InputEvent| {
        let input_elem: HtmlInputElement = input_event.target().unwrap().dyn_into().unwrap();
        let value = input_elem.value();
        my_text_handle.set(value);
    });

    html! {
        <input type={"text"} ref={node_ref.clone()} class={"form-control"} placeholder={text.clone()} aria-label={text.clone()} value={my_text_handle.clone()} oninput={handle_input} />
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
    node_refs: StartEndRefs,
}

#[function_component(FlyInComponent)]
fn fly_in(FlyInProps { node_refs }: &FlyInProps) -> Html {
    html! {
        <div class="d-inline-flex">
            <div class="input-group flex-nowrap pe-2">
                <span class="input-group-text" id="addon-wrapping">{ "Start" }</span>
                <input ref={ &node_refs.start_node } type="date" class="form-control" />
            </div>
            <div class="input-group flex-nowrap pe-2">
                <span class="input-group-text" id="addon-wrapping">{ "End" }</span>
                <input ref={ &node_refs.end_node } type="date" class="form-control" />
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
struct ListItemRefs {
    airport_ref: NodeRef,
    start_date_refs: StartEndRefs,
    end_date_refs: StartEndRefs,
    temp_constraints: StartEndRefs,
}

#[derive(Properties, PartialEq, Clone)]
struct ItineraryListItemProps {
    id: usize,
    remove_handler: Callback<usize>,
    list_item_refs: Rc<ListItemRefs>,
}

struct ItineraryRow {}

impl Component for ItineraryRow {
    type Properties = ItineraryListItemProps;
    type Message = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {}
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
                            <TextBox text={ "Airport Code" } node_ref={ &ctx.props().list_item_refs.airport_ref } />
                        </div>
                        <div class={"col-md-auto"}>
                            <CloseButton text="Close" on_click={remove_handler_passthrough} />
                        </div>
                    </div>
                    <div class="row justify-content-start">
                        <ListItem text={ "Add fly in dates" }>
                            <FlyInComponent node_refs={ ctx.props().list_item_refs.start_date_refs.clone() } />
                        </ListItem>
                    </div>
                    <div class="row justify-content-start">
                        <ListItem text={ "Add fly out dates" }>
                            <FlyInComponent node_refs={ ctx.props().list_item_refs.end_date_refs.clone() } />
                        </ListItem>
                    </div>
                    <div class="row justify-content-start">
                        <ListItem text={ "Add other constraints" }>
                            <FlyInComponent node_refs={ ctx.props().list_item_refs.temp_constraints.clone() } />
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
    ref_list: RefCell<Vec<Rc<ListItemRefs>>>,
}

enum ItineraryListMessage {
    AddChild,
    RemoveChild(usize),
}

impl ItineraryList {
    fn get_formatted_text(&self) -> String {
        self.html_list
            .iter()
            .enumerate()
            .filter(|(_, (_, on))| *on)
            .map(|(idx, (_, _))| {
                let row_ref_list = &self.ref_list.borrow()[idx];
                let airport_code = row_ref_list
                    .airport_ref
                    .cast::<HtmlInputElement>()
                    .unwrap()
                    .value();
                let start_dates = row_ref_list.start_date_refs.get_strings();
                let end_dates = row_ref_list.end_date_refs.get_strings();
                let temp_dates = row_ref_list.temp_constraints.get_strings();

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
            ref_list: RefCell::new(Vec::new()),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            ItineraryListMessage::AddChild => {
                let link = ctx.link();
                let count = self.curr_count.clone();
                self.ref_list
                    .borrow_mut()
                    .push(Rc::new(ListItemRefs::default()));
                self.html_list.push((html! { <ItineraryRow key={ self.curr_count.clone() } list_item_refs={ self.ref_list.borrow().last().unwrap().clone() } id={ self.curr_count.clone() } remove_handler={ link.callback(move |_| ItineraryListMessage::RemoveChild(count)) } /> }, true));
                self.curr_count += 1;
            }
            ItineraryListMessage::RemoveChild(idx) => self.html_list.iter_mut().for_each(|x| {
                if x.0.key().unwrap().eq(&Key::from(idx)) {
                    *x = (x.0.clone(), false)
                }
            }),
            // .retain(|x| !x.0.key().unwrap().eq(&Key::from(idx))),
        };

        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();

        let post_data = {
            Callback::from(move |_| {
                // Get data
                let test = self.get_formatted_text();
                // let text = self.get_formatted_text();

                // let mut opts = RequestInit::new();
                // opts.method("POST");
                // let request = Request::new_with_str_and_init("runflights", &opts).unwrap();
                // let window = web_sys::window().unwrap();
                // let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
            })
        };

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
                        <Button text={"Go!"} on_click={post_data} />
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
