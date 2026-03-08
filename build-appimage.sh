#!/bin/bash
set -euo pipefail

APP_NAME="Canopy"
BIN_NAME="canopy"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BUILD_DIR="$SCRIPT_DIR/appimage-build"
APPDIR="$BUILD_DIR/${APP_NAME}.AppDir"

echo "==> Building release binary..."
cargo build --release

echo "==> Preparing AppDir..."
rm -rf "$BUILD_DIR"
mkdir -p "$APPDIR/usr/bin"
mkdir -p "$APPDIR/usr/share/icons/hicolor/256x256/apps"

cp "$SCRIPT_DIR/target/release/$BIN_NAME" "$APPDIR/usr/bin/$BIN_NAME"
cp "$SCRIPT_DIR/canopy.desktop" "$APPDIR/$BIN_NAME.desktop"
cp "$SCRIPT_DIR/assets/canopy.png" "$APPDIR/$BIN_NAME.png"
cp "$SCRIPT_DIR/assets/canopy.png" "$APPDIR/usr/share/icons/hicolor/256x256/apps/$BIN_NAME.png"

# AppRun script
cat > "$APPDIR/AppRun" << 'APPRUN'
#!/bin/bash
SELF="$(readlink -f "$0")"
APPDIR="$(dirname "$SELF")"
exec "$APPDIR/usr/bin/canopy" "$@"
APPRUN
chmod +x "$APPDIR/AppRun"

# Download appimagetool if not available
APPIMAGETOOL="$BUILD_DIR/appimagetool"
if ! command -v appimagetool &>/dev/null; then
    if [ ! -f "$APPIMAGETOOL" ]; then
        echo "==> Downloading appimagetool..."
        ARCH="$(uname -m)"
        curl -fSL "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-${ARCH}.AppImage" -o "$APPIMAGETOOL"
        chmod +x "$APPIMAGETOOL"
    fi
else
    APPIMAGETOOL="appimagetool"
fi

echo "==> Building AppImage..."
ARCH="$(uname -m)" "$APPIMAGETOOL" "$APPDIR" "$SCRIPT_DIR/${APP_NAME}-${ARCH}.AppImage"

echo "==> Done: ${APP_NAME}-$(uname -m).AppImage"
