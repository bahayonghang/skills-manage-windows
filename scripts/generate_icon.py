#!/usr/bin/env python3
"""Generate skills-manage app icon as 1024x1024 PNG with macOS styling.

Design: "Skill Blocks" — 3 colorful blocks in L-shape + sparkle.
Follows macOS HIG: 824x824 squircle in 1024 canvas, with drop shadow
and subtle top highlight for 3D depth (matching ccmate's polish level).
"""

from PIL import Image, ImageChops, ImageDraw, ImageFilter
import math

SIZE = 1024

# macOS icon grid: body is ~824x824 centered in 1024, room for shadow
SQ_SIZE = 824
SQ_RADIUS = 180
SQ_X = (SIZE - SQ_SIZE) // 2           # 100
SQ_Y = (SIZE - SQ_SIZE) // 2 - 6       # 94 — shift up a bit, shadow extends down

# Shadow
SHADOW_OFFSET_Y = 12
SHADOW_BLUR = 22

# Catppuccin Mocha
BASE = (30, 30, 46)
MANTLE = (24, 24, 37)

# Block colors
LAVENDER = (180, 190, 254)  # #b4befe
MAUVE = (203, 166, 247)     # #cba6f7
BLUE = (137, 180, 250)      # #89b4fa
PEACH = (250, 179, 135)     # #fab387

# Block geometry — sized to ~46% of squircle (similar to ccmate's cup proportion)
BLOCK_SIZE = 175
BLOCK_GAP = 30
BLOCK_RADIUS = 36


def mix(c1, c2, t):
    """Lerp two RGB tuples."""
    return tuple(int(a + (b - a) * t) for a, b in zip(c1, c2))


def star4(draw, cx, cy, outer, inner, fill):
    """4-pointed star."""
    pts = []
    for i in range(8):
        a = math.pi / 4 * i - math.pi / 2
        r = outer if i % 2 == 0 else inner
        pts.append((cx + r * math.cos(a), cy + r * math.sin(a)))
    draw.polygon(pts, fill=fill)


def main():
    # ══════════════════════════════════════════════════════════════════════
    # 1. Drop shadow
    # ══════════════════════════════════════════════════════════════════════
    shadow = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    ImageDraw.Draw(shadow).rounded_rectangle(
        (SQ_X, SQ_Y + SHADOW_OFFSET_Y,
         SQ_X + SQ_SIZE, SQ_Y + SQ_SIZE + SHADOW_OFFSET_Y),
        radius=SQ_RADIUS, fill=(0, 0, 0, 75),
    )
    shadow = shadow.filter(ImageFilter.GaussianBlur(radius=SHADOW_BLUR))

    # ══════════════════════════════════════════════════════════════════════
    # 2. Main icon
    # ══════════════════════════════════════════════════════════════════════
    icon = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    d = ImageDraw.Draw(icon)

    # Squircle body
    sq = (SQ_X, SQ_Y, SQ_X + SQ_SIZE, SQ_Y + SQ_SIZE)
    d.rounded_rectangle(sq, radius=SQ_RADIUS, fill=BASE)

    # Thin outer edge (1px lighter ring for definition against dark backgrounds)
    d.rounded_rectangle(
        (SQ_X, SQ_Y, SQ_X + SQ_SIZE, SQ_Y + SQ_SIZE),
        radius=SQ_RADIUS,
        outline=(*mix(BASE, (255, 255, 255), 0.12), 100),
        width=2,
    )

    # Inner depth ring
    d.rounded_rectangle(
        (SQ_X + 3, SQ_Y + 3, SQ_X + SQ_SIZE - 3, SQ_Y + SQ_SIZE - 3),
        radius=SQ_RADIUS - 2, fill=MANTLE,
    )
    d.rounded_rectangle(
        (SQ_X + 6, SQ_Y + 6, SQ_X + SQ_SIZE - 6, SQ_Y + SQ_SIZE - 6),
        radius=SQ_RADIUS - 3, fill=BASE,
    )

    # ── Top highlight gradient (lit from above) ──────────────────────────
    hl = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    hl_d = ImageDraw.Draw(hl)
    grad_h = SQ_SIZE // 3
    for i in range(grad_h):
        alpha = int(18 * (1 - i / grad_h) ** 2)
        if alpha < 1:
            break
        y = SQ_Y + 6 + i
        hl_d.line([(SQ_X + 6, y), (SQ_X + SQ_SIZE - 6, y)],
                  fill=(255, 255, 255, alpha))
    # Clip highlight to squircle shape: use the squircle as a mask
    mask = Image.new("L", (SIZE, SIZE), 0)
    ImageDraw.Draw(mask).rounded_rectangle(
        (SQ_X + 6, SQ_Y + 6, SQ_X + SQ_SIZE - 6, SQ_Y + SQ_SIZE - 6),
        radius=SQ_RADIUS - 3, fill=255,
    )
    hl.putalpha(ImageChops.darker(hl.split()[3], mask))
    icon = Image.alpha_composite(icon, hl)
    d = ImageDraw.Draw(icon)

    # ══════════════════════════════════════════════════════════════════════
    # 3. Blocks
    # ══════════════════════════════════════════════════════════════════════
    grid_w = BLOCK_SIZE * 2 + BLOCK_GAP  # 380
    grid_h = grid_w
    ox = SQ_X + (SQ_SIZE - grid_w) // 2
    oy = SQ_Y + (SQ_SIZE - grid_h) // 2

    blocks = [
        (ox, oy, LAVENDER),                                    # top-left
        (ox, oy + BLOCK_SIZE + BLOCK_GAP, MAUVE),              # bottom-left
        (ox + BLOCK_SIZE + BLOCK_GAP, oy + BLOCK_SIZE + BLOCK_GAP, BLUE),  # bottom-right
    ]

    # Block drop shadows
    bs = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    bs_d = ImageDraw.Draw(bs)
    for bx, by, _ in blocks:
        bs_d.rounded_rectangle(
            (bx + 2, by + 5, bx + BLOCK_SIZE + 2, by + BLOCK_SIZE + 5),
            radius=BLOCK_RADIUS, fill=(0, 0, 0, 45),
        )
    bs = bs.filter(ImageFilter.GaussianBlur(radius=8))
    icon = Image.alpha_composite(icon, bs)
    d = ImageDraw.Draw(icon)

    # Color glow behind blocks
    glow = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    gd = ImageDraw.Draw(glow)
    for bx, by, c in blocks:
        for sp in range(28, 0, -2):
            a = int(12 * (1 - sp / 28))
            gd.rounded_rectangle(
                (bx - sp, by - sp, bx + BLOCK_SIZE + sp, by + BLOCK_SIZE + sp),
                radius=BLOCK_RADIUS + sp // 2, fill=(*c, a),
            )
    icon = Image.alpha_composite(icon, glow)
    d = ImageDraw.Draw(icon)

    # Draw blocks with highlight strip
    for bx, by, c in blocks:
        d.rounded_rectangle(
            (bx, by, bx + BLOCK_SIZE, by + BLOCK_SIZE),
            radius=BLOCK_RADIUS, fill=c,
        )
        # Top highlight
        hl_c = mix(c, (255, 255, 255), 0.3)
        d.rounded_rectangle(
            (bx + 5, by + 5, bx + BLOCK_SIZE - 5, by + 11),
            radius=3, fill=(*hl_c, 60),
        )

    # ══════════════════════════════════════════════════════════════════════
    # 4. Sparkle (warm Peach in empty top-right cell)
    # ══════════════════════════════════════════════════════════════════════
    scx = ox + BLOCK_SIZE + BLOCK_GAP + BLOCK_SIZE // 2
    scy = oy + BLOCK_SIZE // 2
    so, si = 42, 14

    sg = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    sg_d = ImageDraw.Draw(sg)
    for r in range(so + 35, so, -3):
        a = int(22 * (1 - (r - so) / 35))
        sg_d.ellipse([scx - r, scy - r, scx + r, scy + r], fill=(*PEACH, a))
    icon = Image.alpha_composite(icon, sg)
    d = ImageDraw.Draw(icon)

    star4(d, scx, scy, so, si, PEACH)
    star4(d, scx - 50, scy - 42, 13, 4, mix(PEACH, BASE, 0.15))
    star4(d, scx + 46, scy + 38, 11, 4, mix(PEACH, BASE, 0.15))

    # ══════════════════════════════════════════════════════════════════════
    # 5. Composite shadow + icon, save RGBA
    # ══════════════════════════════════════════════════════════════════════
    result = Image.alpha_composite(shadow, icon)

    out = "src-tauri/icons/icon-source.png"
    result.save(out, "PNG")
    print(f"Icon saved to {out} ({SIZE}x{SIZE})")


if __name__ == "__main__":
    main()
