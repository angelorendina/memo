use super::{App, Msg};

const BACKEND_URL: &'static str = "http://localhost:3000";

pub(crate) fn create_memo(ctx: &yew::Context<App>, new_memo: common::NewMemoPayload) {
    let link = ctx.link().clone();
    match serde_json::to_string(&new_memo) {
        Ok(payload) => {
            wasm_bindgen_futures::spawn_local(async move {
                let response = reqwasm::http::Request::post(BACKEND_URL)
                    .body(payload)
                    .header("content-type", "application/json")
                    .send()
                    .await;
                match response {
                    Ok(body) => match body.json::<common::Memo>().await {
                        Ok(memo) => {
                            link.send_message(Msg::OnMemoCreated(memo));
                        }
                        Err(error) => {
                            link.send_message(Msg::OnError(error.to_string()));
                        }
                    },
                    Err(error) => {
                        link.send_message(Msg::OnError(error.to_string()));
                    }
                }
            });
        }
        Err(error) => {
            link.send_message(Msg::OnError(error.to_string()));
        }
    }
}