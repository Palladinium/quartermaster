FROM rust:latest as build

COPY . /src
WORKDIR /src
RUN cargo build --release

CMD ["/src/target/release/quartermaster"]
