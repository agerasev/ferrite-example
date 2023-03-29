#!/usr/bin/env bash

source scripts/env.sh

./scripts/build.sh && \
cd iocBoot/iocExample && \
./st.cmd
