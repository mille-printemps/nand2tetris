#!/usr/bin/env bash
set -euo pipefail

# Compiles a Jack program all the way to Hack machine code, chaining the
# three tools below so you don't have to cd into each one:
#   .jack -> (compiler) -> .vm -+
#                                +-> (vm) -> .asm -> (asm) -> .hack
#            tools/OS/*.vm -----+
#
# The OS's compiled .vm files (Math, String, Array, Output, Screen,
# Keyboard, Memory, Sys) are staged alongside your program's .vm files
# before translation: the bootstrap code always emits `call Sys.init 0`,
# and Sys.init calls Main.main, so without the OS there's nothing for the
# bootstrap to jump to and the .hack would be incomplete.
#
# Usage: ./run-jack.sh projects/play/MyGame

cd "$(dirname "$0")"

if [ $# -ne 1 ]; then
    echo "Usage: $0 <folder containing .jack files>" >&2
    exit 1
fi

dir="$1"
name="$(basename "$dir")"
build_dir="$dir/build/$name"

echo "==> compiling $dir"
cargo run --release -p compiler -- "$dir"

echo "==> staging program + OS VM code in $build_dir"
rm -rf "$build_dir"
mkdir -p "$build_dir"
cp "$dir"/*.vm "$build_dir"/
cp tools/OS/*.vm "$build_dir"/

echo "==> translating VM code"
cargo run --release -p vm -- "$build_dir"

echo "==> assembling"
cargo run --release -p asm -- "$build_dir/$name.asm"

echo
echo "Done: $build_dir/$name.hack"
echo "Run the VM code directly: tools/VMEmulator.sh"
echo "Run it interactively: tools/CPUEmulator.sh"
