use yew::prelude::*;

pub(crate) struct Writer {
    input_ref: NodeRef,
}

pub(crate) enum Msg {
    Submit,
}

#[derive(PartialEq, Properties)]
pub(crate) struct Props {
    pub(crate) on_submit: Callback<String>,
}

impl Component for Writer {
    type Message = Msg;
    type Properties = Props;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            input_ref: Default::default(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Submit => {
                let input = self
                    .input_ref
                    .cast::<web_sys::HtmlInputElement>()
                    .map(|h| h.value())
                    .unwrap_or(String::new());
                ctx.props().on_submit.emit(input);
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        html! {
            <div style="border: 1px solid black; padding: 8px;">
                <div>{ "New Memo" }</div>
                <input ref={self.input_ref.clone()}/>
                <button onclick={link.callback(|_| Msg::Submit)}>{ "Submit" }</button>
            </div>
        }
    }
}
