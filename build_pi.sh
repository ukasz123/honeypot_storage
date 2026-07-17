#!/bin/bash

# Exit on error
set -e

# Target architecture for Raspberry Pi (AArch64)
TARGET="aarch64-unknown-linux-gnu"
# Feature to use (defaulting to sqlite as requested)
FEATURES="sqlite"

echo "-------------------------------------------------------"
echo "🚀 Starting cross-compilation for $TARGET"
echo "🛠️  Using features: $FEATURES"
echo "-------------------------------------------------------"

# Run cross build
# We point to the custom config file using the --config flag if needed, 
# but cross looks in .cross/ or we can pass RUSTFLAGS.
# Since I created it in cross_configs/, we'll use RUSTFLAGS to ensure it picks up the paths.

RUSTFLAGS="-L /usr/aarch64-linux-gnu/lib/ -L /usr/lib/aarch64-linux-gnu/" \
cross build --release --target="$TARGET" --features "$FEATURES" --no-default-features

echo "-------------------------------------------------------"
echo "✅ Build completed successfully!"
echo "📂 Binary location: target/$TARGET/release/honeypot_storage"
echo "-------------------------------------------------------"
