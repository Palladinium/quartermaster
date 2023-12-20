FROM rust:latest as build

COPY . /src
WORKDIR /src
RUN cargo build --release

FROM scratch
COPY --from=build target/quartermaster /bin/quartermaster
CMD ["/bin/quartermaster"]
