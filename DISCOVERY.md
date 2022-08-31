# Discovery process
## Dockerising Rust for local development
You don't need to install Rust on your machine, thanks to Docker and the `rust` image.

Just set up `docker-compose.yml`:
```
services:
  app:
    image: rust
    volumes:
      - ".:/app"
      - "cargo:/app/.cargo"
      - "target:/app/target"
    working_dir: /app
    environment:
      CARGO_HOME: /app/.cargo
      CARGO_TARGET_DIR: /app/target

volumes:
  cargo:
  target:
```
so that:
- `/app` is where we work in the container (we set that workdir)
- the envars and volumes make sure we cache all Cargo stuff:
    - `CARGO_HOME` is where Cargo lives (https://doc.rust-lang.org/cargo/guide/cargo-home.html#cargo-home) - keeps installed tools and caches dependencies
    - `CARGO_TARGET_DIR` is where the build artifact are stashed - to avoid rebuilding everything from scratch

Launch and log into container with `docker-compose run app`. You will be in the `/app` directory.

Initialise a new Rust project with `cargo init`, which will create the default Hello World app and setup the git repository.

Install `cargo-make` with `cargo install cargo-make`. As per [cargo install docs](https://doc.rust-lang.org/cargo/commands/cargo-install.html#description), it gets installed into `CARGO_HOME/bin`, which is already within the volume.

Define
```
[tasks.run]
command = "cargo"
args = ["run"]
```
in `Makefile.toml` so that `cargo make run` compiles and runs Hello World (dev build).

## HTTP server with Actix
Add `Actix` as dependency in `Cargo.toml`
```
[dependencies]
actix-web = "4"
```
and then write a simple HTTP server in `main.rs`
```
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    actix_web::HttpServer::new(move || {
        actix_web::App::new()
            .route("/", actix_web::web::get().to(hello_world))
    })
    .bind(("0.0.0.0", 3000))?
    .run()
    .await
}

async fn hello_world() -> actix_web::HttpResponse {
    actix_web::HttpResponse::Ok().body("Hello, world!")
}
```
that will display Hello World when GETting `localhost:3000`. Before running this, we need to thread the internal container port through Docker, so in `docker-compose.yml` add
```
services:
  app:
    ports:
      - 3000:3000
    image: rust
    ...
```
Now we can relaunch the container with `docker-compose run --service-ports app` and then `cargo make run`.

Viewing `http://localhost:3000` in a browser will display the message!

## Postgres
We use Postgres to store data. Add this to `docker-compose.yml`
```
services:
  app:
    ...
    depends_on:
      postgres:
        condition: service_healthy
  
  postgres:
    image: postgres
    environment:
      - POSTGRES_PASSWORD=password
      - POSTGRES_USER=user
      - POSTGRES_DB=db
      - PGDATA=/postgres
    healthcheck:
      test: ["CMD", "pg_isready"]
      interval: 5s
      timeout: 5s
      retries: 5
    volumes:
      - "postgres:/postgres"

volumes:
  ...
  postgres
```
so that Postgres gets launched first, and when the healthcheck detects that it is up and running we also start the app.
We also set a volume to store the actual data, which is located in `/postgres` as per the `PGDATA` envar.

## Setting up the database
Now `docker-compose run --service-ports app` will launch postgres and then log us into the app container.

We `cargo install sqlx-cli` to easily manage the database through the cli and cargo-make.

Add the necessary `DATABASE_URL` envar needed by the cli and the task to `Makefile.toml`
```
[env]
DATABASE_URL="postgresql://user:password@postgres:5432/db"

[tasks.db-migration]
command = "cargo"
args = ["sqlx", "migrate", "add", "${@}"]
```
and then `cargo make db-migration memo` to generate the first migration in `./migrations`:
```
CREATE TABLE memos(
  "id" UUID PRIMARY KEY NOT NULL,
  "timestamp" TIMESTAMP WITH TIME ZONE NOT NULL,
  "done" BOOLEAN NOT NULL,
  "text" TEXT NOT NULL
);
```

Then in `Makefile.toml`
```
[tasks.db-setup]
command = "sqlx"
args = ["database", "setup", "--source=./migrations/"]
```
so that `cargo make db-setup` prepares the database.

## Reading from the database
We add a few dependencies to `Cargo.toml`
```
chrono = { version = "0.4.19", features = ["serde"] }
serde = "1"
sqlx = { version = "0.5", features = ["runtime-tokio-native-tls", "uuid", "json", "chrono", "migrate", "postgres", "offline"] }
uuid = { version = "0.8", features = ["serde", "v4"] }
```
and then add a new cargo make task to set up the [offline mode for sqlx](https://docs.rs/sqlx/latest/sqlx/macro.query.html#offline-mode-requires-the-offline-feature)
```
[tasks.db-prepare]
command = "cargo"
args = ["sqlx", "prepare"]
```

In a new module `memo.rs` define
```
#[derive(serde::Serialize)]
struct Memo {
    id: uuid::Uuid,
    timestamp: chrono::DateTime<chrono::Utc>,
    done: bool,
    text: String,
}

impl Memo {
    async fn index(executor: impl sqlx::PgExecutor<'_>) -> Result<Vec<Memo>, sqlx::Error> {
        sqlx::query_as!(
            Memo,
            r#"
        SELECT
            id, timestamp, done, text
        FROM memos
        ORDER BY timestamp
            "#,
        )
        .fetch_all(executor)
        .await
    }
}
```
and now we can `cargo make db-prepare` to generate a new `sqlx-data.json` for sqlx's offline mode.

Before defining the new endpoint, we need to establish a connection with the db. We connect at app launch and store it in a global shared state, so in `main.rs`
```
struct AppState {
    pool: sqlx::Pool<sqlx::Postgres>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let app_state = actix_web::web::Data::new(AppState {
        pool: sqlx::pool::PoolOptions::new()
            .connect("postgresql://user:password@postgres:5432/db")
            .await
            .expect("Could not connect to the DB"),
    });

    actix_web::HttpServer::new(move || {
        actix_web::App::new()
            .app_data(app_state.clone())
            .route("/", actix_web::web::get().to(hello_world))
    })
    .bind(("0.0.0.0", 3000))?
    .run()
    .await
}
```

We can define the new endpoint in `memo.rs`
```
pub(crate) async fn index(app_state: actix_web::web::Data<crate::AppState>) -> actix_web::HttpResponse {
    match Memo::index(&app_state.pool).await {
        Ok(memos) => actix_web::HttpResponse::Ok().json(memos),
        Err(_) => actix_web::HttpResponse::InternalServerError().finish(),
    }
}
```
and remember to mount it
```
.route("/", actix_web::web::get().to(memo::index))
```
replacing the old Hello World (and deleting the unused handler!).

Relaunching the app with `cargo make run` and GETting `locahost:3000` should now display an empty list.

## CRUD
In the impl block for Memo we add the write model
```
async fn insert(
    executor: impl sqlx::PgExecutor<'_>,
    memo: &Memo,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
    INSERT INTO memos
        (id, timestamp, done, text)
    VALUES($1, $2, $3, $4)
        "#,
    )
    .bind(&memo.id)
    .bind(&memo.timestamp)
    .bind(&memo.done)
    .bind(&memo.text)
    .execute(executor)
    .await
    .map(|_| ())
}
```
and then the actual handler
```
#[derive(serde::Deserialize)]
pub(crate) struct NewMemoPayload {
    text: String,
}

pub(crate) async fn create(
    app_state: actix_web::web::Data<crate::AppState>,
    payload: actix_web::web::Json<NewMemoPayload>,
) -> actix_web::HttpResponse {
    let id = uuid::Uuid::new_v4();
    let memo = Memo {
        id,
        timestamp: chrono::Utc::now(),
        done: false,
        text: payload.into_inner().text,
    };
    match Memo::insert(&app_state.pool, &memo).await {
        Ok(_) => actix_web::HttpResponse::Ok().json(memo),
        Err(_) => actix_web::HttpResponse::InternalServerError().finish(),
    }
}
```
not forgetting to route it
```
.route("/", actix_web::web::post().to(memo::create))
```

Can now POST to `localhost:3000` with a JSON body
```
{
	"text": "My memo"
}
```
to create a new memo and record it into the database.

We finally add endpoints to resolve and delete memos
```
impl Memo {
    ...
    async fn update(
        executor: impl sqlx::PgExecutor<'_>,
        id: &uuid::Uuid,
        done: &bool,
    ) -> Result<bool, sqlx::Error> {
        sqlx::query(
            r#"
        UPDATE memos
        SET done = $2
        WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(done)
        .execute(executor)
        .await
        .map(|result| result.rows_affected() > 0)
    }

    async fn delete(
        executor: impl sqlx::PgExecutor<'_>,
        id: &uuid::Uuid,
    ) -> Result<bool, sqlx::Error> {
        sqlx::query(
            r#"
        DELETE FROM memos
        WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(executor)
        .await
        .map(|result| result.rows_affected() > 0)
    }
}

...

#[derive(serde::Deserialize)]
pub(crate) struct UpdateMemoPayload {
    id: uuid::Uuid,
    done: bool,
}

pub(crate) async fn resolve(
    app_state: actix_web::web::Data<crate::AppState>,
    payload: actix_web::web::Json<UpdateMemoPayload>,
) -> actix_web::HttpResponse {
    let payload = payload.into_inner();
    match Memo::update(&app_state.pool, &payload.id, &payload.done).await {
        Ok(deleted) => {
            if deleted {
                actix_web::HttpResponse::Ok().finish()
            } else {
                actix_web::HttpResponse::NotFound().finish()
            }
        },
        Err(_) => actix_web::HttpResponse::InternalServerError().finish(),
    }
}

#[derive(serde::Deserialize)]
pub(crate) struct DeleteMemoPayload {
    id: uuid::Uuid,
}

pub(crate) async fn delete(
    app_state: actix_web::web::Data<crate::AppState>,
    payload: actix_web::web::Json<DeleteMemoPayload>,
) -> actix_web::HttpResponse {
    let payload = payload.into_inner();
    match Memo::delete(&app_state.pool, &payload.id).await {
        Ok(deleted) => {
            if deleted {
                actix_web::HttpResponse::Ok().finish()
            } else {
                actix_web::HttpResponse::NotFound().finish()
            }
        },
        Err(_) => actix_web::HttpResponse::InternalServerError().finish(),
    }
}
```
and
```
.route("/", actix_web::web::put().to(memo::resolve))
.route("/", actix_web::web::delete().to(memo::delete))
```

## Logs and autorun
To quickly add some logs, we add the dependency
```
env_logger = "0.9"
```
and then use it in `main`
```
async fn main() -> std::io::Result<()> {
    env_logger::init();

    ...

    actix_web::HttpServer::new(move || {
        actix_web::App::new()
            .wrap(actix_web::middleware::Logger::default())
            ...
}
```
We also set the log level in `Makefile.toml` as
```
[env]
RUST_LOG="debug"
```
Now `cargo make run` should print out the logs.

To fully complete the backend, we add the command in `docker-compose.yml`
```
services:
  app:
    command: cargo make run
    ...
```
This way, running `docker-compose run --service-ports app` will automatically compile and run the app.
We can still `docker-compose run --service-ports app bash` to get an interactive session inside the container, but then would have to manually `cargo make run` to launch the server.

## Workspace and new package
We are going to split backend and frontend as two packages in the same workspace, a la monorepo.

First, create the `backend` folder and move `src` and `Cargo.toml` in there, also renaming `name = "backend"` in the latter. Then create a new `Cargo.toml` in root with
```
[workspace]
members = [
    "backend",
]
```
and also tweak the old service in `docker-compose.yml` 
```
services:
  backend:
    command: cargo make run-backend
    ...
```
removing the port mapping as we will not expose the backend directly. Finally, update `Makefile.toml` with
```
[config]
default_to_workspace = false

[tasks.run-backend]
command = "cargo"
args = ["run", "-p", "backend"]

[tasks.db-prepare]
command = "cargo"
args = ["sqlx", "prepare", "--", "-p", "backend"]
```
The `default_to_config` flag is related to [workspace support](https://github.com/sagiegurari/cargo-make#workspace-support).

Now everything should be still working with `docker-compose run backend bash` and then `cargo make backend-run`.

Next step is to add a new service to `docker-compose.yml`
```
  frontend:
    image: rust
    volumes:
      - ".:/app"
      - "cargo:/app/.cargo"
      - "target:/app/target"
    working_dir: /app
    environment:
      CARGO_HOME: /app/.cargo
      CARGO_TARGET_DIR: /app/target
    ports:
      - 8080:8080
    depends_on:
      - backend
```
and then create a new `frontend` folder with `frontend/src/main.rs`
```
fn main() {
    println!("Hello, world!");
}
```
and `frontend/Cargo.toml`
```
[package]
name = "frontend"
version = "0.1.0"
edition = "2021"

# See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

[dependencies]
```
Add also the new member of the workspace to the root `Cargo.toml`
```
[workspace]
members = [
    "backend",
    "frontend",
]
```
and a task to run the project in `Makefile.toml`
```
[tasks.frontend-run]
command = "cargo"
args = ["run", "-p", "frontend"]
```
Because we set the frontend container depending on the backend, launching `docker-compose run frontend` should spin up the database first, then the backend, and finally log into the frontend, where `cargo make frontend-run` should print "Hello, World!".

## Serving the frontend
To serve the WASM frontend, we need to install the appropriate Rust toolchain. Set the `RUSTUP_HOME` envar to cache it into a volume
```
frontend:
  image: rust
  volumes:
    - ".:/app"
    - "cargo:/app/.cargo"
    - "target:/app/target"
    - "rustup:/app/.rustup"
  working_dir: /app
  environment:
    CARGO_HOME: /app/.cargo
    CARGO_TARGET_DIR: /app/target
    RUSTUP_HOME: /app/.rustup
  ports:
    - 8080:8080
  depends_on:
    - backend
...
volumes:
  rustup:
  ...
```
and then `docker-compose run --service-ports frontend` to get into the container.

Running `rustup show` will display that no toolchains are installed - as we set a custom folder for those. So we first run `rustup toolchain install stable` and then `rustup target add wasm32-unknown-unknown` to add the WASM target.

To simplify development and build of the frontend, we also `cargo install trunk` and `cargo install wasm-bindgen-cli`.

Create `frontend/index.html`
```
<!DOCTYPE html>
<html>
  <head>
    <meta charset="utf-8" />
    <title>Memo</title>
  </head>
</html>
```
and update `Makefile.toml`
```
[tasks.frontend-run]
command = "trunk"
args = ["serve", "--address", "0.0.0.0", "--dist", "./target/dist", "./frontend/index.html"]
```
Running `cargo make frontend-run` should now start compiling and serving the frontend (a blank page with title `Memos`).

## Yew frontend
Install the needed dependencies in `frontend/Cargo.toml`
```
[dependencies]
yew = "0.19"
web-sys = "0.3"
```
and then we can get started writing the web page in `frontend/src/main.rs`
```
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
            },
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
```
With `cargo make frontend-run`, Trunk will serve the page and also automatically watch for changes and recompile/hot-reload the frontend. Viewing the page on `http://localhost:8080` should display an input box, and its content should be duplicated below it.

## Child-to-parent communication
We create a Writer component, which has an input box to type in and a button to submit the value, so in `frontend/src/app/writer.rs`
```
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
```
The component has no knowledge of where it is or who the parent is. It only communicates upwards through the `on_submit` callback, which is invoked when the button is clicked. Its type is `Callback<String>`, and must be called with `.emit(value)` where `value` is a String (here, it's the value of the input element, which we hold a reference to with `NodeRef`).

The parent hosts the child component and connects its `on_input` callback to the appropriate handler: in `frontend/src/app.rs`
```
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
```
where the handler will emit a `CreateMemo` message for the update function to deal with. Here we simply store all created strings in a vector, and display that as a list of divs.

Finally we refactor `frontend/src/main.rs` for clarity
```
mod app;

fn main() {
    yew::start_app::<app::App>();
}
```
The frontend should now compile and display a boxed element (the Writer) that allows to input and append a message in the space below.

## Parent-to-child communication
Parents pass information to children via props. Here we implement a component to view and delete individual memos: in `frontend/src/app/viewer.rs`
```
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
```
which takes its value as an `AttrValue` from the parent, and also a callback to bubble upwards the deletion of a memo.

We also update `frontend/src/app.rs` to include the new viewer
```
mod viewer;
...
pub(crate) enum Msg {
    CreateMemo(String),
    DeleteMemo(usize),
}
...
fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
    match msg {
        ...
        Msg::DeleteMemo(index) => {
            self.memos.remove(index);
            true
        }
    }
}
...
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
```
We can now create, view and delete simple memos. They will go away when we refresh the page though!

## Posting new memo
We want the frontend to call the backend. Both being in Rust, we can share types between the two. Create a new crate `common`, so `common/Cargo.toml`
```
[package]
name = "common"
version = "0.1.0"
edition = "2021"

# See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

[dependencies]
chrono = { version = "0.4.19", features = ["serde"] }
serde = "1"
uuid = { version = "0.8", features = ["serde"] }
```
and `common/src/lib.rs`
```
#[derive(serde::Deserialize, serde::Serialize)]
pub struct Memo {
    pub id: uuid::Uuid,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub done: bool,
    pub text: String,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct NewMemoPayload {
    pub text: String,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct UpdateMemoPayload {
    pub id: uuid::Uuid,
    pub done: bool,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct DeleteMemoPayload {
    pub id: uuid::Uuid,
}
```

Now we update the backend to use these shared types: tweak the necessary dependencies in `backend/Cargo.toml`
```
actix-cors = "0.6"
actix-web = "4"
chrono = "0.4"
common = { path = "../common" }
env_logger = "0.9"
sqlx = { version = "0.5", features = ["runtime-tokio-native-tls", "uuid", "json", "chrono", "migrate", "postgres", "offline"] }
uuid = { version = "0.8", features = ["v4"] }
```
also including `actix-cors`, which we will use shortly.

Replace all the old types from `backend/src/memo.rs` in favour of those from the new `common` crate [code refactor omitted].

Finally, add some CORS configuration for development in `backend/src/main.rs`
```
actix_web::HttpServer::new(move || {
    actix_web::App::new()
        .wrap(actix_cors::Cors::permissive())
        ...
```

For the frontend, update the dependencies in `frontend/Cargo.toml`
```
common = { path = "../common" }
reqwasm = "0.5"
serde_json = "1"
yew = "0.19"
wasm-bindgen-futures = "0.4"
web-sys = "0.3"
```
and update `frontend/src/app.rs`
```
mod fetch;

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
    ...
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
            ...
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        match &self.state {
            State::Loading => html!(<div></div>),
            State::Error(error) => html!(<div>{ &error }</div>),
            State::Ok => { ... },
        }
    }
}
```
and create a new module `frotend/src/app/fetch.rs`
```
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
```
The frontend should now be able to actually create new memos in the database!

## Fetching all memos
To load all memos on page load, we add a new method in `frontend/src/app/fetch.rs`
```
pub(crate) fn get_memos(ctx: &yew::Context<App>) {
    let link = ctx.link().clone();
    wasm_bindgen_futures::spawn_local(async move {
        let response = reqwasm::http::Request::get(BACKEND_URL).send().await;
        match response {
            Ok(body) => match body.json::<Vec<common::Memo>>().await {
                Ok(memos) => {
                    link.send_message(Msg::OnMemosFetched(memos));
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
```
and tweak `frontend/src/app.rs` to fetch on created
```
pub(crate) enum Msg {
    OnMemosFetched(Vec<common::Memo>),
    ...
}

impl Component for App {
    ...
    fn create(ctx: &Context<Self>) -> Self {
        fetch::get_memos(ctx);
        Self {
            memos: vec![],
            state: State::Loading,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::OnMemosFetched(memos) => {
                self.state = State::Ok;
                self.memos = memos;
                true
            }
            ...
        }
    }
```

## Deleting a memo
Add the functionality to `frontend/src/app/fetch.rs`
```
pub(crate) fn delete_memo(ctx: &yew::Context<App>, delete_memo: common::DeleteMemoPayload) {
    let link = ctx.link().clone();
    match serde_json::to_string(&delete_memo) {
        Ok(payload) => {
            wasm_bindgen_futures::spawn_local(async move {
                let response = reqwasm::http::Request::delete(BACKEND_URL)
                    .body(payload)
                    .header("content-type", "application/json")
                    .send()
                    .await;
                match response {
                    Ok(_) => {
                        link.send_message(Msg::OnMemoDeleted(delete_memo.id));
                    }
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
```
and then tweak `frontend/src/app.rs`
```
pub(crate) enum Msg {
    ...
    DeleteMemo(uuid::Uuid),
    OnMemoDeleted(uuid::Uuid),
}
...
fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
    match msg {
        ...
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
    }
}
...
fn view(&self, ctx: &Context<Self>) -> Html {
    match &self.state {
        ...
        State::Ok => {
            let link = ctx.link();
            html! {
                <div>
                    <writer::Writer on_submit={link.callback(Msg::CreateMemo)}/>
                    <h3>{ "Memos" }</h3>
                    <div style="display: grid; row-gap: 8px; grid-auto-flow: row;">
                        { for self.memos.iter().map(|memo| {
                            let id = memo.id.clone();
                            html!(
                                <viewer::Viewer
                                    value={AttrValue::from(memo.text.clone())}
                                    on_delete={link.callback(move |_| Msg::DeleteMemo(id))}
                                />
                            )
                        })}
                    </div>
                </div>
            }
        }
    }
}
```

## Resolving a memo
Last touch is to mark memos as resolved. Add the api call in `frontend/src/app/fetch.rs`
```
pub(crate) fn resolve_memo(ctx: &yew::Context<App>, update_memo: common::UpdateMemoPayload) {
    let link = ctx.link().clone();
    match serde_json::to_string(&update_memo) {
        Ok(payload) => {
            wasm_bindgen_futures::spawn_local(async move {
                let response = reqwasm::http::Request::put(BACKEND_URL)
                    .body(payload)
                    .header("content-type", "application/json")
                    .send()
                    .await;
                match response {
                    Ok(_) => {
                        link.send_message(Msg::OnMemoUpdated(update_memo.id, update_memo.done));
                    }
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
```
and update `frontend/src/app.rs`
```
pub(crate) enum Msg {
    ...
    UpdateMemo(uuid::Uuid, bool),
    OnMemoUpdated(uuid::Uuid, bool),
}
...
fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
    match msg {
        ...
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
...
fn view(&self, ctx: &Context<Self>) -> Html {
    match &self.state {
        ...
        html!(
            <viewer::Viewer
                value={AttrValue::from(memo.text.clone())}
                checked={memo.done}
                on_delete={link.callback(move |_| Msg::DeleteMemo(id))}
                on_change={link.callback(move |_| Msg::UpdateMemo(id, !done))}
            />
        )
        ...
    }
}
```
and add the new functionality in `frontend/src/app/viewer.rs`
```
pub(crate) enum Msg {
    ...
    Change,
}
...
#[derive(PartialEq, Properties)]
pub(crate) struct Props {
    ...
    pub(crate) checked: bool,
    pub(crate) on_change: Callback<()>,
}
...
fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
    match msg {
        ...
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
```

## Completing the local development
Update `backend/src/main.rs`
```
async fn main() -> std::io::Result<()> {
    let database_url = std::env!("DATABASE_URL");
    let server_port = std::env!("BACKEND_PORT")
        .parse::<u16>()
        .expect("Port must be a u16");

    env_logger::init();

    let app_state = actix_web::web::Data::new(AppState {
        pool: sqlx::pool::PoolOptions::new()
            .connect(&database_url)
            .await
            .expect("Could not connect to the DB"),
    });

    actix_web::HttpServer::new(...)
    .bind(("0.0.0.0", server_port))?
    .run()
    .await
}
```
and `frontend/src/app/fetch.rs`
```
const BACKEND_URL: &'static str = std::env!("BACKEND_URL");
```
and `Makefile.toml`
```
[env]
RUST_LOG="debug"
DATABASE_URL="postgresql://user:password@postgres:5432/db"
BACKEND_URL="http://localhost:3000"
BACKEND_PORT=3000
...
```
and finally tweak `docker-compose.yml`
```
services:
  frontend:
    command: cargo make frontend-run
....
```
Now the whole app should run with `docker-compose up`.

## Building release images
We use cargo-make's profiles to streamline the process. Update `Makefile.toml`
```
[env.development]
RUST_LOG="debug"
DATABASE_URL="postgresql://user:password@postgres:5432/db"
BACKEND_URL="http://localhost:3000"
BACKEND_PORT=3000

[env.release]
RUST_LOG="info"
DATABASE_URL="${DATABASE_URL}"
BACKEND_URL="${BACKEND_URL}"
BACKEND_PORT="${BACKEND_PORT}"

[tasks.backend-build]
command = "cargo"
args = ["build", "--release", "-p", "backend"]

[tasks.frontend-build]
command = "trunk"
args = ["build", "--release", "--dist", "./dist", "./frontend/index.html"]
...
```
and tweak `docker-compose.yml`
```
services:
  backend:
    ...
    command: cargo make --profile development backend-run

  frontend:
    ...
    command: cargo make --profile development frontend-run

  postgres:
    ...
    ports:
      - 5432:5432
```
Define the build instructions for the BE in `backend/Dockerfile`
```
FROM rust AS builder
RUN cargo install cargo-make

FROM builder as compiler
ARG BACKEND_PORT
ARG DATABASE_URL
ENV BACKEND_PORT ${BACKEND_PORT}
ENV DATABASE_URL ${DATABASE_URL}
COPY . /memo
WORKDIR /memo
RUN cargo make --profile release backend-build

FROM debian:buster-slim as runner
RUN apt update
RUN apt install -y libssl1.1

FROM runner
ARG BACKEND_PORT
COPY --from=compiler /memo/target/release/backend /memo/backend
EXPOSE ${BACKEND_PORT}
CMD ["/memo/backend"]
```
and for the FE in `frontend/Dockerfile`
```
FROM rust AS base
RUN cargo install cargo-make
RUN rustup toolchain install stable
RUN rustup target add wasm32-unknown-unknown
RUN cargo install trunk
RUN cargo install wasm-bindgen-cli

FROM base AS builder
ARG BACKEND_URL
ENV BACKEND_URL ${BACKEND_URL}
COPY . /memo
WORKDIR /memo
RUN cargo make --profile release frontend-build

FROM httpd:alpine
COPY --from=builder /memo/dist/. /usr/local/apache2/htdocs/.
EXPOSE 80
```

We can build the images and run them locally to test them out. For the backend,
```
docker build -f ./backend/Dockerfile -t memo-backend --build-arg DATABASE_URL=postgresql://user:password@host.docker.internal:5432/db --build-arg BACKEND_PORT=3000 .
docker create -p 3000:3000 --name memo-backend memo-backend
docker start memo-backend
```
which should serve the BE on `localhost:3000`. To test it out, run postgres locally with `docker-compose run --service-ports postgres` and restart the BE container so that it can connect.

In parallel we can also
```
docker build -f ./frontend/Dockerfile -t memo-frontend --build-arg BACKEND_URL=http://localhost:3000 .
docker create -p 8080:80 --name memo-frontend memo-frontend
docker start memo-frontend
```
to run the FE on `localhost:8080`, which should prove the whole app working as intended.

## Deploying with Kubernetes
We use `minikube` to simulate Kubernetes locally.

We want a Postgres deployment and a backend deployment (connected internally with the DB) which exposes the API publicly on `http://backend.memo`.
In addition, another deployment will serve the frontend on `http://frontend.memo`.

We start with the configuration for the db in `k8s/postgres/configmap.yml`
```
apiVersion: v1
kind: ConfigMap
metadata:
  name: postgres-config
  labels:
    app: postgres
data:
  POSTGRES_DB: db
  POSTGRES_USER: user
  POSTGRES_PASSWORD: password
```
and a storage to persist the data in `k8s/postgres/storage.yml`
```
apiVersion: v1
kind: PersistentVolume
metadata:
  name: postgres-pv-volume
  labels:
    type: local
    app: postgres
spec:
  storageClassName: manual
  capacity:
    storage: 5Gi
  accessModes:
    - ReadWriteMany
  hostPath:
    path: "/mnt/data"
---
kind: PersistentVolumeClaim
apiVersion: v1
metadata:
  name: postgres-pv-claim
  labels:
    app: postgres
spec:
  storageClassName: manual
  accessModes:
    - ReadWriteMany
  resources:
    requests:
      storage: 5Gi
```
The deployment (using a default postgres image) is `k8s/postgres/deployment.yml`
```
apiVersion: apps/v1
kind: Deployment
metadata:
  name: postgres
  labels:
    app: postgres
spec:
  replicas: 1
  selector:
    matchLabels:
      app: postgres
  template:
    metadata:
      labels:
        app: postgres
    spec:
      containers:
        - name: postgres
          image: postgres:14.5-alpine
          imagePullPolicy: IfNotPresent
          ports:
            - containerPort: 5432
          envFrom:
            - configMapRef:
                name: postgres-config
          volumeMounts:
            - mountPath: /var/lib/postgresql/data
              name: postgredb
      volumes:
        - name: postgredb
          persistentVolumeClaim:
            claimName: postgres-pv-claim
```
and finally the (internal) service `k8s/postgres/service.yml`
```
apiVersion: v1
kind: Service
metadata:
  name: postgres
  labels:
    app: postgres
spec:
  ports:
  - port: 5432
  selector:
    app: postgres
```

Now deploy everything with
```
kubectl apply -f k8s/postgres/configmap.yml
kubectl apply -f k8s/postgres/storage.yml
kubectl apply -f k8s/postgres/deployment.yml
kubectl apply -f k8s/postgres/service.yml
```

Obviously the database starts empty, so we need to apply the app schema. We already have the necessary migration, but we need to access the database (which is not exposed outside the cluster).
With `minikube service postgres --url` we can temporarily open a tunnel via `localhost:XXXXX` (a random port will be allocated), and then run
```
DATABASE_URL=postgresql://user:password@localhost:XXXXX/db cargo sqlx database setup --source=./migrations
```
to generate the necessary table. The tunnel can now be closed.

For the backend, we first need to build the image with the correct configuration, so
```
docker build -f ./backend/Dockerfile -t memo-backend --build-arg DATABASE_URL=postgresql://user:password@postgres:5432/db --build-arg BACKEND_PORT=3000 .
```
Notice the db url refers to `postgres`, the name of the k8s service inside the cluster. Then
```
minikube image load memo-backend
```
and setup `k8s/backend/deployment.yml`
```
apiVersion: apps/v1
kind: Deployment
metadata:
  name: backend
  labels:
    app: backend
spec:
  replicas: 1
  selector:
    matchLabels:
      app: backend
  template:
    metadata:
      labels:
        app: backend
    spec:
      containers:
        - name: backend
          image: memo-backend
          imagePullPolicy: Never
          ports:
            - containerPort: 3000
```
and `k8s/backend/service.yml`
```
apiVersion: v1
kind: Service
metadata:
  name: backend
  labels:
    app: backend
spec:
  ports:
   - port: 3000
  selector:
   app: backend
```
and apply everything with
```
kubectl apply -f k8s/backend/deployment.yml
kubectl apply -f k8s/backend/service.yml
```

Similarly for the frontend
```
docker build -f ./frontend/Dockerfile -t memo-frontend --build-arg BACKEND_URL=http://backend.memo .
```
then
```
minikube image load memo-frontend
```
and `k8s/frontend/deployment.yml`
```
apiVersion: apps/v1
kind: Deployment
metadata:
  name: frontend
  labels:
    app: frontend
spec:
  replicas: 1
  selector:
    matchLabels:
      app: frontend
  template:
    metadata:
      labels:
        app: frontend
    spec:
      containers:
        - name: frontend
          image: memo-frontend
          imagePullPolicy: Never
          ports:
            - containerPort: 80
```
and `k8s/frontend/service.yml`
```
apiVersion: v1
kind: Service
metadata:
  name: frontend
  labels:
    app: frontend
spec:
  ports:
   - port: 80 
  selector:
   app: frontend
```
and apply everything with
```
kubectl apply -f k8s/frontend/deployment.yml
kubectl apply -f k8s/frontend/service.yml
```

To expose the app, we need an ingress routing the traffic. First
```
minikube addons enable ingress
```
then setup `k8s/ingress.yml`
```
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: memo
spec:
  rules:
  - host: backend.memo
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: backend
            port:
              number: 3000
  - host: frontend.memo
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: frontend
            port:
              number: 80
```
and
```
kubectl apply -f k8s/ingress.yml
```
Now open the local tunnel with
```
minikube tunnel
```
which listens on `localhost:80` and routes traffic through the ingress.

Last needed step (just for the local machine) is to setup the correct hostname resolution: simply add
```
127.0.0.1 backend.memo
127.0.0.1 frontend.memo
```
to `/etc/hosts`.

Visiting `http://frontend.memo` on the browser should display the running app, and the REST API should also be directly accessible on `http://backend.memo`.
