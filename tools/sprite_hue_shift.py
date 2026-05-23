"""Hue-shift d'une arborescence de sprites PNG.

Cas d'usage initial : générer un octopus vert à partir du violet/magenta
existant en `assets/images/octopus/`. Le shift ne touche QUE les pixels
dont la teinte source tombe dans la plage `SOURCE_HUE_RANGE` (le violet
de l'octopus) — le casque cyan, les outlines noirs et les yeux restent
intacts.

Usage :
    python tools/sprite_hue_shift.py <input_dir> <output_dir>

Le script préserve la hiérarchie : chaque sous-dossier est reproduit
dans `output_dir`.
"""

from __future__ import annotations

import colorsys
import os
import sys
from pathlib import Path

from PIL import Image

# Plages de teintes source à shifter (degrés HSL, 0-360). On accepte deux
# plages distinctes pour couvrir le wrap-around : violet pur (250-360°) ET
# rose chaud / saumon (0-25°). Le casque cyan (~180°) et les yeux noirs
# ne tombent dans aucune plage → ils restent intacts.
SOURCE_HUE_RANGES = [(250.0, 360.0), (0.0, 25.0)]

# Décalage appliqué à la teinte (degrés). -180° transforme magenta → vert.
HUE_SHIFT = -180.0

# Seuil de saturation : sous ce seuil on ne touche pas (évite de teinter
# le gris foncé des outlines). Bas exprès pour attraper le rose pâle.
MIN_SATURATION = 0.05


def shift_pixel(r: int, g: int, b: int, a: int) -> tuple[int, int, int, int]:
    if a == 0:
        return (r, g, b, a)

    rn, gn, bn = r / 255.0, g / 255.0, b / 255.0
    h, l, s = colorsys.rgb_to_hls(rn, gn, bn)
    h_deg = h * 360.0

    if s < MIN_SATURATION:
        return (r, g, b, a)
    if not any(lo <= h_deg <= hi for (lo, hi) in SOURCE_HUE_RANGES):
        return (r, g, b, a)

    new_h_deg = (h_deg + HUE_SHIFT) % 360.0
    new_h = new_h_deg / 360.0
    rn2, gn2, bn2 = colorsys.hls_to_rgb(new_h, l, s)
    return (int(round(rn2 * 255)), int(round(gn2 * 255)), int(round(bn2 * 255)), a)


def process_file(in_path: Path, out_path: Path) -> None:
    img = Image.open(in_path).convert("RGBA")
    pixels = img.load()
    for y in range(img.height):
        for x in range(img.width):
            pixels[x, y] = shift_pixel(*pixels[x, y])
    out_path.parent.mkdir(parents=True, exist_ok=True)
    img.save(out_path)


def main() -> None:
    if len(sys.argv) != 3:
        print(f"Usage: python {sys.argv[0]} <input_dir> <output_dir>", file=sys.stderr)
        sys.exit(2)

    in_root = Path(sys.argv[1])
    out_root = Path(sys.argv[2])

    if not in_root.is_dir():
        print(f"Input dir not found: {in_root}", file=sys.stderr)
        sys.exit(1)

    count = 0
    for in_path in in_root.rglob("*.png"):
        rel = in_path.relative_to(in_root)
        out_path = out_root / rel
        process_file(in_path, out_path)
        count += 1
        print(f"  {rel}")

    print(f"\nProcessed {count} files into {out_root}")


if __name__ == "__main__":
    main()
