use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    rc::Rc,
};

use log::info;
use route_solver_shared::queries::*;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};
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

#[derive(Properties, PartialEq, Clone)]
struct ItineraryListItemProps {
    id: usize,
    remove_handler: Callback<usize>,
}

struct ItineraryListItems {
    items: Vec<ItineraryListItemProps>,
}

#[function_component(TextBox)]
fn text_box(TextBoxProps { text, node_ref }: &TextBoxProps) -> Html {
    html! {
        <input type={"text"} ref={node_ref.clone()} class={"form-control"} placeholder={text.clone()} aria-label={text.clone()} />
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

#[function_component(FlyInComponent)]
fn fly_in() -> Html {
    html! {
        <div class="d-inline-flex">
            <div class="input-group flex-nowrap pe-2">
                <span class="input-group-text" id="addon-wrapping">{ "Start" }</span>
                <input type="date" class="form-control" />
            </div>
            <div class="input-group flex-nowrap pe-2">
                <span class="input-group-text" id="addon-wrapping">{ "End" }</span>
                <input type="date" class="form-control" />
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

struct ItineraryRow {
    node_refs: Vec<NodeRef>,
}

impl Component for ItineraryRow {
    type Properties = ItineraryListItemProps;
    type Message = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            node_refs: vec![
                NodeRef::default(),
                NodeRef::default(),
                NodeRef::default(),
                NodeRef::default(),
            ],
        }
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
                            <TextBox text={ "Airport Code" } node_ref={ self.node_refs[0].clone() } />
                        </div>
                        <div class={"col-md-auto"}>
                            <CloseButton text="Close" on_click={remove_handler_passthrough} />
                        </div>
                    </div>
                    <div class="row justify-content-start">
                        <ListItem text={ "Add fly in dates" }>
                            <FlyInComponent />
                        </ListItem>
                    </div>
                    <div class="row justify-content-start">
                        <ListItem text={ "Add fly out dates" }>
                            <FlyInComponent />
                        </ListItem>
                    </div>
                    <div class="row justify-content-start">
                        <ListItem text={ "Add other constraints" }>
                            <FlyInComponent />
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
}

enum ItineraryListMessage {
    AddChild,
    RemoveChild(usize),
}

impl Component for ItineraryList {
    type Properties = ();
    type Message = ItineraryListMessage;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            html_list: vec![],
            curr_count: 0,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            ItineraryListMessage::AddChild => {
                let link = ctx.link();
                let count = self.curr_count.clone();
                self.html_list.push((html! { <ItineraryRow key={ self.curr_count.clone() } id={ self.curr_count.clone() } remove_handler={ link.callback(move |_| ItineraryListMessage::RemoveChild(count)) } /> }, true));
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
                let mut opts = RequestInit::new();
                opts.method("POST");
                let request = Request::new_with_str_and_init("runflights", &opts).unwrap();
                let window = web_sys::window().unwrap();
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
                <div>
                    <Button text={"Add new row"} on_click={ link.callback(|_| ItineraryListMessage::AddChild) } />
                    <Button text={"CRUNch it"} on_click={post_data} />
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
