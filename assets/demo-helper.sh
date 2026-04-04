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

sleep 3

# Change 1: Update DATABASE_URL
cat > "$DEMO_DIR/.env" << 'EOF'
DATABASE_URL=postgres://prod-db:5432/myapp
API_KEY=sk-live-abc123
PORT=3000
DEBUG=false
REDIS_URL=redis://cache:6379
EOF

sleep 5

# Change 2: Modify PORT and add a new variable
cat > "$DEMO_DIR/.env" << 'EOF'
DATABASE_URL=postgres://prod-db:5432/myapp
API_KEY=sk-live-abc123
PORT=8080
DEBUG=false
REDIS_URL=redis://cache:6379
NEW_FEATURE_FLAG=enabled
EOF

sleep 5

# Change 3: Write invalid TOML (to show validation catching errors)
cat > "$DEMO_DIR/config.toml" << 'EOF'
[server
port = 3000
EOF

sleep 4

# Change 4: Fix the TOML (to show config diff)
cat > "$DEMO_DIR/config.toml" << 'EOF'
[server]
host = "0.0.0.0"
port = 3000
max_connections = 200
EOF
