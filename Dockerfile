FROM rust:1.75-bullseye as build

RUN apt-get update && \
    apt-get upgrade -y && \
    apt-get install -y openssl pkg-config libssl-dev

COPY . /src
WORKDIR /src
RUN cargo build --release

FROM debian:bullseye-slim

RUN apt-get update && \
    apt-get upgrade -y && \
    apt-get install -y libssl1.1 ca-certificates && \
    apt-get autoremove -y && \
    apt-get clean -y

COPY COPYING ./

COPY --from=build /src/target/release/quartermaster ./quartermaster

EXPOSE 8000
ENTRYPOINT ["./quartermaster"]
CMD []
