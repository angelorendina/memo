mod fetch;
mod viewer;
mod writer;

use yew::{prelude::*, virtual_dom::AttrValue};

enum State {
    Loading,
    Error(String),
    Ok,
}

pub(crate) struct App {
    memos: Vec<common::Memo>,
    state: State,
}

pub(crate) enum Msg {
    CreateMemo(String),
    OnMemoCreated(common::Memo),
    OnError(String),
    DeleteMemo(usize),
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            memos: vec![],
            state: State::Ok,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::CreateMemo(value) => {
                self.state = State::Loading;
                fetch::create_memo(ctx, common::NewMemoPayload { text: value });
                true
            }
            Msg::OnMemoCreated(memo) => {
                self.state = State::Ok;
                self.memos.push(memo);
                true
            }
            Msg::OnError(error) => {
                self.state = State::Error(error);
                true
            }
            Msg::DeleteMemo(index) => {
                self.memos.remove(index);
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        match &self.state {
            State::Loading => html!(<div></div>),
            State::Error(error) => html!(<div>{ &error }</div>),
            State::Ok => {
                let link = ctx.link();
                html! {
                    <div>
                        <writer::Writer on_submit={link.callback(Msg::CreateMemo)}/>
                        <h3>{ "Memos" }</h3>
                        <div style="display: grid; row-gap: 8px; grid-auto-flow: row;">
                            { for self.memos.iter().enumerate().map(|(index, memo)| {
                                html!(
                                    <viewer::Viewer
                                        value={AttrValue::from(memo.text.clone())}
                                        on_delete={link.callback(move |_| Msg::DeleteMemo(index))}
                                    />
                                )
                            })}
                        </div>
                    </div>
                }
            }
        }
    }
}
