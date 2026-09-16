"""GTK 3 runtime regression/visual audit; needs PyGObject, GTK3 and a display.

Build the CLI first, then run this script (or use xvfb-run on CI).
Outputs four galleries: each generated mode over each Adwaita base variant.
"""

import argparse
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path


def run_gallery(css, base, output):
    import cairo
    import gi

    gi.require_version("Gtk", "3.0")
    gi.require_version("Gdk", "3.0")
    gi.require_foreign("cairo")
    from gi.repository import Gdk, GLib, Gtk

    settings = Gtk.Settings.get_default()
    assert settings is not None, "GTK needs a display (use xvfb-run on CI)"
    settings.props.gtk_theme_name = "Adwaita"
    settings.props.gtk_application_prefer_dark_theme = base == "dark"
    settings.props.gtk_enable_animations = False
    errors = []
    provider = Gtk.CssProvider()
    provider.connect(
        "parsing-error", lambda _, section, error: errors.append(error.message)
    )
    provider.load_from_path(str(css))
    Gtk.StyleContext.add_provider_for_screen(
        Gdk.Screen.get_default(), provider, Gtk.STYLE_PROVIDER_PRIORITY_USER
    )
    assert not errors, errors

    window = Gtk.OffscreenWindow()
    window.set_default_size(980, 780)
    root = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12, margin=12)
    window.add(root)
    root.pack_start(
        Gtk.Label(label=f"Chromasync GTK3 — {css.parent.name}, Adwaita {base}"),
        False,
        False,
        0,
    )
    columns = Gtk.Box(spacing=14)
    root.pack_start(columns, True, True, 0)
    left = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=10)
    right = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=10)
    columns.pack_start(left, True, True, 0)
    columns.pack_start(right, True, True, 0)
    notebook = Gtk.Notebook()
    page = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8, margin=8)
    label = Gtk.Label(
        label="Pitivi regression: readable text on a notebook page", wrap=True
    )
    page.pack_start(label, False, False, 0)
    expander = Gtk.Expander(label="Expanded section")
    expander.add(Gtk.Label(label="Properties inherit the page foreground"))
    expander.set_expanded(True)
    page.pack_start(expander, False, False, 0)
    page.pack_start(Gtk.Button(label="Create a title clip"), False, False, 0)
    notebook.append_page(page, Gtk.Label(label="Clip"))
    notebook.append_page(Gtk.Label(label="Transition"), Gtk.Label(label="Transition"))
    notebook.set_size_request(-1, 175)
    left.pack_start(notebook, False, False, 0)
    controls = Gtk.Box(spacing=10)
    left.pack_start(controls, False, False, 0)
    for checked, sensitive in [(False, True), (True, True), (True, False)]:
        check = Gtk.CheckButton(label="Check")
        check.set_active(checked)
        check.set_sensitive(sensitive)
        controls.pack_start(check, False, False, 0)
    radios = Gtk.Box(spacing=10)
    first_radio = None
    for name in ["Radio A", "Radio B"]:
        radio = Gtk.RadioButton.new_with_label_from_widget(first_radio, name)
        first_radio = first_radio or radio
        radios.pack_start(radio, False, False, 0)
    switch = Gtk.Switch(active=True)
    radios.pack_end(switch, False, False, 0)
    left.pack_start(radios, False, False, 0)
    inputs = Gtk.Box(spacing=8)
    spin = Gtk.SpinButton.new_with_range(0, 100, 1)
    spin.set_value(24)
    combo = Gtk.ComboBoxText()
    combo.append_text("Combo box")
    combo.set_active(0)
    inputs.pack_start(spin, True, True, 0)
    inputs.pack_start(combo, True, True, 0)
    left.pack_start(inputs, False, False, 0)
    entry = Gtk.Entry(text="Input with error border")
    entry.get_style_context().add_class("error")
    left.pack_start(entry, False, False, 0)
    scale = Gtk.Scale.new_with_range(Gtk.Orientation.HORIZONTAL, 0, 100, 1)
    scale.set_value(35)
    scale.add_mark(75, Gtk.PositionType.BOTTOM, "Mark")
    left.pack_start(scale, False, False, 0)
    disabled_scale = Gtk.Scale.new_with_range(Gtk.Orientation.HORIZONTAL, 0, 100, 1)
    disabled_scale.set_value(60)
    disabled_scale.set_sensitive(False)
    left.pack_start(disabled_scale, False, False, 0)
    progress = Gtk.ProgressBar(fraction=0.6, show_text=True, text="Progress")
    left.pack_start(progress, False, False, 0)
    level = Gtk.LevelBar.new_for_interval(0, 5)
    level.set_mode(Gtk.LevelBarMode.DISCRETE)
    level.set_value(3)
    left.pack_start(level, False, False, 0)
    calendar = Gtk.Calendar()
    calendar.select_month(8, 2026)
    calendar.select_day(16)
    right.pack_start(calendar, False, False, 0)
    text = Gtk.TextView()
    text.get_buffer().set_text("TextView interior and selection\nSecond line")
    text.get_buffer().select_range(
        text.get_buffer().get_iter_at_offset(0), text.get_buffer().get_iter_at_offset(8)
    )
    scroll = Gtk.ScrolledWindow()
    scroll.set_policy(Gtk.PolicyType.ALWAYS, Gtk.PolicyType.ALWAYS)
    scroll.add(text)
    scroll.set_size_request(-1, 110)
    right.pack_start(scroll, False, False, 0)
    listing = Gtk.ListBox()
    listing.add(Gtk.Label(label="Selected list row"))
    listing.add(Gtk.Label(label="Ordinary list row"))
    listing.select_row(listing.get_row_at_index(0))
    right.pack_start(listing, False, False, 0)
    flow = Gtk.FlowBox()
    flow.add(Gtk.Label(label="Flow selection"))
    flow.add(Gtk.Label(label="Flow item"))
    flow.select_child(flow.get_child_at_index(0))
    right.pack_start(flow, False, False, 0)
    info = Gtk.InfoBar(message_type=Gtk.MessageType.WARNING)
    warning_label = Gtk.Label(label="Warning message")
    info.get_content_area().add(warning_label)
    right.pack_start(info, False, False, 0)
    drawing = Gtk.DrawingArea()
    drawing.get_style_context().add_class("background")
    drawing.set_size_request(-1, 35)
    drawing.connect(
        "draw",
        lambda widget, cr: Gtk.render_background(
            widget.get_style_context(),
            cr,
            0,
            0,
            widget.get_allocated_width(),
            widget.get_allocated_height(),
        ),
    )
    frame = Gtk.Frame(label="Drawing area .background (audio meter)")
    frame.add(drawing)
    right.pack_start(frame, False, False, 0)
    window.show_all()

    # Query real GTK CSS resolution on documented subnodes. No CSS string matching.
    # Widget paths cover private subnodes that PyGObject does not expose as widgets.
    def context(parent, name, classes=(), state=Gtk.StateFlags.NORMAL):
        ctx = Gtk.StyleContext()
        path = parent.get_path().copy()
        pos = path.append_type(Gtk.Widget)
        path.iter_set_object_name(pos, name)
        for cls in classes:
            path.iter_add_class(pos, cls)
        path.iter_set_state(pos, state)
        ctx.set_path(path)
        ctx.set_parent(parent)
        ctx.set_state(state)
        return ctx

    def rgba(ctx, prop):
        color = ctx.get_property(prop, ctx.get_state())
        return tuple(
            round(v * 255) for v in (color.red, color.green, color.blue, color.alpha)
        )

    def named(name):
        found, color = window.get_style_context().lookup_color(name)
        assert found, name
        return tuple(
            round(v * 255) for v in (color.red, color.green, color.blue, color.alpha)
        )

    checks = []
    snapshots = {}

    def paint(ctx):
        surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, 40, 40)
        Gtk.render_background(ctx, cairo.Context(surface), 0, 0, 40, 40)
        surface.flush()
        # Native-endian ARGB bytes are used only for comparison on the same host.
        return list(surface.get_data()[20 * surface.get_stride() + 20 * 4 :][:4])

    def equal(ctx, prop, expected, description):
        actual = rgba(ctx, prop)
        assert actual == expected, f"{description}: {actual} != {expected}"
        checks.append(description)

    def verify():
        try:
            bg = named("theme_bg_color")
            fg = named("theme_fg_color")
            view_bg = named("theme_base_color")
            equal(label.get_style_context(), "color", fg, "notebook label")
            equal(
                drawing.get_style_context(), "background-color", bg, "drawing surface"
            )
            nctx = notebook.get_style_context()
            for state in [Gtk.StateFlags.NORMAL, Gtk.StateFlags.BACKDROP]:
                stack = context(nctx, "stack", state=state)
                equal(stack, "background-color", bg, f"notebook stack {state}")
                equal(stack, "color", fg, f"notebook foreground {state}")
                for side in ["top", "bottom", "left", "right"]:
                    header = context(nctx, "header", [side], state)
                    tabs = context(header, "tabs", state=state)
                    tab = context(tabs, "tab", state=state | Gtk.StateFlags.CHECKED)
                    equal(tab, "color", fg, f"{side} checked tab {state}")
                    equal(
                        tab,
                        "background-color",
                        bg,
                        f"{side} checked tab background {state}",
                    )
            equal(
                context(text.get_style_context(), "text"),
                "background-color",
                view_bg,
                "text interior",
            )
            equal(entry.get_style_context(), "color", fg, "error input foreground")
            equal(
                entry.get_style_context(),
                "background-color",
                named("theme_entry_background"),
                "error input background",
            )
            # Check the actual child box painted by GTK 3.24's InfoBar, not only its root.
            info_surface = info.get_content_area().get_parent().get_style_context()
            equal(
                info_surface,
                "background-color",
                named("warning_color"),
                "warning surface",
            )
            equal(
                warning_label.get_style_context(),
                "color",
                rgba(info.get_style_context(), "color"),
                "warning label",
            )
            # Regression probe for actual notebook rendering (including background images).
            pixbuf = window.get_pixbuf()
            x, y = page.translate_coordinates(
                window, 4, page.get_allocated_height() - 4
            )
            offset = y * pixbuf.get_rowstride() + x * pixbuf.get_n_channels()
            assert tuple(pixbuf.get_pixels()[offset : offset + 3]) == bg[:3], (
                "painted notebook background"
            )
            # Compare computed colors AND painted surfaces across base variants. This catches
            # remaining Adwaita gradients that obscure an otherwise correct background-color.
            paths = [
                "notebook/header.top/tabs/tab",
                "notebook/stack",
                "checkbutton/check",
                "radiobutton/radio",
                "spinbutton/entry",
                "spinbutton/button.up",
                "scale/contents/trough",
                "scale/contents/trough/highlight",
                "scale/contents/trough/slider",
                "scrollbar/contents/trough",
                "scrollbar/contents/trough/slider",
                "progressbar/trough/progress",
                "levelbar/trough/block.filled",
                "levelbar/trough/block.empty",
                "textview.view/text",
                "calendar",
                "calendar.header",
                "calendar.button",
                "flowbox/flowboxchild",
                "list/row",
                "expander/title/arrow",
                "tooltip.background",
                "tooltip.background/label",
                "paned/separator",
                "infobar.warning/revealer/box",
                "infobar.warning/revealer/box/label",
                "searchbar/revealer/box",
                "actionbar/revealer/box",
                "menu/menuitem",
                "menu/menuitem/check",
                "button",
                "entry",
                "spinner",
                "treeview.view",
                "treeview.view/header/button",
                "treeview.view/rubberband",
                "iconview.view",
                "frame/border",
                "textview.view/text/selection",
                "scrolledwindow/junction",
            ]
            for path in paths:
                for state in [
                    Gtk.StateFlags.NORMAL,
                    Gtk.StateFlags.PRELIGHT,
                    Gtk.StateFlags.CHECKED,
                    Gtk.StateFlags.INSENSITIVE,
                    Gtk.StateFlags.BACKDROP,
                    Gtk.StateFlags.ACTIVE,
                    Gtk.StateFlags.SELECTED,
                    Gtk.StateFlags.FOCUSED,
                    Gtk.StateFlags.INCONSISTENT,
                    Gtk.StateFlags.CHECKED | Gtk.StateFlags.INSENSITIVE,
                    Gtk.StateFlags.CHECKED | Gtk.StateFlags.BACKDROP,
                    Gtk.StateFlags.CHECKED | Gtk.StateFlags.PRELIGHT,
                ]:
                    ctx = window.get_style_context()
                    for segment in path.split("/"):
                        name, *classes = segment.split(".")
                        ctx = context(ctx, name, classes, state)
                    snapshots[f"{path}:{int(state)}"] = [
                        rgba(ctx, "color"),
                        rgba(ctx, "background-color"),
                        paint(ctx),
                    ]
            output.with_suffix(".json").write_text(
                json.dumps(snapshots, indent=2) + "\n"
            )

            # Contrast checks catch mismatched foreground/background pairs across variants.
            def luminance(color):
                linear = [
                    v / 255 / 12.92
                    if v / 255 <= 0.04045
                    else ((v / 255 + 0.055) / 1.055) ** 2.4
                    for v in color[:3]
                ]
                return sum(
                    v * weight for v, weight in zip(linear, [0.2126, 0.7152, 0.0722])
                )

            def contrast(a, b):
                lo, hi = sorted([luminance(a), luminance(b)])
                return (hi + 0.05) / (lo + 0.05)

            assert contrast(fg, bg) >= 4.5, "notebook text contrast"
            # Compare widget colors, not a fixed palette or CSS implementation.
            for name, widget, node, active in [
                ("check", first_radio, "radio", Gtk.StateFlags.CHECKED),
                ("scale", scale, "slider", Gtk.StateFlags.NORMAL),
            ]:
                parent = widget.get_style_context()
                if name == "scale":
                    parent = context(context(parent, "contents"), "trough")
                ctx = context(parent, node, state=active)
                assert contrast(rgba(ctx, "background-color"), bg) >= 3, (
                    name + " indicator contrast"
                )
                assert (
                    contrast(rgba(ctx, "color"), rgba(ctx, "background-color")) >= 4.5
                ), name + " foreground contrast"
                checks.append(name + " contrast")
            window.get_pixbuf().savev(str(output), "png", [], [])
            # Color chooser gradients are application data. Verify an application
            # provider survives our higher-priority user stylesheet unchanged.
            data_provider = Gtk.CssProvider()
            data_provider.load_from_data(
                b"scale.color trough, colorswatch { background-color: #123456; background-image: linear-gradient(to right, #ff0000, #00ff00); }"
            )
            Gtk.StyleContext.add_provider_for_screen(
                Gdk.Screen.get_default(),
                data_provider,
                Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION,
            )

            def data_surfaces():
                color_scale = context(window.get_style_context(), "scale", ["color"])
                return [
                    paint(context(color_scale, "trough")),
                    paint(context(window.get_style_context(), "colorswatch")),
                ]

            with_overlay = data_surfaces()
            Gtk.StyleContext.remove_provider_for_screen(
                Gdk.Screen.get_default(), provider
            )
            assert with_overlay == data_surfaces(), (
                "application color gradients must be preserved"
            )

            print(
                json.dumps(
                    {
                        "base": base,
                        "css": str(css),
                        "checks": len(checks),
                        "screenshot": str(output),
                    }
                )
            )
        except Exception as error:  # noqa: BLE001 - re-raised after leaving the GTK callback
            failures.append(error)
        finally:
            Gtk.main_quit()

    failures = []
    GLib.timeout_add(100, verify)
    Gtk.main()
    if failures:
        raise failures[0]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=Path("target/debug/chromasync"))
    parser.add_argument("--output", type=Path, default=Path("target/gtk3-audit"))
    parser.add_argument("--css", type=Path, help=argparse.SUPPRESS)
    parser.add_argument("--base", choices=["light", "dark"], help=argparse.SUPPRESS)
    args = parser.parse_args()
    if args.css:
        run_gallery(args.css, args.base, args.output)
        return
    args.output.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="chromasync-gtk3-") as directory:
        env = {**os.environ, "XDG_CONFIG_HOME": directory}
        env.pop("GTK_THEME", None)
        for mode in ["light", "dark"]:
            destination = args.output.resolve() / mode
            subprocess.run(
                [
                    str(args.binary.resolve()),
                    "generate",
                    "--seed",
                    "#55AAFF",
                    "--template",
                    "materialish",
                    "--mode",
                    mode,
                    "--targets",
                    "gtk3",
                    "--output",
                    str(destination),
                    "--force",
                ],
                env=env,
                check=True,
            )
            for base in ["light", "dark"]:
                subprocess.run(
                    [
                        sys.executable,
                        __file__,
                        "--css",
                        str(destination / "chromasync.css"),
                        "--base",
                        base,
                        "--output",
                        str(args.output.resolve() / f"{mode}-on-{base}.png"),
                    ],
                    env=env,
                    check=True,
                )
            light = json.loads((args.output / f"{mode}-on-light.json").read_text())
            dark = json.loads((args.output / f"{mode}-on-dark.json").read_text())
            differences = {
                key: (light[key], dark[key]) for key in light if light[key] != dark[key]
            }
            assert not differences, json.dumps(differences, indent=2)
            print(
                f"{mode}: {len(light)} node/state combinations independent of base variant"
            )


if __name__ == "__main__":
    main()
