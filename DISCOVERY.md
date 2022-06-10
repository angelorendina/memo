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
