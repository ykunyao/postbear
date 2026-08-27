# 生成 postbear.ico：米黄圆角底 + 深棕爪印，多尺寸（256~16）
# 用法：python packaging/gen_icon.py  → 输出 assets/postbear.ico

from pathlib import Path

from PIL import Image, ImageDraw

SS = 1024  # 超采样画布
SIZES = [256, 128, 64, 48, 32, 16]

PAPER = (243, 235, 216, 255)   # #F3EBD8 与贴纸同色
PAPER_EDGE = (205, 191, 162, 255)  # #CDBFA2 边线
INK = (61, 53, 40, 255)        # #3D3528 墨色


def rounded_bg() -> Image.Image:
    img = Image.new("RGBA", (SS, SS), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    margin = SS * 0.04
    radius = SS * 0.22
    d.rounded_rectangle(
        [margin, margin, SS - margin, SS - margin],
        radius=radius,
        fill=PAPER,
        outline=PAPER_EDGE,
        width=int(SS * 0.015),
    )
    return img


def draw_paw(img: Image.Image) -> None:
    d = ImageDraw.Draw(img)

    def blob(cx: float, cy: float, rx: float, ry: float) -> None:
        d.ellipse(
            [SS * (cx - rx), SS * (cy - ry), SS * (cx + rx), SS * (cy + ry)],
            fill=INK,
        )

    # 掌垫
    blob(0.5, 0.66, 0.185, 0.155)
    # 三只脚趾
    blob(0.315, 0.42, 0.095, 0.115)
    blob(0.5, 0.355, 0.10, 0.12)
    blob(0.685, 0.42, 0.095, 0.115)


def main() -> None:
    base = rounded_bg()
    draw_paw(base)

    frames = [
        base.resize((size, size), Image.LANCZOS) for size in SIZES
    ]
    out = Path(__file__).resolve().parent.parent / "assets" / "postbear.ico"
    out.parent.mkdir(parents=True, exist_ok=True)
    frames[0].save(out, format="ICO", sizes=[(s, s) for s in SIZES])
    print(f"wrote {out}")


if __name__ == "__main__":
    main()
