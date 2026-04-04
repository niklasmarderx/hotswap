# hotswap — Demo
# Record with: vhs assets/demo.tape

Output assets/demo.gif

Set Shell "bash"
Set FontSize 15
Set Width 960
Set Height 560
Set Theme "Catppuccin Mocha"
Set Padding 15
Set TypingSpeed 35ms
Set CursorBlink false

# === Setup (hidden — runs but not recorded) ===
Hide
Type "export DEMO_DIR=$(mktemp -d) && bash assets/demo-setup.sh $DEMO_DIR && cargo build --quiet 2>/dev/null"
Enter
Sleep 10s
Type "clear"
Enter
Sleep 500ms
Show

# === Scene 1: hotswap init ===
Sleep 500ms
Type "hotswap init --output $DEMO_DIR/hotswap.toml"
Enter
Sleep 3s

# === Scene 2: Start watching with validation ===
Sleep 500ms
Type "# Watch .env + config.toml — validate, diff, zero downtime"
Enter
Sleep 1s

# Start the background helper (hidden from recording)
Hide
Type "bash assets/demo-helper.sh $DEMO_DIR &"
Enter
Sleep 200ms
Type "clear"
Enter
Sleep 500ms
Show

Sleep 300ms
Type "hotswap --watch $DEMO_DIR/.env --watch $DEMO_DIR/config.toml --validate toml --diff-only"
Enter

# === Scene 3: Env diff (helper modifies .env after ~3s) ===
Sleep 7s

# === Scene 4: Invalid TOML (helper breaks config.toml after ~6s) ===
Sleep 8s

# === Scene 5: Config diff (helper fixes config.toml after ~5s) ===
Sleep 7s

# === Exit ===
Ctrl+C
Sleep 1s

Type "# Zero downtime. Zero drama."
Enter
Sleep 3s
