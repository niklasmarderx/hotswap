# hotswap — Professional Demo
# Record with: vhs assets/demo.tape

Output assets/demo.gif

Set Shell "bash"
Set FontSize 15
Set Width 950
Set Height 550
Set Theme "Catppuccin Mocha"
Set Padding 15
Set TypingSpeed 35ms
Set CursorBlink false

# --- Setup (hidden from viewer via speed) ---
Hide
Type "export DEMO_DIR=$(mktemp -d) && cp /dev/null $DEMO_DIR/.env && cp /dev/null $DEMO_DIR/config.toml"
Enter
Sleep 200ms
Type `cat > $DEMO_DIR/.env << 'EOF'
DATABASE_URL=postgres://localhost:5432/myapp
API_KEY=sk-test-1234567890
PORT=3000
DEBUG=true
REDIS_URL=redis://localhost:6379
EOF`
Enter
Sleep 200ms
Type `cat > $DEMO_DIR/config.toml << 'EOF'
[server]
host = "localhost"
port = 3000
max_connections = 100
EOF`
Enter
Sleep 200ms
Show

# --- Scene 1: Show current env ---
Type "cat $DEMO_DIR/.env"
Enter
Sleep 2s

# --- Scene 2: Start hotswap with diff-only ---
Type "# Watch .env and config.toml — validate TOML, show diffs"
Enter
Sleep 800ms

# Start the helper script in background
Hide
Type "bash assets/demo-helper.sh $DEMO_DIR &"
Enter
Sleep 200ms
Show

Type "hotswap --watch $DEMO_DIR/.env --watch $DEMO_DIR/config.toml --validate toml --diff-only --quiet"
Enter

# --- Wait for changes to happen ---
# Change 1: DATABASE_URL update (~3s)
Sleep 5s

# Change 2: PORT change + NEW_FEATURE_FLAG added (~5s)
Sleep 6s

# Change 3: Invalid TOML (~5s) — validation error shown
Sleep 5s

# Change 4: Fixed TOML (~4s) — validation passes
Sleep 5s

# --- Exit ---
Ctrl+C
Sleep 500ms

Type "# Zero downtime. Zero drama. github.com/niklasmarderx/hotswap"
Enter
Sleep 3s
