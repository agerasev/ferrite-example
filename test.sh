#!/usr/bin/env bash

export EPICS_BASE=/home/agerasev/develop/binp/epics-base
export EPICS_CA_AUTO_ADDR_LIST=NO
export EPICS_CA_ADDR_LIST=127.0.0.1
export LD_LIBRARY_PATH=$LD_LIBRARY_PATH:$EPICS_BASE/lib/linux-x86_64

cd test && \
cargo run
