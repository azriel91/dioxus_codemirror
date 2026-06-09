# Changelog

## Unreleased

* `Ctrl + Up` / `Ctrl + Down` scrolls the view by one line without moving the caret. ([#5][#5])
* `Left` / `Right` (and `Shift` variants) at a fold boundary jump across the fold instead of expanding it. ([#5][#5])
* The active line within a selection uses the new `active_line_selected` theme colour, so it reads as selected instead of hiding the selection. ([#5][#5])
* `Ctrl + Shift + ]` with a selection unfolds all folds inside it, leaving each unfolded range selected. ([#5][#5])
* `Ctrl + Shift + [` with a selection folds exactly the selected characters. ([#5][#5])

[#5]: https://github.com/azriel91/dioxus_codemirror/pull/5


## 0.3.0 (2026-06-09)

* Support code folding. ([#4][#4])

[#4]: https://github.com/azriel91/dioxus_codemirror/pull/4


## 0.2.0 (2026-06-07)

* Support code actions with `Ctrl/Cmd + .` keybinding. ([#2][#2])
* Support theme colour overrides via the `theme_colors` prop (`ThemeColors` / `ThemeColor`). ([#3][#3])

[#2]: https://github.com/azriel91/dioxus_codemirror/pull/2
[#3]: https://github.com/azriel91/dioxus_codemirror/pull/3


## 0.1.0 (2026-06-06)

* Add `CodeMirror` dioxus component with CodeMirror 6 integration. ([#1][#1])
* Support some CodeMirror features. ([#1][#1])

[#1]: https://github.com/azriel91/dioxus_codemirror/pull/1
