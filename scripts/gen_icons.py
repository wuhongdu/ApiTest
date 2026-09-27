from PIL import Image
from pathlib import Path

src = Path(r"C:\Users\Amber\.cursor\projects\f-MyProject-ApiTest\assets\apitest-app-icon.png")
out_dir = Path(r"f:\MyProject\ApiTest\src-tauri\icons")
src_png = Path(r"f:\MyProject\ApiTest\src-tauri\app-icon-source.png")

img = Image.open(src).convert("RGBA")
w, h = img.size
side = max(w, h)
canvas = Image.new("RGBA", (side, side), (0, 0, 0, 0))
canvas.paste(img, ((side - w) // 2, (side - h) // 2))
master = canvas.resize((1024, 1024), Image.Resampling.LANCZOS)
master.save(src_png, format="PNG")
print("saved source", src_png, master.size)

sizes_png = {
    "32x32.png": 32,
    "64x64.png": 64,
    "128x128.png": 128,
    "128x128@2x.png": 256,
    "icon.png": 512,
    "StoreLogo.png": 50,
    "Square30x30Logo.png": 30,
    "Square44x44Logo.png": 44,
    "Square71x71Logo.png": 71,
    "Square89x89Logo.png": 89,
    "Square107x107Logo.png": 107,
    "Square142x142Logo.png": 142,
    "Square150x150Logo.png": 150,
    "Square284x284Logo.png": 284,
    "Square310x310Logo.png": 310,
}
for name, size in sizes_png.items():
    master.resize((size, size), Image.Resampling.LANCZOS).save(out_dir / name, format="PNG")
    print("wrote", name)

ico_sizes = [(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
# Pillow resizes the source for each entry when `sizes=` is set.
# Do NOT use append_images here — it often leaves only a single 256px frame,
# and Windows taskbar then falls back to a default (blue) icon.
master.save(out_dir / "icon.ico", format="ICO", sizes=ico_sizes)
print("wrote icon.ico", ico_sizes)

# Also copy next to release helpers
release_ico = Path(r"f:\MyProject\ApiTest\release\ApiTest.ico")
if release_ico.parent.exists():
    master.save(release_ico, format="ICO", sizes=ico_sizes)
    print("wrote", release_ico)

# Minimal ICNS for mac (store PNG 512 as fallback via pillow if available)
try:
    master.resize((512, 512), Image.Resampling.LANCZOS).save(out_dir / "icon.icns")
except Exception as e:
    print("icns skip:", e)
    # keep existing icns; also write high-res png copy used by some tools
    master.resize((512, 512), Image.Resampling.LANCZOS).save(out_dir / "icon-512.png", format="PNG")

android = {
    "mipmap-mdpi": 48,
    "mipmap-hdpi": 72,
    "mipmap-xhdpi": 96,
    "mipmap-xxhdpi": 144,
    "mipmap-xxxhdpi": 192,
}
for folder, size in android.items():
    d = out_dir / "android" / folder
    d.mkdir(parents=True, exist_ok=True)
    im = master.resize((size, size), Image.Resampling.LANCZOS)
    im.save(d / "ic_launcher.png", format="PNG")
    im.save(d / "ic_launcher_round.png", format="PNG")
    im.save(d / "ic_launcher_foreground.png", format="PNG")

ios_map = {
    "AppIcon-20x20@1x.png": 20,
    "AppIcon-20x20@2x.png": 40,
    "AppIcon-20x20@2x-1.png": 40,
    "AppIcon-20x20@3x.png": 60,
    "AppIcon-29x29@1x.png": 29,
    "AppIcon-29x29@2x.png": 58,
    "AppIcon-29x29@2x-1.png": 58,
    "AppIcon-29x29@3x.png": 87,
    "AppIcon-40x40@1x.png": 40,
    "AppIcon-40x40@2x.png": 80,
    "AppIcon-40x40@2x-1.png": 80,
    "AppIcon-40x40@3x.png": 120,
    "AppIcon-60x60@2x.png": 120,
    "AppIcon-60x60@3x.png": 180,
    "AppIcon-76x76@1x.png": 76,
    "AppIcon-76x76@2x.png": 152,
    "AppIcon-83.5x83.5@2x.png": 167,
    "AppIcon-512@2x.png": 1024,
}
ios_dir = out_dir / "ios"
for name, size in ios_map.items():
    master.resize((size, size), Image.Resampling.LANCZOS).save(ios_dir / name, format="PNG")

print("done")
