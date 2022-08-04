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
