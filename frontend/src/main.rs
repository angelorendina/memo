use yew::prelude::*;

enum Msg {
    Changed(String),
}

struct App {
    memo: String,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            memo: String::new(),
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Changed(memo) => {
                self.memo = memo;
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        html! {
            <div>
                <input oninput={link.callback(|ev: InputEvent| Msg::Changed(
                    ev
                        .target_dyn_into::<web_sys::HtmlInputElement>()
                        .map(|h| h.value())
                        .unwrap_or(String::new())
                ))}/>
                <p>{ &self.memo }</p>
            </div>
        }
    }
}

fn main() {
    yew::start_app::<App>();
}
