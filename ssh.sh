#!/bin/sh

set -e

HOST=$(lanipof '0c:dd:24:e5:40:fc')

echo "Forwarding 127.0.0.1:9000"
echo "Connecting to user@$HOST"

exec ssh \
  -o UserKnownHostsFile=/dev/null -o StrictHostKeyChecking=no \
  -L 127.0.0.1:9000:127.0.0.1:9000 \
   user@$HOST
