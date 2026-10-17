#!/bin/sh
# Run after extracting the portable package to its permanent location.
set -eu
HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
DATA_HOME=${XDG_DATA_HOME:-"$HOME/.local/share"}
APP_ID=com.dc1818.tf2-demo-toolkit
case "$HERE$DATA_HOME" in
    *'
'*) echo 'Launcher paths must not contain newlines.' >&2; exit 1 ;;
esac
if [ ! -x "$HERE/TF2_Demo_Toolkit.sh" ]; then
    echo 'Keep this script beside TF2_Demo_Toolkit.sh in the extracted package.' >&2
    exit 1
fi
mkdir -p "$DATA_HOME/applications" "$DATA_HOME/icons"
cp -R "$HERE/share/icons/hicolor" "$DATA_HOME/icons/"
# Desktop Exec fields have both string escaping and quoted-argument escaping.
# Percent is doubled because desktop launchers interpret % field codes.
EXEC_PATH=$(printf '%s' "$HERE/TF2_Demo_Toolkit.sh" | sed 's/\\/\\\\\\\\/g; s/"/\\\\"/g; s/`/\\\\`/g; s/\$/\\\\$/g; s/%/%%/g')
cat > "$DATA_HOME/applications/$APP_ID.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=TF2 Demo Toolkit
Comment=Analyze TF2 demos and record highlights
Exec=/bin/sh "$EXEC_PATH"
Icon=$APP_ID
Terminal=false
Categories=AudioVideo;Video;
StartupWMClass=$APP_ID
EOF
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$DATA_HOME/applications" || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t "$DATA_HOME/icons/hicolor" >/dev/null 2>&1 || true
fi
printf '%s\n' 'Installed TF2 Demo Toolkit in your application menu.'
printf '%s\n' 'If you move this folder, run install-desktop.sh again.'
