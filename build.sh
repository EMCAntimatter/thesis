#!/bin/bash

env CFLAGS="-fuse-linker-plugin" CC_LD="gold" meson --reconfigure build
ninja -C build