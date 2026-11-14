#!/bin/sh
# Runs generate.sh, then installs the app icon for the dev launcher
# (~/.local/share/applications/freecal.desktop, `Icon=freecal-dev`; ticket 28).
set -eu
cd "$(dirname "$0")"

./generate.sh
hicolor="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor"
dest="$hicolor/512x512/apps"
mkdir -p "$dest"
cp app/512x512.png "$dest/freecal-dev.png"
# The icon caches (KDE's ~/.cache/icon-cache.kcache, GTK's icon-theme.cache)
# go stale only when an icon folder's time changes, which overwriting a file
# doesn't do.
touch "$dest" "$hicolor"
if command -v gtk-update-icon-cache >/dev/null; then
  gtk-update-icon-cache -q -t "$hicolor"
fi
echo "Installed $dest/freecal-dev.png"
