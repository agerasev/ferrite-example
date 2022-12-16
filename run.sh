#!/usr/bin/env bash

cd backend && \
cargo build && \
cd .. && \
make && \
cd iocBoot/iocExample && \
./st.cmd
