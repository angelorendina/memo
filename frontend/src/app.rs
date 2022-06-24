mod viewer;
mod writer;

use yew::{prelude::*, virtual_dom::AttrValue};

pub(crate) struct App {
    memos: Vec<String>,
}

pub(crate) enum Msg {
    CreateMemo(String),
    DeleteMemo(usize),
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
            Msg::DeleteMemo(index) => {
                self.memos.remove(index);
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        html! {
            <div>
                <writer::Writer on_submit={link.callback(Msg::CreateMemo)}/>
                <h3>{ "Memos" }</h3>
                <div style="display: grid; row-gap: 8px; grid-auto-flow: row;">
                    { for self.memos.iter().enumerate().map(|(index, memo)| {
                        html!(
                            <viewer::Viewer
                                value={AttrValue::from(memo.clone())}
                                on_delete={link.callback(move |_| Msg::DeleteMemo(index))}
                            />
                        )
                    })}
                </div>
            </div>
        }
    }
}
