use chromasync_types::{GeneratedArtifact, HexColor, RenderTarget, SemanticTokens};

use crate::{Renderer, RendererError, terminal_ansi_colors};

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct KittyRenderer;

#[derive(Debug, Clone, PartialEq, Eq)]
struct KittyTheme {
    foreground: HexColor,
    background: HexColor,
    selection_foreground: HexColor,
    selection_background: HexColor,
    cursor: HexColor,
    cursor_text: HexColor,
    url_color: HexColor,
    active_border_color: HexColor,
    inactive_border_color: HexColor,
    active_tab_background: HexColor,
    active_tab_foreground: HexColor,
    inactive_tab_background: HexColor,
    inactive_tab_foreground: HexColor,
    ansi: [HexColor; 16],
}

impl KittyRenderer {
    fn map(&self, tokens: &SemanticTokens) -> KittyTheme {
        KittyTheme {
            foreground: tokens.text.clone(),
            background: tokens.bg.clone(),
            selection_foreground: tokens.text.clone(),
            selection_background: tokens.selection.clone(),
            cursor: tokens.accent.clone(),
            cursor_text: tokens.accent_fg.clone(),
            url_color: tokens.link.clone(),
            active_border_color: tokens.accent.clone(),
            inactive_border_color: tokens.border.clone(),
            active_tab_background: tokens.accent.clone(),
            active_tab_foreground: tokens.accent_fg.clone(),
            inactive_tab_background: tokens.surface.clone(),
            inactive_tab_foreground: tokens.text_muted.clone(),
            ansi: terminal_ansi_colors(tokens),
        }
    }

    fn render(&self, theme: &KittyTheme) -> String {
        format!(
            concat!(
                "background {background}\n",
                "foreground {foreground}\n",
                "selection_background {selection_background}\n",
                "selection_foreground {selection_foreground}\n",
                "cursor {cursor}\n",
                "cursor_text_color {cursor_text}\n",
                "url_color {url_color}\n",
                "active_border_color {active_border_color}\n",
                "inactive_border_color {inactive_border_color}\n",
                "active_tab_background {active_tab_background}\n",
                "active_tab_foreground {active_tab_foreground}\n",
                "inactive_tab_background {inactive_tab_background}\n",
                "inactive_tab_foreground {inactive_tab_foreground}\n",
                "color0 {color0}\n",
                "color1 {color1}\n",
                "color2 {color2}\n",
                "color3 {color3}\n",
                "color4 {color4}\n",
                "color5 {color5}\n",
                "color6 {color6}\n",
                "color7 {color7}\n",
                "color8 {color8}\n",
                "color9 {color9}\n",
                "color10 {color10}\n",
                "color11 {color11}\n",
                "color12 {color12}\n",
                "color13 {color13}\n",
                "color14 {color14}\n",
                "color15 {color15}\n"
            ),
            background = theme.background,
            foreground = theme.foreground,
            selection_background = theme.selection_background,
            selection_foreground = theme.selection_foreground,
            cursor = theme.cursor,
            cursor_text = theme.cursor_text,
            url_color = theme.url_color,
            active_border_color = theme.active_border_color,
            inactive_border_color = theme.inactive_border_color,
            active_tab_background = theme.active_tab_background,
            active_tab_foreground = theme.active_tab_foreground,
            inactive_tab_background = theme.inactive_tab_background,
            inactive_tab_foreground = theme.inactive_tab_foreground,
            color0 = theme.ansi[0],
            color1 = theme.ansi[1],
            color2 = theme.ansi[2],
            color3 = theme.ansi[3],
            color4 = theme.ansi[4],
            color5 = theme.ansi[5],
            color6 = theme.ansi[6],
            color7 = theme.ansi[7],
            color8 = theme.ansi[8],
            color9 = theme.ansi[9],
            color10 = theme.ansi[10],
            color11 = theme.ansi[11],
            color12 = theme.ansi[12],
            color13 = theme.ansi[13],
            color14 = theme.ansi[14],
            color15 = theme.ansi[15],
        )
    }
}

impl Renderer for KittyRenderer {
    fn target(&self) -> RenderTarget {
        RenderTarget::Kitty
    }

    fn name(&self) -> &'static str {
        "kitty"
    }

    fn render_artifact(&self, tokens: &SemanticTokens) -> Result<GeneratedArtifact, RendererError> {
        let theme = self.map(tokens);

        Ok(GeneratedArtifact {
            target: self.name().to_owned(),
            file_name: self.file_name().to_owned(),
            content: self.render(&theme),
        })
    }
}
