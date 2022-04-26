FROM rustlang/rust:nightly-bullseye-slim as builder

# DPDK prerequisites
RUN apt update
RUN apt install -y \
    clang clang-tools libclang-dev libc-dev libbsd-dev linux-headers-amd64 \
    python3 python3-pyelftools python3-pip ninja-build \
    numactl libnuma-dev zlib1g-dev libpcap-dev libbpf-dev libssl-dev \
    git cmake llvm-dev libnl-3-dev libnl-route-3-dev wget

# mlx prerequisites
RUN git clone https://github.com/linux-rdma/rdma-core.git
WORKDIR /rdma-core/build
RUN cmake -DNO_MAN_PAGES=1 -DNO_PYVERBS=1 -GNinja ..
RUN ninja install
WORKDIR /

# download
RUN wget https://git.dpdk.org/dpdk/snapshot/dpdk-22.03.tar.gz && tar xf dpdk-22.03.tar.gz

# build DPDK
RUN pip3 install meson
WORKDIR /dpdk-22.03/build
RUN meson ..
RUN ninja install

# bindgen
RUN cargo install bindgen

# build rust
WORKDIR /rust