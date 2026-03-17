use chromasync_types::{GeneratedArtifact, HexColor, RenderTarget, SemanticTokens};

use crate::{Renderer, RendererError, normalized_hex, terminal_ansi_colors};

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct AlacrittyRenderer;

#[derive(Debug, Clone, PartialEq, Eq)]
struct AlacrittyTheme {
    foreground: HexColor,
    background: HexColor,
    cursor: HexColor,
    cursor_text: HexColor,
    selection_foreground: HexColor,
    selection_background: HexColor,
    footer_background: HexColor,
    footer_foreground: HexColor,
    line_indicator_background: HexColor,
    line_indicator_foreground: HexColor,
    search_match_background: HexColor,
    search_match_foreground: HexColor,
    focused_match_background: HexColor,
    focused_match_foreground: HexColor,
    hint_start_background: HexColor,
    hint_start_foreground: HexColor,
    hint_end_background: HexColor,
    hint_end_foreground: HexColor,
    ansi: [HexColor; 16],
}

impl AlacrittyRenderer {
    fn map(&self, tokens: &SemanticTokens) -> AlacrittyTheme {
        AlacrittyTheme {
            foreground: tokens.text.clone(),
            background: tokens.bg.clone(),
            cursor: tokens.accent.clone(),
            cursor_text: tokens.accent_fg.clone(),
            selection_foreground: tokens.text.clone(),
            selection_background: tokens.selection.clone(),
            footer_background: tokens.surface_elevated.clone(),
            footer_foreground: tokens.text.clone(),
            line_indicator_background: tokens.surface.clone(),
            line_indicator_foreground: tokens.text_muted.clone(),
            search_match_background: tokens.surface.clone(),
            search_match_foreground: tokens.text.clone(),
            focused_match_background: tokens.accent.clone(),
            focused_match_foreground: tokens.accent_fg.clone(),
            hint_start_background: tokens.link.clone(),
            hint_start_foreground: tokens.accent_fg.clone(),
            hint_end_background: tokens.accent_hover.clone(),
            hint_end_foreground: tokens.accent_fg.clone(),
            ansi: terminal_ansi_colors(tokens),
        }
    }

    fn render(&self, theme: &AlacrittyTheme) -> Result<String, RendererError> {
        Ok(format!(
            concat!(
                "[colors.primary]\n",
                "background = \"{background}\"\n",
                "foreground = \"{foreground}\"\n",
                "\n",
                "[colors.cursor]\n",
                "cursor = \"{cursor}\"\n",
                "text = \"{cursor_text}\"\n",
                "\n",
                "[colors.selection]\n",
                "background = \"{selection_background}\"\n",
                "text = \"{selection_foreground}\"\n",
                "\n",
                "[colors.footer_bar]\n",
                "background = \"{footer_background}\"\n",
                "foreground = \"{footer_foreground}\"\n",
                "\n",
                "[colors.hints.start]\n",
                "background = \"{hint_start_background}\"\n",
                "foreground = \"{hint_start_foreground}\"\n",
                "\n",
                "[colors.hints.end]\n",
                "background = \"{hint_end_background}\"\n",
                "foreground = \"{hint_end_foreground}\"\n",
                "\n",
                "[colors.line_indicator]\n",
                "background = \"{line_indicator_background}\"\n",
                "foreground = \"{line_indicator_foreground}\"\n",
                "\n",
                "[colors.search.matches]\n",
                "background = \"{search_match_background}\"\n",
                "foreground = \"{search_match_foreground}\"\n",
                "\n",
                "[colors.search.focused_match]\n",
                "background = \"{focused_match_background}\"\n",
                "foreground = \"{focused_match_foreground}\"\n",
                "\n",
                "[colors.normal]\n",
                "black = \"{color0}\"\n",
                "red = \"{color1}\"\n",
                "green = \"{color2}\"\n",
                "yellow = \"{color3}\"\n",
                "blue = \"{color4}\"\n",
                "magenta = \"{color5}\"\n",
                "cyan = \"{color6}\"\n",
                "white = \"{color7}\"\n",
                "\n",
                "[colors.bright]\n",
                "black = \"{color8}\"\n",
                "red = \"{color9}\"\n",
                "green = \"{color10}\"\n",
                "yellow = \"{color11}\"\n",
                "blue = \"{color12}\"\n",
                "magenta = \"{color13}\"\n",
                "cyan = \"{color14}\"\n",
                "white = \"{color15}\"\n"
            ),
            background = normalized_hex(&theme.background)?,
            foreground = normalized_hex(&theme.foreground)?,
            cursor = normalized_hex(&theme.cursor)?,
            cursor_text = normalized_hex(&theme.cursor_text)?,
            selection_background = normalized_hex(&theme.selection_background)?,
            selection_foreground = normalized_hex(&theme.selection_foreground)?,
            footer_background = normalized_hex(&theme.footer_background)?,
            footer_foreground = normalized_hex(&theme.footer_foreground)?,
            hint_start_background = normalized_hex(&theme.hint_start_background)?,
            hint_start_foreground = normalized_hex(&theme.hint_start_foreground)?,
            hint_end_background = normalized_hex(&theme.hint_end_background)?,
            hint_end_foreground = normalized_hex(&theme.hint_end_foreground)?,
            line_indicator_background = normalized_hex(&theme.line_indicator_background)?,
            line_indicator_foreground = normalized_hex(&theme.line_indicator_foreground)?,
            search_match_background = normalized_hex(&theme.search_match_background)?,
            search_match_foreground = normalized_hex(&theme.search_match_foreground)?,
            focused_match_background = normalized_hex(&theme.focused_match_background)?,
            focused_match_foreground = normalized_hex(&theme.focused_match_foreground)?,
            color0 = normalized_hex(&theme.ansi[0])?,
            color1 = normalized_hex(&theme.ansi[1])?,
            color2 = normalized_hex(&theme.ansi[2])?,
            color3 = normalized_hex(&theme.ansi[3])?,
            color4 = normalized_hex(&theme.ansi[4])?,
            color5 = normalized_hex(&theme.ansi[5])?,
            color6 = normalized_hex(&theme.ansi[6])?,
            color7 = normalized_hex(&theme.ansi[7])?,
            color8 = normalized_hex(&theme.ansi[8])?,
            color9 = normalized_hex(&theme.ansi[9])?,
            color10 = normalized_hex(&theme.ansi[10])?,
            color11 = normalized_hex(&theme.ansi[11])?,
            color12 = normalized_hex(&theme.ansi[12])?,
            color13 = normalized_hex(&theme.ansi[13])?,
            color14 = normalized_hex(&theme.ansi[14])?,
            color15 = normalized_hex(&theme.ansi[15])?,
        ))
    }
}

impl Renderer for AlacrittyRenderer {
    fn target(&self) -> RenderTarget {
        RenderTarget::Alacritty
    }

    fn name(&self) -> &'static str {
        "alacritty"
    }

    fn render_artifact(&self, tokens: &SemanticTokens) -> Result<GeneratedArtifact, RendererError> {
        let theme = self.map(tokens);

        Ok(GeneratedArtifact {
            target: self.name().to_owned(),
            file_name: self.file_name().to_owned(),
            content: self.render(&theme)?,
        })
    }
}
