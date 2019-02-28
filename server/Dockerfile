FROM rustlang/rust:nightly

WORKDIR /usr/src/checkin-embedded
COPY . /usr/src/checkin-embedded

RUN cargo build --release

EXPOSE 3000
CMD ["cargo", "run"]
