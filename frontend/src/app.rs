mod writer;

use yew::prelude::*;

pub(crate) struct App {
    memos: Vec<String>,
}

pub(crate) enum Msg {
    CreateMemo(String),
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self { memos: vec![] }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::CreateMemo(value) => {
                self.memos.push(value);
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        html! {
            <div>
                <writer::Writer on_submit={link.callback(Msg::CreateMemo)}/>
                { for self.memos.iter().map(|memo| {
                    html!(
                        <div>{ memo }</div>
                    )
                })}
            </div>
        }
    }
}
