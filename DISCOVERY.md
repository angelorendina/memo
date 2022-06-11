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
