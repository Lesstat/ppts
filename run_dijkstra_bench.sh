#!/usr/bin/env sh

cargo bench --bench dijkstra_bench  --  --measurement-time 30 --warm-up-time 10
