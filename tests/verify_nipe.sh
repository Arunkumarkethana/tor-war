#!/bin/bash
set -e

echo "Starting Nipe verification..."

# 1. Check if binary runs
nipe --version

# 2. Start Nipe (background)
# Note: In Docker, we can't easily modify the host firewall or use systemd.
# We will verify that the process starts and Tor bootstraps.
# We assume the container has NET_ADMIN capability.

# Create directory manually since we aren't using the installer here
mkdir -p /var/lib/nipe/tor-data

echo "Running start command..."
nipe start &

pid=$!
sleep 15

# Check status
nipe status

# Verify Tor process is running
if pgrep -x "tor" > /dev/null; then
    echo "SUCCESS: Tor is running."
else
    echo "FAILURE: Tor is not running."
    exit 1
fi

echo "Stopping Nipe..."
nipe stop
