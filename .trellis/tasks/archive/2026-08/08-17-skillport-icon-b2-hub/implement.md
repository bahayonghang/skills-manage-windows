# Implement: B2 应用图标

## Checklist

1. 从会话 `images/b2-master.png` 恢复 1024 RGBA 主图到 `src-tauri/icons/icon-source.png`。
2. 运行 `pnpm tauri icon src-tauri/icons/icon-source.png --ios-color "#1e1e2e"`。
3. 目视核对 `icon.png`、`128x128.png`、`32x32.png`，确认不是门户稿。
4. 改 `scripts/generate_icon.py`：删除 L 形绘制；核验 `icon-source.png` 后打印 `pnpm tauri icon` 用法，不覆盖主图。
5. 跑 `just ci`。

## Validation

```text
python -c "from PIL import Image; im=Image.open('src-tauri/icons/icon-source.png'); print(im.size, im.mode)"
pnpm tauri icon src-tauri/icons/icon-source.png --ios-color "#1e1e2e"
python scripts/generate_icon.py
just ci
```

## Rollback

`git checkout -- src-tauri/icons ref/SkillPort-icon-assets scripts/generate_icon.py`

## Notes

- 不改版本号。
- 不提交 4096 PNG。
