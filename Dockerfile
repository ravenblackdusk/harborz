FROM arm64v8/debian:unstable
RUN apt-get update && apt-get install -y rustc build-essential libadwaita-1-dev libsqlite3-dev \
    libgstreamer-plugins-bad1.0-dev libproxy-dev libssl-dev
RUN apt-get update && apt-get install -y libdbus-1-dev pkg-config
ENTRYPOINT ["cargo", "build", "--release", "-Z", "sparse-registry"]
