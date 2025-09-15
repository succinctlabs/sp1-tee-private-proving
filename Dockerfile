# Base stage: Install Rust and dependencies
FROM ubuntu:24.04 AS rust-base

WORKDIR /usr/src/app

# Install required dependencies
RUN apt-get update && apt-get install -y \
    curl \
    clang \
    build-essential \
    git \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH=/root/.cargo/bin:$PATH
RUN rustup install stable && rustup default stable

# Install Go
RUN apt update
RUN apt install -y golang-go

# Install SP1
RUN curl -L https://sp1.succinct.xyz | bash && \
    ~/.sp1/bin/sp1up && \
    ~/.sp1/bin/cargo-prove prove --version

###############################################################################
#                                                                             #
#                               Server Builder                                #
#                                                                             #
###############################################################################
FROM rust-base AS server-builder

# Copy the entire workspace
COPY . .

# Build the proposer binary
RUN cargo build --release --bin sp1-tee-private-server

###############################################################################
#                                                                             #
#                              Fulfiller Builder                              #
#                                                                             #
###############################################################################
FROM rust-base AS fulfiller-builder

# Copy the entire workspace
COPY . .

# Build the proposer binary
RUN cargo build --release --bin sp1-tee-private-fulfiller

###############################################################################
#                                                                             #
#                               Base Runtime                                  #
#                                                                             #
###############################################################################
FROM ubuntu:24.04 as runtime

WORKDIR /app

# Install only necessary runtime dependencies
RUN apt-get update && apt-get install -y \
    curl \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH=/root/.cargo/bin:$PATH
RUN rustup install stable && rustup default stable

###############################################################################
#                                                                             #
#                              Server Runtime                                 #
#                                                                             #
###############################################################################
FROM runtime as server

# Copy the built proposer binary
COPY --from=server-builder /usr/src/app/target/release/sp1-tee-private-server /usr/local/bin/

# Set the command
CMD ["sp1-tee-private-server"]

###############################################################################
#                                                                             #
#                            Fulfiller Runtime                                #
#                                                                             #
###############################################################################
FROM runtime as fulfiller

# Copy the built proposer binary
COPY --from=fulfiller-builder /usr/src/app/target/release/sp1-tee-private-fulfiller /usr/local/bin/

# Set the command
CMD ["sp1-tee-private-fulfiller"]