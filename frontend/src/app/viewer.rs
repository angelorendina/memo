use yew::{prelude::*, virtual_dom::AttrValue};

pub(crate) struct Viewer;

pub(crate) enum Msg {
    Delete,
    Change,
}

#[derive(PartialEq, Properties)]
pub(crate) struct Props {
    pub(crate) value: AttrValue,
    pub(crate) checked: bool,
    pub(crate) on_delete: Callback<()>,
    pub(crate) on_change: Callback<()>,
}

impl Component for Viewer {
    type Message = Msg;
    type Properties = Props;

    fn create(_ctx: &Context<Self>) -> Self {
        Self
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Delete => {
                ctx.props().on_delete.emit(());
                false
            }
            Msg::Change => {
                ctx.props().on_change.emit(());
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let props = ctx.props();
        html! {
            <div style="padding: 4px; border: 1px dashed black;">
                <button onclick={link.callback(|_| Msg::Delete)}>{ "X" }</button>
                <input type="checkbox" checked={props.checked} onchange={link.callback(|_| Msg::Change)}/>
                { &props.value }
            </div>
        }
    }
}
