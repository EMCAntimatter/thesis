# syntax = docker/dockerfile:1.2
FROM rustlang/rust:nightly-bullseye-slim as builder

# DPDK prerequisites
RUN apt update
RUN apt install -y \
    clang clang-tools libclang-dev libc-dev libbsd-dev linux-headers-amd64 \
    python3 python3-pyelftools python3-pip ninja-build \
    numactl zlib1g-dev libpcap-dev libbpf-dev libssl-dev \
    git cmake llvm-dev libnl-3-dev libnl-route-3-dev wget \
    ccache libnuma-dev libpci-dev clangd babeltrace

ENV CFLAGS="-g3 -march=native -mtune=native -gdwarf -ggdb3"
ENV CCACHE_DIR=/ccache

# mlx prerequisites
RUN git clone https://github.com/linux-rdma/rdma-core.git
WORKDIR /rdma-core/build
RUN cmake -DNO_MAN_PAGES=1 -DNO_PYVERBS=1 -GNinja ..
RUN ninja install
WORKDIR /

# build DPDK
RUN pip3 install meson

RUN wget https://git.dpdk.org/dpdk/snapshot/dpdk-22.03.tar.gz && tar xf dpdk-22.03.tar.gz
WORKDIR /dpdk-22.03/
RUN meson -Dtests=false -Dc_std=c18 /dpdk-22.03/build

# download
RUN --mount=type=cache,target=/dpdk-22.03 \
    --mount=type=cache,target=/ccache \
    ninja -j $(expr $(nproc) / 2 ) -C /dpdk-22.03/build install

RUN ldconfig

# bindgen
RUN --mount=type=cache,target=/usr/local/cargo/registry \
		--mount=type=cache,target=/usr/local/cargo/git \
		--mount=type=cache,target=/usr/local/rustup \
		set -eux; \
        rustup install nightly ; \
        rustup default nightly ; \
        cargo install bindgen

# build rust
WORKDIR /rust
