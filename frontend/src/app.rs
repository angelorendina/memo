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
    OnMemosFetched(Vec<common::Memo>),
    OnError(String),
    DeleteMemo(uuid::Uuid),
    OnMemoDeleted(uuid::Uuid),
    UpdateMemo(uuid::Uuid, bool),
    OnMemoUpdated(uuid::Uuid, bool),
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        fetch::get_memos(ctx);
        Self {
            memos: vec![],
            state: State::Loading,
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
            Msg::OnMemosFetched(memos) => {
                self.state = State::Ok;
                self.memos = memos;
                true
            }
            Msg::OnError(error) => {
                self.state = State::Error(error);
                true
            }
            Msg::DeleteMemo(id) => {
                self.state = State::Loading;
                fetch::delete_memo(ctx, common::DeleteMemoPayload { id });
                true
            }
            Msg::OnMemoDeleted(id) => {
                self.state = State::Ok;
                self.memos.retain(|memo| memo.id != id);
                true
            }
            Msg::UpdateMemo(id, done) => {
                self.state = State::Loading;
                fetch::resolve_memo(ctx, common::UpdateMemoPayload { id, done });
                true
            }
            Msg::OnMemoUpdated(id, done) => {
                self.state = State::Ok;
                self.memos.iter_mut().for_each(|memo| {
                    if memo.id == id {
                        memo.done = done
                    }
                });
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
                            { for self.memos.iter().map(|memo| {
                                let id = memo.id.clone();
                                let done = memo.done.clone();
                                html!(
                                    <viewer::Viewer
                                        value={AttrValue::from(memo.text.clone())}
                                        checked={memo.done}
                                        on_delete={link.callback(move |_| Msg::DeleteMemo(id))}
                                        on_change={link.callback(move |_| Msg::UpdateMemo(id, !done))}
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
