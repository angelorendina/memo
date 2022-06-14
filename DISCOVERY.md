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
