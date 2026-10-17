"""Regenerate committed native icons. Requires Inkscape and Pillow (builds do not)."""
from io import BytesIO
from pathlib import Path
import shutil
import struct
import subprocess
import tempfile
import xml.etree.ElementTree as ET

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / "app/ui/assets/branding"
SIZES = (16, 20, 24, 32, 40, 48, 64, 128, 256, 512)


def render(source, output, size):
    subprocess.run([
        "inkscape", str(source), "--export-type=png",
        f"--export-filename={output}", f"--export-width={size}",
        f"--export-height={size}",
    ], check=True, stdout=subprocess.DEVNULL)


def main():
    if not shutil.which("inkscape"):
        raise SystemExit("Install Inkscape to regenerate icons.")
    source = ASSETS / "toolkit.svg"
    for size in SIZES:
        render(source, ASSETS / f"toolkit-{size}.png", size)

    # ICO stores an individually rendered PNG for each Windows target size.
    # This avoids resizing a single large bitmap in Explorer or the title bar.
    sizes = [size for size in SIZES if size <= 256]
    payloads = [(ASSETS / f"toolkit-{size}.png").read_bytes() for size in sizes]
    offset = 6 + 16 * len(sizes)
    directory = bytearray(struct.pack("<HHH", 0, 1, len(sizes)))
    for size, payload in zip(sizes, payloads):
        directory += struct.pack("<BBBBHHII", size % 256, size % 256, 0, 0,
                                 1, 32, len(payload), offset)
        offset += len(payload)
    (ASSETS / "toolkit.ico").write_bytes(directory + b"".join(payloads))

    # macOS gets the same mark on a charcoal rounded-square backing.
    svg = ET.parse(source).getroot()
    body = "".join(ET.tostring(child, encoding="unicode") for child in svg)
    mac_svg = ('<svg xmlns="http://www.w3.org/2000/svg" width="1024" '
               'height="1024" viewBox="0 0 512 512">'
               '<rect x="16" y="16" width="480" height="480" rx="108" '
               'fill="#24231f"/>'
               '<g transform="translate(38.4 38.4) scale(.85)">'
               + body + '</g></svg>')
    with tempfile.TemporaryDirectory() as temp:
        path = Path(temp) / "macos.svg"
        path.write_text(mac_svg, encoding="utf-8")
        render(path, ASSETS / "toolkit-macos-1024.png", 1024)
    # PNG-backed ICNS representations, including Retina resolutions.
    chunks = []
    with Image.open(ASSETS / "toolkit-macos-1024.png") as master:
        for kind, size in [(b"icp4", 16), (b"icp5", 32), (b"icp6", 64),
                           (b"ic07", 128), (b"ic08", 256), (b"ic09", 512),
                           (b"ic10", 1024), (b"ic11", 32), (b"ic12", 64),
                           (b"ic13", 256), (b"ic14", 512)]:
            buffer = BytesIO()
            master.resize((size, size), Image.Resampling.LANCZOS).save(buffer, "PNG")
            data = buffer.getvalue()
            chunks.append(kind + struct.pack(">I", len(data) + 8) + data)
    data = b"".join(chunks)
    (ASSETS / "toolkit.icns").write_bytes(b"icns" + struct.pack(">I", len(data) + 8) + data)
    print(f"Generated native icons in {ASSETS}")


if __name__ == "__main__":
    main()
