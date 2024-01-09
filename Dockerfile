FROM rust:1.75-bullseye as build

COPY . /src
WORKDIR /src
RUN cargo build --release

FROM debian:bullseye-slim

COPY COPYING ./

COPY --from=build /src/target/release/quartermaster ./quartermaster

ENTRYPOINT ["./quartermaster"]
CMD []
