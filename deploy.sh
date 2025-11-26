#!/bin/bash
set -e  # Exit on error

# === CONFIGURATION ===
HOST="deploy@97.107.134.18"
BASE="/var/www/chess-backend"
REL="release_$(date +%Y%m%d%H%M%S)"
BINARY_NAME="chess_engine"
LOCAL_BUILD_PATH="./target/release/$BINARY_NAME"

# === STEP 1: Build backend ===
echo "Building Rust backend..."
cargo build --release

if [ ! -f "$LOCAL_BUILD_PATH" ]; then
    echo "❌ Build failed: binary not found at $LOCAL_BUILD_PATH"
    exit 1
fi

# === STEP 2: Create release directory on server ===
echo "Creating release dir on server..."
ssh $HOST "mkdir -p $BASE/releases/$REL"

# === STEP 3: Upload binary ===
echo "Uploading backend binary..."
rsync -avz "$LOCAL_BUILD_PATH" "$HOST:$BASE/releases/$REL/"

# === STEP 4: Set permissions ===
ssh $HOST "chmod +x $BASE/releases/$REL/$BINARY_NAME"

# === STEP 5: Switch current symlink ===
echo "Updating current symlink..."
ssh $HOST "sudo ln -sfn $BASE/releases/$REL $BASE/current"

# === STEP 6: Upload Book ===
echo "Uploading opening book..."
rsync -avz "./book.ron" "$HOST:/home/deploy/book.ron"

# === STEP 7: Restart backend service ===
echo "Restarting backend systemd service..."
ssh $HOST "sudo systemctl restart chess-backend"

# === DONE ===
echo ""
echo "---------------------------------------"
echo " Backend deployed successfully!"
echo " Release: $REL"
echo "---------------------------------------"

