#!/bin/bash
# Demo setup: creates the initial files for the VHS demo
# Usage: ./demo-setup.sh <DEMO_DIR>

set -euo pipefail

DEMO_DIR="${1:?Usage: demo-setup.sh <DEMO_DIR>}"
mkdir -p "$DEMO_DIR"

cat > "$DEMO_DIR/.env" << 'EOF'
DATABASE_URL=postgres://localhost:5432/myapp
API_KEY=sk-test-1234567890
PORT=3000
DEBUG=true
REDIS_URL=redis://localhost:6379
EOF

cat > "$DEMO_DIR/config.toml" << 'EOF'
[server]
host = "localhost"
port = 3000
max_connections = 100
EOF
