# Application icon

`toolkit.svg` is the editable source for the approved filmstrip/play/keyframe
design. The four-part ring is orange; the timeline is neutral charcoal and
muted gray. There are no external corner accents. The icon belongs in the
native window and operating-system launchers, not beside the in-app heading.

The production artwork uses flat fills for consistent rendering at small sizes:
cream `#decba9`, orange `#b8662b`, charcoal `#181918`, timeline `#2b2b28`,
and muted timeline marks `#756d60`.

Run `python packaging/generate_icons.py` from the repository root to regenerate
the PNG, ICO and ICNS files. This authoring step needs Inkscape and Pillow.
Generated assets are committed; normal builds and end users need neither tool.

- Windows: the ICO is embedded into the executable and used by the installer.
- Slint: the 256px PNG supplies the native window icon where supported.
- macOS: ICNS uses the same mark on a charcoal rounded-square backing.
- Linux: PNG sizes and scalable SVG are installed into the hicolor icon theme
  by the optional per-user `install-desktop.sh` launcher installer.
