# Themes

- `default/layout.css`: current default app shell layout theme
- `github/layout.css`: GitHub-style app shell theme
- `dark/layout.css`: dark reading shell theme
- `light/layout.css`: bright neutral shell theme

MarkHola loads the selected theme from `themes/<theme-name>/layout.css` at runtime when available.
In development, edit the repository `themes/<theme-name>/layout.css` directly.
In the packaged macOS app, the same theme directories are copied into `MarkHola.app/Contents/Resources/themes/`.
