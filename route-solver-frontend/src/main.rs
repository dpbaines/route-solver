use route_solver_shared::Queries::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};
use yew::prelude::*;

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
struct DropDownProps {
    text: String,
    opts: Vec<String>,
    node_ref: NodeRef,
}

#[derive(PartialEq, Clone)]
enum ItemType {
    Fixed(u16),
}

#[derive(Properties, PartialEq, Clone)]
struct ItineraryListItemProps {
    item_type: ItemType,
    remove_handler: Callback<()>,
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
            Callback::from(move |_| {
                remove_handler.emit(());
            })
        };

        let id = if let ItemType::Fixed(id1) = ctx.props().item_type {
            id1
        } else {
            0
        };

        html! {
            <div id={ format!("itin-row-{}", id) } class={"row my-2 justify-content-start"}>
                <div class={"col"}>
                    <TextBox text={ "Airport Code" } node_ref={ self.node_refs[0].clone() } />
                </div>
                <div class={"col-md-auto"}>
                    <DropDown text={ "Flexibility" } opts={ vec!["Dates".to_string(), "Rough number of Days".to_string()]} node_ref={ self.node_refs[1].clone() } />
                </div>
                <div class={"col"}>
                    <TextBox text={ "Start Date" } node_ref={ self.node_refs[2].clone() } />
                </div>
                <div class={"col"}>
                    <TextBox text={ "End Date" } node_ref={ self.node_refs[3].clone() } />
                </div>
                <div class={"col"}>
                    <CloseButton text="Close" on_click={remove_handler_passthrough} />
                </div>
            </div>
        }
    }
}

#[function_component(ItineraryList)]
fn itin_list() -> Html {
    let default_list = vec![ItemType::Fixed(0), ItemType::Fixed(1), ItemType::Fixed(2)];

    let list_num_hook = use_state(move || default_list);

    let on_add_row = {
        let list_num_hook = list_num_hook.clone();
        Callback::from(move |_| {
            let mut li = list_num_hook.to_vec();
            list_num_hook.set({
                li.push(ItemType::Fixed(
                    match li.last().unwrap_or(&ItemType::Fixed(0)) {
                        ItemType::Fixed(i) => i + 1,
                        _ => 0,
                    },
                ));
                li
            })
        })
    };

    let post_data = {
        Callback::from(move |_| {
            let mut opts = RequestInit::new();
            opts.method("POST");
            let request = Request::new_with_str_and_init("runflights", &opts).unwrap();
            let window = web_sys::window().unwrap();
            let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
        })
    };

    let item_list_html = list_num_hook.iter().enumerate().map(|(idx, item)| {
        let remove_handler_passthrough = {
            let list_num_hook = list_num_hook.clone();
            Callback::from(move |_| {
                let li = list_num_hook
                    .iter()
                    .enumerate()
                    .filter(|(curr_idx, _)| idx != *curr_idx)
                    .map(|(_, item_t)| item_t.clone())
                    .collect::<Vec<ItemType>>();
                list_num_hook.set(li)
            })
        };

        html! {
            <ItineraryRow item_type={item.clone()} remove_handler={remove_handler_passthrough} />
        }
    });

    html! {
        <>
            { for item_list_html }
            <div>
                <Button text={"Add new row"} on_click={on_add_row} />
                <Button text={"CRUNch it"} on_click={post_data} />
            </div>
        </>
    }
}

fn main() {
    let window = web_sys::window().expect("Can't find window");
    let document = window.document().expect("Can't find document in window");
    let itin_box = document
        .get_element_by_id("rust-box")
        .expect("Can't find rust-box");

    yew::Renderer::<ItineraryList>::with_root(itin_box).render();
}
