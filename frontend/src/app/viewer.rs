use yew::{prelude::*, virtual_dom::AttrValue};

pub(crate) struct Viewer;

pub(crate) enum Msg {
    Delete,
}

#[derive(PartialEq, Properties)]
pub(crate) struct Props {
    pub(crate) value: AttrValue,
    pub(crate) on_delete: Callback<()>,
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
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let props = ctx.props();
        html! {
            <div style="padding: 4px; border: 1px dashed black;">
                { &props.value }
                <button onclick={link.callback(|_| Msg::Delete)}>{ "X" }</button>
            </div>
        }
    }
}
