#!/bin/bash

# Generate completions for these binaries and shells:

# ./target/debug/precachedctl completions bash > support/shell/completions/precachedctl.bash-completion
# ./target/debug/precachedctl completions zsh > support/shell/completions/precachedctl.zsh-completion
# ./target/debug/precachedctl completions fish > support/shell/completions/precachedctl.fish-completion
# ./target/debug/precachedctl completions powershell > support/shell/completions/precachedctl.powershell-completion

# ./target/debug/precachedtop completions bash > support/shell/completions/precachedtop.bash-completion
# ./target/debug/precachedtop completions zsh > support/shell/completions/precachedtop.zsh-completion
# ./target/debug/precachedtop completions fish > support/shell/completions/precachedtop.fish-completion
# ./target/debug/precachedtop completions powershell > support/shell/completions/precachedtop.powershell-completion

# ./target/debug/rulesctl completions bash > support/shell/completions/rulesctl.bash-completion
# ./target/debug/rulesctl completions zsh > support/shell/completions/rulesctl.zsh-completion
# ./target/debug/rulesctl completions fish > support/shell/completions/rulesctl.fish-completion
# ./target/debug/rulesctl completions powershell > support/shell/completions/rulesctl.powershell-completion

# ./target/debug/iotracectl completions bash > support/shell/completions/iotracectl.bash-completion
# ./target/debug/iotracectl completions zsh > support/shell/completions/iotracectl.zsh-completion
# ./target/debug/iotracectl completions fish > support/shell/completions/iotracectl.fish-completion
# ./target/debug/iotracectl completions powershell > support/shell/completions/iotracectl.powershell-completion

# ./target/debug/precached-debugtool completions bash > support/shell/completions/precached-debugtool.bash-completion
# ./target/debug/precached-debugtool completions zsh > support/shell/completions/precached-debugtool.zsh-completion
# ./target/debug/precached-debugtool completions fish > support/shell/completions/precached-debugtool.fish-completion
# ./target/debug/precached-debugtool completions powershell > support/shell/completions/precached-debugtool.powershell-completion

function gen_completions {
    ./target/debug/"$1" "completions" "bash" > "support/shell/completions/$1.bash-completion"
    ./target/debug/"$1" "completions" "zsh" > "support/shell/completions/$1.zsh-completion"
    ./target/debug/"$1" "completions" "fish" > "support/shell/completions/$1.fish-completion"
    ./target/debug/"$1" "completions" "powershell" > "support/shell/completions/$1.powershell-completion"
}

gen_completions "precachedctl"
gen_completions "precachedtop"
gen_completions "rulesctl"
gen_completions "iotracectl"
gen_completions "precached-debugtool"

exit 0