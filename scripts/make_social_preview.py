"""Compose docs/assets/social-preview.png (1280x640) from the real app icon and the captured
HUD pill (docs/assets/overlay.png). Run after scripts/capture-readme.ps1."""

from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter, ImageFont

ROOT = Path(__file__).resolve().parent.parent
W, H = 1280, 640
FONTS = Path("C:/Windows/Fonts")
INK, GRAY, LIGHT = (29, 29, 31), (110, 110, 115), (245, 245, 247)

img = Image.new("RGB", (W, H), LIGHT)
# One soft white highlight behind the content, no hard edges.
glow = Image.new("L", (W, H), 0)
ImageDraw.Draw(glow).ellipse((-160, -220, 900, 700), fill=255)
img.paste(Image.new("RGB", (W, H), (255, 255, 255)), (0, 0), glow.filter(ImageFilter.GaussianBlur(140)))

icon = Image.open(ROOT / "src-tauri/icons/128x128@2x.png").convert("RGBA").resize((150, 150), Image.LANCZOS)
shadow = Image.new("RGBA", (W, H), (0, 0, 0, 0))
ImageDraw.Draw(shadow).rounded_rectangle((104, 122, 236, 254), radius=32, fill=(0, 0, 0, 40))
img.paste(shadow.filter(ImageFilter.GaussianBlur(14)), (0, 0), shadow.filter(ImageFilter.GaussianBlur(14)))
img.paste(icon, (94, 108), icon)

draw = ImageDraw.Draw(img)
title = ImageFont.truetype(str(FONTS / "segoeuib.ttf"), 96)
body = ImageFont.truetype(str(FONTS / "segoeui.ttf"), 36)
small = ImageFont.truetype(str(FONTS / "segoeui.ttf"), 27)
draw.text((272, 120), "Voxora", font=title, fill=INK)
draw.text((100, 300), "Hold a key, speak, let go. Clean text in any app.", font=body, fill=INK)
draw.text((100, 352), "Whisper and a local LLM on your own machine. No account, no cloud.", font=body, fill=GRAY)

pill = Image.open(ROOT / "docs/assets/overlay.png").convert("RGB")
pw, ph = pill.size
pill = pill.crop((int(pw * 0.18), int(ph * 0.2), int(pw * 0.82), int(ph * 0.8)))
pill = pill.resize((int(pill.width * 1.3), int(pill.height * 1.3)), Image.LANCZOS)
mask = Image.new("L", pill.size, 0)
ImageDraw.Draw(mask).rounded_rectangle((4, 4, pill.width - 4, pill.height - 4), radius=pill.height // 2, fill=255)
img.paste(pill, (96, 452), mask.filter(ImageFilter.GaussianBlur(2)))
draw.text((96 + pill.width + 34, 452 + pill.height // 2 - 17), "Windows  ·  macOS beta  ·  open source, MIT",
          font=small, fill=GRAY)

out = ROOT / "docs/assets/social-preview.png"
img.save(out, optimize=True)
print(out, img.size, out.stat().st_size // 1024, "KB")
