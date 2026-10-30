# Records assets/demo.gif. Run from the repo root after `cargo build --release`: vhs assets/demo.tape
# Uses a sample hosts file in a temp dir, never the real /etc/hosts.

Output assets/demo.gif

Set Shell "bash"
Set FontSize 18
Set Width 1280
Set Height 620
Set Padding 16
Set Theme "Catppuccin Mocha"
Set TypingSpeed 90ms

Hide
Type "export PATH=$PWD/target/release:$PATH D=$(mktemp -d) && cp assets/demo-hosts $D/hosts && cd $D && export XDG_CONFIG_HOME=$D/config XDG_DATA_HOME=$D/data && clear"
Enter
Type "hostkeep --file hosts --lang en"
Enter
Sleep 1s
Show

Sleep 2s

# Enable a single entry
Down 5
Sleep 600ms
Space
Sleep 1.2s

# Turn a whole group on
Left
Sleep 400ms
Down 3
Sleep 800ms
Space
Sleep 1.5s

# Add an entry to that group
Right
Sleep 400ms
Type "a"
Sleep 800ms
Type "staging.example.com cdn.example.com"
Sleep 400ms
Down
Sleep 300ms
Type "new CDN"
Sleep 800ms
Enter
Sleep 1.5s

# Search across all groups
Left
Up 3
Right
Type "/"
Sleep 300ms
Type "corp"
Sleep 1.5s
Escape
Sleep 800ms

# Save with a diff preview
Type "s"
Sleep 3s
Enter
Sleep 2.5s

Hide
Type "q"
