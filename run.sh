#!/bin/bash

if [ "$1" = "test" ]; then
    docker compose run --rm --user "$(id -u):$(id -g)" rust_dev cargo test -- --nocapture
elif [ "$1" = "release" ]; then
    shift
    docker compose run --rm --user "$(id -u):$(id -g)" rust_dev cargo run --release -- "$@"
else
    docker compose run --rm --user "$(id -u):$(id -g)" rust_dev cargo run -- "$@"
fi
