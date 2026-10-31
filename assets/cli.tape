# Records assets/cli.gif. Run from the repo root after `cargo build --release`: vhs assets/cli.tape
# Uses a sample hosts file in a temp dir, never the real /etc/hosts.

Output assets/cli.gif

Set Shell "bash"
Set FontSize 18
Set Width 1280
Set Height 540
Set Padding 16
Set Theme "Catppuccin Mocha"
Set TypingSpeed 60ms

Hide
Type "export PATH=$PWD/target/release:$PATH D=$(mktemp -d) && cp assets/demo-hosts $D/hosts && cd $D"
Enter
Type "export XDG_CONFIG_HOME=$D/config XDG_DATA_HOME=$D/data && mkdir -p config/hostkeep"
Enter
Type `printf 'hosts_path = "hosts"\nlanguage = "en"\n' > config/hostkeep/config.toml`
Enter
Type `PS1='\[\e[35m\]$\[\e[0m\] ' && clear`
Enter
Sleep 500ms
Show

Type "hostkeep groups"
Sleep 300ms
Enter
Sleep 2s

Type "hostkeep on Staging --dry-run"
Sleep 300ms
Enter
Sleep 3.5s

Hide
Type "clear"
Enter
Show

Type "hostkeep on Staging"
Sleep 300ms
Enter
Sleep 1.5s

Type "hostkeep on kafka.local"
Sleep 300ms
Enter
Sleep 1.5s

Type 'hostkeep add 127.0.0.1 api.local -g Docker -c "new backend"'
Sleep 300ms
Enter
Sleep 1.5s

Type "hostkeep groups"
Sleep 300ms
Enter
Sleep 3s
