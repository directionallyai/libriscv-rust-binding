#!/usr/bin/env sh
set -eu

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
UNAME_S="$(uname -s)"
if [ "$UNAME_S" = "Darwin" ]; then
	TARGET="${TARGET:-riscv64-linux-musl}"
else
	TARGET="${TARGET:-riscv64-linux-gnu}"
fi

if [ "$UNAME_S" = "Darwin" ]; then
	ZIG="${ZIG:-zig}"
	if ! command -v "$ZIG" >/dev/null 2>&1; then
		echo "zig not found; install zig or set ZIG=/path/to/zig" >&2
		exit 1
	fi
	ZIG_LOCAL_CACHE_DIR="${ZIG_LOCAL_CACHE_DIR:-$SCRIPT_DIR/.zig-cache}"
	ZIG_GLOBAL_CACHE_DIR="${ZIG_GLOBAL_CACHE_DIR:-$SCRIPT_DIR/.zig-cache/global}"
	mkdir -p "$ZIG_LOCAL_CACHE_DIR" "$ZIG_GLOBAL_CACHE_DIR"
	export ZIG_LOCAL_CACHE_DIR ZIG_GLOBAL_CACHE_DIR
	"$ZIG" cc -target "$TARGET" -static -O2 "$SCRIPT_DIR/micro.c" -o "$SCRIPT_DIR/micro"
else
	CC="${CC:-riscv64-linux-gnu-gcc-12}"
	if ! command -v "$CC" >/dev/null 2>&1; then
		CC="riscv64-linux-gnu-gcc"
	fi
	"$CC" -static -O2 "$SCRIPT_DIR/micro.c" -o "$SCRIPT_DIR/micro"
fi
