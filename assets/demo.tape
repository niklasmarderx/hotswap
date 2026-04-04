# hotswap — Demo
# Record with: vhs assets/demo.tape

Output assets/demo.gif

Set Shell "bash"
Set FontSize 14
Set Width 960
Set Height 550
Set Theme "Catppuccin Mocha"
Set Padding 15
Set TypingSpeed 30ms
Set CursorBlink false

# === Setup (hidden) ===
Hide
Type "export DEMO_DIR=/tmp/demo && rm -rf $DEMO_DIR && bash assets/demo-setup.sh $DEMO_DIR"
Enter
Sleep 1s
Type "export PATH=$(pwd)/target/release:$PATH"
Enter
Sleep 200ms
# Start background helper early so its sleep aligns with the watcher start
Type "bash assets/demo-helper.sh $DEMO_DIR >/dev/null 2>&1 &"
Enter
Sleep 200ms
Type "clear"
Enter
Sleep 500ms
Show

# === Scene 1: hotswap init ===
Sleep 300ms
Type "hotswap init --output $DEMO_DIR/hotswap.toml"
Enter
Sleep 4s

# === Scene 2: Start watching ===
Type "hotswap --watch $DEMO_DIR/.env --watch $DEMO_DIR/config.toml --validate toml --diff-only --quiet"
Enter

# Helper writes .env at t=6s from its start. It started ~2s ago during setup.
# So .env change should arrive ~4s after this command.
# === Scene 3: Env diff ===
Sleep 8s

# === Scene 4: Config diff (helper writes config.toml 8s after .env change) ===
Sleep 10s

# === Exit ===
Ctrl+C
Sleep 800ms

Type "# Zero downtime. Zero drama."
Enter
Sleep 3s
