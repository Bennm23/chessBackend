#!/usr/bin/env bash
set -euo pipefail

# Name of the PGO training binary (the one above).
PGO_BIN="engine_pgo"

# Name of your real server binary that you run with Tokio + websockets.
# Replace this with your actual bin target, e.g. "server" or "backend".
REAL_ENGINE_BIN="chess_engine"

# Number of PGO runs (each run does many searches + self-play).
RUNS=1

if [[ "${1:-}" != "-x" ]]
then
	echo "==> Building instrumented PGO binary…"

	RUSTFLAGS="-Cprofile-generate=pgo -Cembed-bitcode=yes -Ccodegen-units=1" \
	    cargo build --release --bin "$PGO_BIN"

	echo "==> Running PGO training ($RUNS runs)…"

	for i in $(seq 1 "$RUNS"); do
	    echo "  -> Run $i / $RUNS"
	    LLVM_PROFILE_FILE="pgo-data/pgo-data-$i-%p.profraw" \
		"./target/release/$PGO_BIN"
	done

	echo "==> Merging profile data…"
fi

/home/bmelling/opt/LLVM-21.1.6-Linux-X64/bin/llvm-profdata merge -output=pgo-data.profdata pgo-data/pgo-data-*.profraw

echo "==> Building final optimized engine binary ($REAL_ENGINE_BIN)…"

PROFILE_FILE="$(pwd)/pgo-data.profdata"
	
RUSTFLAGS="-Cprofile-use=$PROFILE_FILE \
           -Cllvm-args=-pgo-warn-missing-function \
		   -Cembed-bitcode=yes \
           -Ccodegen-units=1" \
    cargo build --release --bin "$REAL_ENGINE_BIN"

echo "==> Done. Optimized binary: target/release/$REAL_ENGINE_BIN"
