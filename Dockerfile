FROM rustlang/rust:nightly

# Create empty shell project
RUN USER=root cargo new --bin checkin-embedded
WORKDIR /checkin-embedded

COPY ./server/Cargo.lock ./Cargo.lock
COPY ./server/Cargo.toml ./Cargo.toml

# This will cache dependencies
RUN cargo build --release
RUN rm src/*.rs

COPY ./server/src ./src

# Build for release
RUN cargo build --release

EXPOSE 3000
CMD ["cargo", "run"]
