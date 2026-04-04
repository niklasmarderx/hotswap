#!/bin/bash
# Demo helper: modifies files in the background to trigger hotswap
# Used by the VHS demo tape
#
# Usage: ./demo-helper.sh <DEMO_DIR>

set -euo pipefail

DEMO_DIR="${1:?Usage: demo-helper.sh <DEMO_DIR>}"

if [ ! -d "$DEMO_DIR" ]; then
    echo "error: $DEMO_DIR is not a directory" >&2
    exit 1
fi

# Wait for hotswap to start and be ready
sleep 10

# --- Change 1: Update env vars (triggers env diff with masking) ---
cat > "$DEMO_DIR/.env" << 'EOF'
DATABASE_URL=postgres://prod-db:5432/myapp
API_KEY=sk-live-abc123
PORT=8080
DEBUG=false
REDIS_URL=redis://cache:6379
NEW_FEATURE_FLAG=enabled
EOF

# Wait for diff to display
sleep 8

# --- Change 2: Update config.toml values (triggers config diff) ---
cat > "$DEMO_DIR/config.toml" << 'EOF'
[server]
host = "0.0.0.0"
port = 8080
workers = 8
max_connections = 200
EOF
