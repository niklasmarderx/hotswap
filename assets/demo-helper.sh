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

# --- Change 1: Update env vars (triggers env diff with masking) ---
sleep 3

cat > "$DEMO_DIR/.env" << 'EOF'
DATABASE_URL=postgres://prod-db:5432/myapp
API_KEY=sk-live-abc123
PORT=8080
DEBUG=false
REDIS_URL=redis://cache:6379
NEW_FEATURE_FLAG=enabled
EOF

# --- Change 2: Break config.toml (triggers validation error) ---
sleep 6

cat > "$DEMO_DIR/config.toml" << 'EOF'
[server
port = 3000
EOF

# --- Change 3: Fix config.toml with new values (triggers config diff) ---
sleep 5

cat > "$DEMO_DIR/config.toml" << 'EOF'
[server]
host = "0.0.0.0"
port = 8080
workers = 8
max_connections = 200
EOF
