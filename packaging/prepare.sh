#!/usr/bin/env bash
# Fetches the host icon from its one source, fs-icons (context apps), at a pinned commit and checks it. The packaging
# (deb, rpm, AppImage, Windows) uses the file next to this script; it is not kept in this repository (no copy).
set -euo pipefail
cd "$(dirname "$0")"
REV=ccea0f88fcd022ea53aa0ba037cfa9d40a045beb
SHA=b8f00e19dd4c9549c3a617aaa5a664ef0bb66518148014f3689386fd6507c431
ICON=net.freesynergy.Starter.svg
curl -fsSL -o "$ICON" "https://raw.githubusercontent.com/FreeSynergy-Net/fs-icons/$REV/sets/freesynergy-theme/scalable/apps/$ICON"
echo "$SHA  $ICON" | sha256sum -c -
