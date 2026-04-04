#!/bin/bash
# Demo helper: modifies files in the background to trigger hotswap
# Used by the VHS demo tape

DEMO_DIR="$1"

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

# Change 2: Add a new variable
cat > "$DEMO_DIR/.env" << 'EOF'
DATABASE_URL=postgres://prod-db:5432/myapp
API_KEY=sk-live-abc123
PORT=8080
DEBUG=false
REDIS_URL=redis://cache:6379
NEW_FEATURE_FLAG=enabled
EOF

sleep 5

# Change 3: Write invalid TOML (to show validation)
cat > "$DEMO_DIR/config.toml" << 'EOF'
[server
port = 3000
EOF

sleep 4

# Change 4: Fix the TOML
cat > "$DEMO_DIR/config.toml" << 'EOF'
[server]
host = "0.0.0.0"
port = 3000
max_connections = 200
EOF
