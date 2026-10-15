#!/bin/sh
set -eu

mkdir -p "$TRACEPULSE_DATA_DIR"
if [ ! -e "$TRACEPULSE_DATA_DIR/config.toml" ]; then
    cp /etc/tracepulse/config.toml "$TRACEPULSE_DATA_DIR/config.toml"
fi

exec "$@"