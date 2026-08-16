#!/usr/bin/env bash
set -euo pipefail

# Compiles a Jack program all the way to Hack machine code,
# chaining the three tools below so you don't have to cd into each one:
#   .jack -> (compiler) -> .vm -+
#                                +-> (vm) -> .asm -> (asm) -> .hack
#            tools/OS/*.vm -----+    (only if tools/OS exists)
#
# When present, the OS's compiled .vm files are staged alongside your
# program's .vm files before translation: the bootstrap code always emits
# 'call Sys.init 0', and Sys.init calls Main.main.

cd "$(dirname "$0")"

if [ $# -ne 1 ]; then
    echo "Usage: $0 <folder containing .jack files>" >&2
    exit 1
fi

dir="$1"
name="$(basename "$dir")"

echo "==> compiling $dir"
cargo run --release -p compiler -- "$dir"

if [ ! -d tools/OS ]; then
    echo
    echo "tools/OS not found — stopping after compiling to .vm."
    echo "Install the course tools into tools/ to translate and assemble."
    exit 0
fi

build_dir="$dir/build/$name"

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
