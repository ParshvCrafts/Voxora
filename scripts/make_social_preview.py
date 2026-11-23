"""Compose docs/assets/social-preview.png (1280x640) from the real app icon and the captured
overlay pill (docs/assets/overlay.png). Run after scripts/capture-readme.ps1."""

from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter, ImageFont

ROOT = Path(__file__).resolve().parent.parent
W, H = 1280, 640
FONTS = Path("C:/Windows/Fonts")

img = Image.new("RGB", (W, H), (13, 14, 20))
# Soft indigo/cyan glows instead of hard edges.
glow = Image.new("RGB", (W, H), (0, 0, 0))
d = ImageDraw.Draw(glow)
d.ellipse((-200, -260, 620, 420), fill=(60, 62, 170))
d.ellipse((820, 300, 1500, 900), fill=(8, 110, 140))
img = Image.blend(img, glow.filter(ImageFilter.GaussianBlur(160)), 0.55)

icon = Image.open(ROOT / "src-tauri/icons/128x128@2x.png").convert("RGBA").resize((132, 132), Image.LANCZOS)
img.paste(icon, (96, 112), icon)

draw = ImageDraw.Draw(img)
title = ImageFont.truetype(str(FONTS / "segoeuib.ttf"), 92)
body = ImageFont.truetype(str(FONTS / "segoeui.ttf"), 34)
small = ImageFont.truetype(str(FONTS / "segoeui.ttf"), 26)
draw.text((256, 118), "Voxora", font=title, fill=(240, 242, 250))
draw.text((98, 290), "Hold a key, speak, release: clean text in any Windows app.", font=body, fill=(214, 218, 232))
draw.text((98, 340), "Whisper + a local LLM on your GPU. No account, no cloud.", font=body, fill=(160, 166, 190))

pill = Image.open(ROOT / "docs/assets/overlay.png").convert("RGB")
# Crop the backdrop to the pill, then feather it onto the card.
pw, ph = pill.size
pill = pill.crop((int(pw * 0.2), int(ph * 0.22), int(pw * 0.8), int(ph * 0.78)))
pill = pill.resize((int(pill.width * 1.35), int(pill.height * 1.35)), Image.LANCZOS)
mask = Image.new("L", pill.size, 0)
ImageDraw.Draw(mask).rounded_rectangle((6, 6, pill.width - 6, pill.height - 6), radius=pill.height // 2, fill=255)
img.paste(pill, (98, 440), mask.filter(ImageFilter.GaussianBlur(3)))
draw.text((98 + pill.width + 30, 440 + pill.height // 2 - 16), "Rust · Tauri · whisper.cpp · llama.cpp · MIT",
          font=small, fill=(140, 146, 170))

out = ROOT / "docs/assets/social-preview.png"
img.save(out, optimize=True)
print(out, img.size, out.stat().st_size // 1024, "KB")
