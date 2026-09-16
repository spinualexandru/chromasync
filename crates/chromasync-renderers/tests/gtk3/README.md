# GTK3 coverage audit

The target is a color overlay for GTK 3.24, not a replacement layout theme.
The audit uses the official [CSS overview](https://docs.gtk.org/gtk3/css-overview.html)
and widget CSS-node sections, checked against the installed GTK 3.24 Adwaita CSS
and rendered widgets. Composite widgets reuse their constituent controls.

| Family | Coverage / reference |
| --- | --- |
| Windows, dialogs, drawing surfaces | Window nodes and the opt-in `.background` class; arbitrary canvases are untouched |
| [Notebooks](https://docs.gtk.org/gtk3/class.Notebook.html) | Stack, header, tab states, scroll arrows; selected indicator on all four sides |
| Buttons, toggles, links, switches | Normal, hover, active, checked, disabled; foreground inheritance for child labels/icons |
| [Check buttons](https://docs.gtk.org/gtk3/class.CheckButton.html), [radio buttons](https://docs.gtk.org/gtk3/class.RadioButton.html) | Indicator nodes, checked, indeterminate, disabled; also used by menu items and tree cells |
| Entries, [spin buttons](https://docs.gtk.org/gtk3/class.SpinButton.html), [combo boxes](https://docs.gtk.org/gtk3/class.ComboBox.html) | Input surface, focus, error/warning borders, progress, selection; combo entry/button/popup reuse |
| [Scales](https://docs.gtk.org/gtk3/class.Scale.html) | Trough, highlight, fill, handle, marks/value, hover/active/disabled; excludes `.color` troughs/handles |
| [Scrollbars](https://docs.gtk.org/gtk3/class.Scrollbar.html), [scrolled windows](https://docs.gtk.org/gtk3/class.ScrolledWindow.html) | Trough, handle, junction, directional overflow feedback; retains overlay behavior |
| [Progress bars](https://docs.gtk.org/gtk3/class.ProgressBar.html), [level bars](https://docs.gtk.org/gtk3/class.LevelBar.html) | Trough/progress, value text, filled/empty blocks and low/high/full levels |
| [Expanders](https://docs.gtk.org/gtk3/class.Expander.html), [panes](https://docs.gtk.org/gtk3/class.Paned.html), [frames](https://docs.gtk.org/gtk3/class.Frame.html) | Title/arrow, split handles, frame borders and separators |
| [Text views](https://docs.gtk.org/gtk3/class.TextView.html), tree/icon views | Interior `text`, border windows, selection, headers and rubberband |
| Lists, [flow boxes](https://docs.gtk.org/gtk3/class.FlowBox.html), sidebars | Row/item selection, hover, disabled, inherited text and rubberband |
| Calendar | Header, arrows, highlighted dates, selected dates, out-of-month and disabled text |
| Menus, popovers, tooltips | Surfaces, model/menu items, hover/selected/disabled; tooltip child foreground |
| [Info bars](https://docs.gtk.org/gtk3/class.InfoBar.html) | Semantic message surfaces and child text; includes the actual 3.24 `revealer > box` surface |
| Search/action/header/tool bars, spinner | Internal bar surfaces, controls, spinner foreground; base animations retained |
| File/font/app/recent choosers, stack switcher/sidebar, scale/volume buttons | Compose the covered views, entries, buttons, popovers and scales |
| Color chooser, images, video, GL/custom drawing | Preserve color swatches, hue/alpha gradients, images and content colors intentionally |
| Boxes, grids, overlays, stacks, revealers, viewports | Keep transparent layout containers; their content or explicit `.background` supplies the surface |

## Runtime regression and visual check

Requires Python with PyGObject, Pycairo, GTK3 introspection and a display.
On headless Linux, prefix the Python command with `xvfb-run -a`.

```sh
cargo build -p chromasync
python3 crates/chromasync-renderers/tests/gtk3/verify.py
```

The script generates isolated light/dark themes (it never writes user theme
configuration), renders four widget galleries under both Adwaita variants, and
checks parsing, notebook pixels, text/indicator contrast, foreground inheritance,
and internal surfaces. It compares colors and painted backgrounds for 480
node/state combinations between the base variants. Results are written to
`target/gtk3-audit/`; `--output` and `--binary` override these locations.
CI runs the same script under Xvfb. The Rust golden test independently locks down
the complete generated stylesheet.

Inspect all four galleries after changes. These checks cover standard nodes;
they do not certify arbitrary base themes or application-specific CSS. Pitivi's
fixed-white favorite SVGs are loaded as pixbufs and need an application change,
not a global image recoloring rule.
