# Dockerising and deploying a full-stack Rust + WASM web app

This is the code for the homonymous talk I gave at [RustLab2022](https://rustlab.it/).

The supporting presentation slides are in the `slides.pdf` file in this repo, and a recording of the talk can be found [on Youtube](https://www.youtube.com/watch?v=0geOH76BCbQ).

It is a full-stack "todo" app, with both Backend and Frontend written in Rust.

The backend mainly uses Actix to spin up a HttpServer.

The frontend uses the Yew framework to compile Rust into WASM: it is a client-side rendered SPA.

The code is organised as a monorepo, using `cargo workspace` to tie it all together and `cargo-make` to run and build it.

The `k8s` folder contains configuration files to deploy the app through kubernetes, mainly geared towards deploying to the local minikube cluster.

See the `DISCOVERY.md` file for a step-by-step walkthrough of how I wrote the app, learning the various tools at each step. Each section corresponds to a commit, so you can use the git history to navigate in time and see the development as it happened.
