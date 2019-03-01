FROM hackgt/checkin-embedded-init:latest

WORKDIR /usr/src/checkin-embedded
COPY ./server /usr/src/checkin-embedded
RUN cargo build --release
CMD ["cargo", "run", "--release"]