use chromasync_color::{MIN_CONTRAST_RATIO, apca_contrast_score, contrast_ratio, generate_palette};
use chromasync_template::{built_in_templates, resolve_tokens_with_strategy};
use chromasync_types::{ChromaStrategy, ContrastStrategy, SemanticTokens, ThemeMode};

const SEEDS: [&str; 16] = [
    "#000000", "#FFFFFF", "#808080", "#FF0000", "#00FF00", "#0000FF", "#FFFF00", "#00FFFF",
    "#FF00FF", "#FF6B6B", "#4ECDC4", "#8B5CF6", "#F59E0B", "#22C55E", "#0EA5E9", "#EC4899",
];

const CHROMA_STRATEGIES: [ChromaStrategy; 5] = [
    ChromaStrategy::Subtle,
    ChromaStrategy::Normal,
    ChromaStrategy::Vibrant,
    ChromaStrategy::Muted,
    ChromaStrategy::Industrial,
];

#[derive(Clone, Copy)]
struct ContrastContract {
    foreground: Token,
    background: Token,
    minimum_ratio: f32,
    usage: &'static str,
}

#[derive(Clone, Copy)]
enum Token {
    Bg,
    BgSecondary,
    Surface,
    SurfaceElevated,
    Text,
    TextMuted,
    Accent,
    AccentHover,
    AccentActive,
    AccentFg,
    Selection,
    Link,
    Success,
    Warning,
    Error,
}

// These are the readable foreground/background adjacencies emitted by the
// built-in target specs. Disabled and explicitly dim text, decorative borders,
// and translucent background overlays are intentionally excluded.
const CONTRAST_CONTRACTS: [ContrastContract; 31] = [
    contract(Token::Text, Token::Bg, "default foreground"),
    contract(Token::Text, Token::BgSecondary, "bars and browser tabs"),
    contract(
        Token::Text,
        Token::Surface,
        "views, inputs, and inactive tabs",
    ),
    contract(
        Token::Text,
        Token::SurfaceElevated,
        "header bars, dialogs, and status bars",
    ),
    contract(Token::Text, Token::Selection, "selected text"),
    contract(Token::TextMuted, Token::Bg, "comments and line numbers"),
    contract(
        Token::TextMuted,
        Token::BgSecondary,
        "inactive bars and terminal text",
    ),
    contract(Token::TextMuted, Token::Surface, "inactive tab text"),
    contract(
        Token::TextMuted,
        Token::SurfaceElevated,
        "secondary elevated-surface text",
    ),
    contract(Token::AccentFg, Token::Accent, "accent buttons and badges"),
    contract(
        Token::AccentFg,
        Token::AccentHover,
        "hovered VS Code buttons",
    ),
    contract(Token::AccentFg, Token::Success, "GTK success backgrounds"),
    contract(
        Token::AccentFg,
        Token::Warning,
        "warning status backgrounds",
    ),
    contract(
        Token::AccentFg,
        Token::Error,
        "error and destructive backgrounds",
    ),
    contract(Token::Accent, Token::Bg, "keywords and terminal colors"),
    contract(
        Token::AccentHover,
        Token::Bg,
        "bright terminal and editor colors",
    ),
    contract(
        Token::AccentActive,
        Token::Bg,
        "types, operators, and terminal colors",
    ),
    contract(Token::Link, Token::Bg, "links and editor syntax"),
    contract(Token::Success, Token::Bg, "strings and success text"),
    contract(Token::Warning, Token::Bg, "numbers and warning text"),
    contract(Token::Error, Token::Bg, "errors and invalid syntax"),
    contract(Token::TextMuted, Token::Selection, "inactive selected text"),
    ui_contract(Token::Accent, Token::BgSecondary, "focused borders on bars"),
    ui_contract(Token::Accent, Token::Surface, "focused control borders"),
    ui_contract(
        Token::Accent,
        Token::SurfaceElevated,
        "focused borders on elevated surfaces",
    ),
    ui_contract(
        Token::AccentHover,
        Token::BgSecondary,
        "hover decorations on bars",
    ),
    ui_contract(
        Token::AccentHover,
        Token::Surface,
        "hover decorations on controls",
    ),
    ui_contract(
        Token::AccentHover,
        Token::SurfaceElevated,
        "hover decorations on elevated surfaces",
    ),
    ui_contract(
        Token::AccentActive,
        Token::BgSecondary,
        "selected borders on bars",
    ),
    ui_contract(
        Token::AccentActive,
        Token::Surface,
        "selected control borders",
    ),
    ui_contract(
        Token::AccentActive,
        Token::SurfaceElevated,
        "selected borders on elevated surfaces",
    ),
];

const fn contract(foreground: Token, background: Token, usage: &'static str) -> ContrastContract {
    ContrastContract {
        foreground,
        background,
        minimum_ratio: MIN_CONTRAST_RATIO,
        usage,
    }
}

const fn ui_contract(
    foreground: Token,
    background: Token,
    usage: &'static str,
) -> ContrastContract {
    ContrastContract {
        foreground,
        background,
        minimum_ratio: 3.0,
        usage,
    }
}

#[test]
fn built_in_target_pairs_meet_wcag_contrast() {
    let templates = built_in_templates().expect("built-in templates should load");

    for template in templates {
        for chroma in CHROMA_STRATEGIES {
            for seed in SEEDS {
                let palette = generate_palette(seed, template.definition.mode, chroma)
                    .expect("palette should generate");
                let tokens = resolve_tokens_with_strategy(
                    &palette,
                    &template.definition,
                    ContrastStrategy::RelativeLuminance,
                )
                .expect("semantic tokens should resolve");

                for check in CONTRAST_CONTRACTS {
                    assert_contract(
                        check,
                        &tokens,
                        &template.definition.name,
                        template.definition.mode,
                        chroma,
                        seed,
                    );
                }
            }
        }
    }
}

fn assert_contract(
    contract: ContrastContract,
    tokens: &SemanticTokens,
    template: &str,
    mode: ThemeMode,
    chroma: ChromaStrategy,
    seed: &str,
) {
    let foreground = contract.foreground.value(tokens);
    let background = contract.background.value(tokens);
    let ratio = contrast_ratio(foreground, background).expect("token colors should be valid");

    if ratio < contract.minimum_ratio {
        let apca = apca_contrast_score(foreground, background)
            .expect("APCA diagnostics should compute for valid token colors");

        panic!(
            "{} on {} ({}) has contrast {ratio:.3}:1 and APCA {apca:.1} Lc; expected at least {:.1}:1 (template={template}, mode={}, chroma={chroma}, seed={seed}, foreground={foreground}, background={background})",
            contract.foreground.name(),
            contract.background.name(),
            contract.usage,
            contract.minimum_ratio,
            mode,
        );
    }
}

impl Token {
    fn name(self) -> &'static str {
        match self {
            Self::Bg => "bg",
            Self::BgSecondary => "bg_secondary",
            Self::Surface => "surface",
            Self::SurfaceElevated => "surface_elevated",
            Self::Text => "text",
            Self::TextMuted => "text_muted",
            Self::Accent => "accent",
            Self::AccentHover => "accent_hover",
            Self::AccentActive => "accent_active",
            Self::AccentFg => "accent_fg",
            Self::Selection => "selection",
            Self::Link => "link",
            Self::Success => "success",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }

    fn value(self, tokens: &SemanticTokens) -> &str {
        match self {
            Self::Bg => &tokens.bg,
            Self::BgSecondary => &tokens.bg_secondary,
            Self::Surface => &tokens.surface,
            Self::SurfaceElevated => &tokens.surface_elevated,
            Self::Text => &tokens.text,
            Self::TextMuted => &tokens.text_muted,
            Self::Accent => &tokens.accent,
            Self::AccentHover => &tokens.accent_hover,
            Self::AccentActive => &tokens.accent_active,
            Self::AccentFg => &tokens.accent_fg,
            Self::Selection => &tokens.selection,
            Self::Link => &tokens.link,
            Self::Success => &tokens.success,
            Self::Warning => &tokens.warning,
            Self::Error => &tokens.error,
        }
    }
}
