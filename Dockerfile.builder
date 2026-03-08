FROM rust:bookworm

# Install Node.js
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - && \
    apt-get install -y nodejs

# Install Tauri prerequisites for Linux (and Windows cross-compilation tools if needed later)
RUN apt-get update && apt-get install -y \
    libwebkit2gtk-4.0-dev \
    libwebkit2gtk-4.1-dev \
    build-essential \
    curl \
    wget \
    file \
    libssl-dev \
    libgtk-3-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev \
    mingw-w64 \
    nsis

# Add rust windows target
RUN rustup target add x86_64-pc-windows-gnu

WORKDIR /app
