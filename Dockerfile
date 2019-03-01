FROM hackgt/checkin-embedded-init:latest

WORKDIR /usr/src/checkin-embedded
COPY ./server /usr/src/checkin-embedded
RUN cargo build --release
FROM debian
COPY --from=0 /usr/src/checkin-embedded/target/release/checkin-embedded-server /checkin-embedded-server
RUN apt-get update && apt-get install libssl-dev -y
EXPOSE 3000
CMD /checkin-embedded-server