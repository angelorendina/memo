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
