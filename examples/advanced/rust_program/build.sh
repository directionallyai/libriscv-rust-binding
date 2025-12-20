#!/usr/bin/env sh
set -eu

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
UNAME_S="$(uname -s)"
if [ "$UNAME_S" = "Darwin" ]; then
	TARGET="${TARGET:-riscv64gc-unknown-linux-musl}"
else
	TARGET="${TARGET:-riscv64gc-unknown-linux-gnu}"
fi

zig_target_from_rust() {
	case "$1" in
		*riscv64gc-unknown-linux-gnu*)
			echo "riscv64-linux-gnu"
			;;
		*riscv64gc-unknown-linux-musl*)
			echo "riscv64-linux-musl"
			;;
		*)
			echo ""
			;;
	esac
}

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

	ZIG_TARGET="$(zig_target_from_rust "$TARGET")"
	if [ -z "$ZIG_TARGET" ]; then
		echo "unsupported TARGET=$TARGET for zig on macOS" >&2
		exit 1
	fi

	LINKER_SCRIPT="$SCRIPT_DIR/zig-linker.sh"
	cat > "$LINKER_SCRIPT" <<EOF
#!/usr/bin/env sh
exec "$ZIG" cc -target "$ZIG_TARGET" "\$@"
EOF
	chmod +x "$LINKER_SCRIPT"
	case "$TARGET" in
		*riscv64gc-unknown-linux-gnu*)
			export CARGO_TARGET_RISCV64GC_UNKNOWN_LINUX_GNU_LINKER="$LINKER_SCRIPT"
			;;
		*riscv64gc-unknown-linux-musl*)
			export CARGO_TARGET_RISCV64GC_UNKNOWN_LINUX_MUSL_LINKER="$LINKER_SCRIPT"
			;;
	esac
	export RUSTFLAGS="${RUSTFLAGS:-} -C target-feature=+crt-static -C link-self-contained=no"
else
	case "$TARGET" in
		riscv64gc-unknown-linux-gnu)
			LINKER="${LINKER:-riscv64-linux-gnu-gcc}"
			if ! command -v "$LINKER" >/dev/null 2>&1; then
				LINKER="riscv64-linux-gnu-gcc-12"
			fi
			export CARGO_TARGET_RISCV64GC_UNKNOWN_LINUX_GNU_LINKER="$LINKER"
			export RUSTFLAGS="${RUSTFLAGS:-} -C target-feature=+crt-static"
			;;
		riscv64gc-unknown-linux-musl)
			LINKER="${LINKER:-riscv64-linux-musl-gcc}"
			if ! command -v "$LINKER" >/dev/null 2>&1; then
				LINKER="riscv64-linux-gnu-gcc"
			fi
			export CARGO_TARGET_RISCV64GC_UNKNOWN_LINUX_MUSL_LINKER="$LINKER"
			export RUSTFLAGS="${RUSTFLAGS:-} -C target-feature=+crt-static"
			;;
	esac
fi

CARGO_CMD="cargo"
RUSTC_CMD=""
if command -v rustup >/dev/null 2>&1; then
	ACTIVE_TOOLCHAIN="$(rustup show active-toolchain | awk '{print $1}')"
	TOOLCHAIN="${RUSTUP_TOOLCHAIN:-$ACTIVE_TOOLCHAIN}"
	CARGO_CMD="$(rustup which cargo --toolchain "$TOOLCHAIN")"
	RUSTC_CMD="$(rustup which rustc --toolchain "$TOOLCHAIN")"
fi

if [ -n "$RUSTC_CMD" ]; then
	RUSTC="$RUSTC_CMD" "$CARGO_CMD" build --release --target "$TARGET" --manifest-path "$SCRIPT_DIR/Cargo.toml"
else
	"$CARGO_CMD" build --release --target "$TARGET" --manifest-path "$SCRIPT_DIR/Cargo.toml"
fi
