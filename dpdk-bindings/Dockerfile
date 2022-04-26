FROM rustlang/rust:nightly-alpine as builder

# DPDK prerequisites
RUN apk --no-cache add \
    clang-dev clang-libs clang-static libc-dev bsd-compat-headers linux-headers \
    python3 py3-elftools meson ninja \
    libexecinfo-dev numactl-dev zlib-dev libpcap-dev libbpf-dev openssl-dev \
    alpine-sdk libpcap git cmake libnl3-dev llvm-dev libffi-dev ncurses-static

# mlx prerequisites
RUN git clone https://github.com/linux-rdma/rdma-core.git
WORKDIR /rdma-core/build
RUN cmake -DNO_MAN_PAGES=1 -DNO_PYVERBS=1 -GNinja ..
RUN ninja install
WORKDIR /

# download
RUN wget https://git.dpdk.org/dpdk/snapshot/dpdk-22.03.tar.gz && tar xf dpdk-22.03.tar.gz

# build DPDK
WORKDIR dpdk-22.03/build
RUN meson ..
RUN ninja install

# bindgen
RUN cargo install bindgen --no-default-features --features static,which-rustfmt,logging,clap

# build rust
WORKDIR /rust