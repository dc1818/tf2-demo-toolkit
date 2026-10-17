"""Stage the Linux icon theme and optional per-user launcher installer."""
from pathlib import Path
import shutil

APP_ID = "com.dc1818.tf2-demo-toolkit"


def stage_desktop_assets(root: Path, branding: Path):
    for size in (16, 24, 32, 48, 64, 128, 256, 512):
        target = root / "share/icons/hicolor" / f"{size}x{size}" / "apps"
        target.mkdir(parents=True, exist_ok=True)
        shutil.copy2(branding / f"toolkit-{size}.png", target / f"{APP_ID}.png")
    target = root / "share/icons/hicolor/scalable/apps"
    target.mkdir(parents=True, exist_ok=True)
    shutil.copy2(branding / "toolkit.svg", target / f"{APP_ID}.svg")
    shutil.copy2(Path(__file__).with_name("install-desktop.sh"), root / "install-desktop.sh")
    (root / "install-desktop.sh").chmod(0o755)
