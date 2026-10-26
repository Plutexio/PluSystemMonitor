//! Colour schemes: Material 3's colour roles, from the desktop's Matugen
//! scheme, the app's own accent, a built-in "rice" palette or a theme file of
//! the user's. QML derives the layer, hover and pressed colours from them
//! (`design/MaterialScheme.qml`).

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use material_colors::blend::harmonize;
use material_colors::color::Rgb;
use material_colors::dynamic_color::{DynamicScheme, Variant};
use material_colors::hct::Hct;
use material_colors::palette::TonalPalette;
use serde::Deserialize;
use serde_json::{Map, Value as Json, json};

/// Follows the desktop's Matugen scheme, or the accent while there is none.
pub const DEFAULT_SOURCE: &str = "system";
const ACCENT_SOURCE: &str = "plusysmon";
/// The app's own seed: the violet the monitor has always had.
const ACCENT: u32 = 0x8b5cf6;

/// Material's colour roles, in the order `Roles` holds them, with their QML
/// names and how a dynamic scheme computes each.
type RoleGetter = fn(&DynamicScheme) -> Rgb;

const ROLES: [(&str, RoleGetter); 49] = [
    ("background", DynamicScheme::background),
    ("onBackground", DynamicScheme::on_background),
    ("surface", DynamicScheme::surface),
    ("surfaceDim", DynamicScheme::surface_dim),
    ("surfaceBright", DynamicScheme::surface_bright),
    ("surfaceContainerLowest", DynamicScheme::surface_container_lowest),
    ("surfaceContainerLow", DynamicScheme::surface_container_low),
    ("surfaceContainer", DynamicScheme::surface_container),
    ("surfaceContainerHigh", DynamicScheme::surface_container_high),
    ("surfaceContainerHighest", DynamicScheme::surface_container_highest),
    ("onSurface", DynamicScheme::on_surface),
    ("surfaceVariant", DynamicScheme::surface_variant),
    ("onSurfaceVariant", DynamicScheme::on_surface_variant),
    ("inverseSurface", DynamicScheme::inverse_surface),
    ("inverseOnSurface", DynamicScheme::inverse_on_surface),
    ("outline", DynamicScheme::outline),
    ("outlineVariant", DynamicScheme::outline_variant),
    ("shadow", DynamicScheme::shadow),
    ("scrim", DynamicScheme::scrim),
    ("surfaceTint", DynamicScheme::surface_tint),
    ("primary", DynamicScheme::primary),
    ("onPrimary", DynamicScheme::on_primary),
    ("primaryContainer", DynamicScheme::primary_container),
    ("onPrimaryContainer", DynamicScheme::on_primary_container),
    ("inversePrimary", DynamicScheme::inverse_primary),
    ("secondary", DynamicScheme::secondary),
    ("onSecondary", DynamicScheme::on_secondary),
    ("secondaryContainer", DynamicScheme::secondary_container),
    ("onSecondaryContainer", DynamicScheme::on_secondary_container),
    ("tertiary", DynamicScheme::tertiary),
    ("onTertiary", DynamicScheme::on_tertiary),
    ("tertiaryContainer", DynamicScheme::tertiary_container),
    ("onTertiaryContainer", DynamicScheme::on_tertiary_container),
    ("error", DynamicScheme::error),
    ("onError", DynamicScheme::on_error),
    ("errorContainer", DynamicScheme::error_container),
    ("onErrorContainer", DynamicScheme::on_error_container),
    ("primaryFixed", DynamicScheme::primary_fixed),
    ("primaryFixedDim", DynamicScheme::primary_fixed_dim),
    ("onPrimaryFixed", DynamicScheme::on_primary_fixed),
    ("onPrimaryFixedVariant", DynamicScheme::on_primary_fixed_variant),
    ("secondaryFixed", DynamicScheme::secondary_fixed),
    ("secondaryFixedDim", DynamicScheme::secondary_fixed_dim),
    ("onSecondaryFixed", DynamicScheme::on_secondary_fixed),
    ("onSecondaryFixedVariant", DynamicScheme::on_secondary_fixed_variant),
    ("tertiaryFixed", DynamicScheme::tertiary_fixed),
    ("tertiaryFixedDim", DynamicScheme::tertiary_fixed_dim),
    ("onTertiaryFixed", DynamicScheme::on_tertiary_fixed),
    ("onTertiaryFixedVariant", DynamicScheme::on_tertiary_fixed_variant),
];

#[derive(Clone, Copy)]
enum Role {
    Background = 0,
    OnBackground = 1,
    Surface = 2,
    SurfaceDim = 3,
    SurfaceBright = 4,
    SurfaceContainerLowest = 5,
    SurfaceContainerLow = 6,
    SurfaceContainer = 7,
    SurfaceContainerHigh = 8,
    SurfaceContainerHighest = 9,
    OnSurface = 10,
    SurfaceVariant = 11,
    OnSurfaceVariant = 12,
    InverseSurface = 13,
    InverseOnSurface = 14,
    Outline = 15,
    OutlineVariant = 16,
    Shadow = 17,
    Scrim = 18,
    SurfaceTint = 19,
    Primary = 20,
    OnPrimary = 21,
    PrimaryContainer = 22,
    OnPrimaryContainer = 23,
    InversePrimary = 24,
    Secondary = 25,
    OnSecondary = 26,
    SecondaryContainer = 27,
    OnSecondaryContainer = 28,
    Tertiary = 29,
    OnTertiary = 30,
    TertiaryContainer = 31,
    OnTertiaryContainer = 32,
    Error = 33,
    OnError = 34,
    ErrorContainer = 35,
    OnErrorContainer = 36,
    PrimaryFixed = 37,
    PrimaryFixedDim = 38,
    OnPrimaryFixed = 39,
    OnPrimaryFixedVariant = 40,
    SecondaryFixed = 41,
    SecondaryFixedDim = 42,
    OnSecondaryFixed = 43,
    OnSecondaryFixedVariant = 44,
    TertiaryFixed = 45,
    TertiaryFixedDim = 46,
    OnTertiaryFixed = 47,
    OnTertiaryFixedVariant = 48,
}

type Roles = [Rgb; 49];

/// The colours the data is drawn in. Fixed hues so a CPU line stays violet-ish
/// in every theme, each pulled a little toward the scheme's primary so they
/// sit in it, at the tone the scheme's own accents use.
const DATA_HUES: [(&str, u32); 8] = [
    ("cpu", 0x8b5cf6),
    ("memory", 0x22d3ee),
    ("swap", 0xf472b6),
    ("gpu", 0xa3e635),
    ("network", 0x60a5fa),
    ("disk", 0xfb923c),
    ("good", 0x34d399),
    ("warn", 0xfbbf24),
];

pub struct Scheme {
    pub dark: bool,
    roles: Roles,
    /// The theme the scheme came from: the requested one, or what stood in.
    pub source: String,
}

pub struct Resolved {
    pub scheme: Scheme,
    /// Files and folders whose changes should rebuild the scheme.
    pub watch: Vec<PathBuf>,
}

fn rgb(value: u32) -> Rgb {
    Rgb::new((value >> 16) as u8, (value >> 8) as u8, value as u8)
}

fn hex(color: Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", color.red, color.green, color.blue)
}

/// `amount` of `a`, the rest of `b` (end4's ColorUtils.mix).
fn mix(a: Rgb, b: Rgb, amount: f64) -> Rgb {
    let t = amount.clamp(0.0, 1.0);
    let channel = |x: u8, y: u8| (f64::from(x) * t + f64::from(y) * (1.0 - t)).round() as u8;
    Rgb::new(
        channel(a.red, b.red),
        channel(a.green, b.green),
        channel(a.blue, b.blue),
    )
}

fn linear(channel: u8) -> f64 {
    let c = f64::from(channel) / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn relative_luminance(color: Rgb) -> f64 {
    0.2126 * linear(color.red) + 0.7152 * linear(color.green) + 0.0722 * linear(color.blue)
}

fn contrast(a: Rgb, b: Rgb) -> f64 {
    let (one, two) = (relative_luminance(a), relative_luminance(b));
    (one.max(two) + 0.05) / (one.min(two) + 0.05)
}

/// Rec. 709 weights on the gamma-encoded channels: what tells a dark
/// background from a light one.
fn luminance709(color: Rgb) -> f64 {
    (0.2126 * f64::from(color.red) + 0.7152 * f64::from(color.green) + 0.0722 * f64::from(color.blue))
        / 255.0
}

fn from_dynamic(scheme: &DynamicScheme) -> Roles {
    ROLES.map(|(_, role)| role(scheme))
}

fn from_seed(seed: Rgb, variant: Variant, dark: bool) -> Roles {
    from_dynamic(&DynamicScheme::by_variant(seed, &variant, dark, None))
}

impl Scheme {
    fn new(roles: Roles, dark: bool, source: &str) -> Self {
        Self {
            dark,
            roles,
            source: source.to_owned(),
        }
    }

    fn role(&self, role: Role) -> Rgb {
        self.roles[role as usize]
    }

    /// `{ dark, source, roles: { background: "#…", … }, data: { cpu: "#…", … } }`
    pub fn to_json(&self) -> Json {
        let roles: Map<String, Json> = ROLES
            .iter()
            .zip(self.roles)
            .map(|((name, _), color)| ((*name).to_owned(), Json::String(hex(color))))
            .collect();
        let primary = self.role(Role::Primary);
        let data: Map<String, Json> = DATA_HUES
            .iter()
            .map(|(name, base)| {
                let hct = Hct::new(harmonize(rgb(*base), primary));
                let tone = if self.dark { 80 } else { 40 };
                let color = TonalPalette::of(hct.get_hue(), hct.get_chroma().max(48.0)).tone(tone);
                ((*name).to_owned(), Json::String(hex(color)))
            })
            .collect();
        json!({ "dark": self.dark, "source": self.source, "roles": roles, "data": data })
    }

    /// What the theme picker draws a swatch from.
    fn swatch(&self) -> Json {
        json!({
            "background": hex(self.role(Role::Background)),
            "surface": hex(self.role(Role::SurfaceContainerLow)),
            "text": hex(self.role(Role::OnSurface)),
            "primary": hex(self.role(Role::Primary)),
            "secondary": hex(self.role(Role::Secondary)),
            "tertiary": hex(self.role(Role::Tertiary)),
        })
    }
}

// --- rice palettes ------------------------------------------------------------

/// Thirteen colours a desktop "rice" theme names, mapped onto Material's roles
/// so the theme keeps its own grounds and accents instead of being regenerated
/// from a seed.
#[derive(Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
struct Palette {
    lowest: HexColor,
    window: HexColor,
    page: HexColor,
    raised: HexColor,
    raised_more: HexColor,
    line: HexColor,
    text: HexColor,
    subtext: HexColor,
    muted: HexColor,
    primary: HexColor,
    secondary: HexColor,
    tertiary: HexColor,
    error: HexColor,
}

#[derive(Clone, Copy)]
struct HexColor(Rgb);

impl<'de> Deserialize<'de> for HexColor {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        parse_hex(&raw).map(HexColor).ok_or_else(|| {
            serde::de::Error::custom(format!("invalid colour {raw:?}, expected #rgb or #rrggbb"))
        })
    }
}

fn parse_hex(raw: &str) -> Option<Rgb> {
    let digits = raw.strip_prefix('#')?;
    if !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let value = u32::from_str_radix(digits, 16).ok()?;
    match digits.len() {
        // Each digit doubled, as in CSS.
        3 => Some(rgb(
            ((value >> 8) & 0xf) * 0x110000 + ((value >> 4) & 0xf) * 0x1100 + (value & 0xf) * 0x11,
        )),
        6 => Some(rgb(value)),
        _ => None,
    }
}

const fn palette(c: [u32; 13]) -> [u32; 13] {
    c
}

fn palette_from(c: [u32; 13]) -> Palette {
    let h = |i: usize| HexColor(rgb(c[i]));
    Palette {
        lowest: h(0),
        window: h(1),
        page: h(2),
        raised: h(3),
        raised_more: h(4),
        line: h(5),
        text: h(6),
        subtext: h(7),
        muted: h(8),
        primary: h(9),
        secondary: h(10),
        tertiary: h(11),
        error: h(12),
    }
}

struct Family {
    id: &'static str,
    name: &'static str,
    light: [u32; 13],
    dark: [u32; 13],
}

// lowest, window, page, raised, raised_more, line; text, subtext, muted;
// primary, secondary, tertiary, error. Shades a palette has no name for are
// blended from its own neighbours.
const LATTE: [u32; 13] = palette([
    0xeff1f5, 0xe6e9ef, 0xeff1f5, 0xdce0e8, 0xccd0da, 0xbcc0cc, 0x4c4f69, 0x6c6f85, 0x8c8fa1,
    0x8839ef, 0xea76cb, 0x179299, 0xd20f39,
]);
const ROSE_PINE_DAWN: [u32; 13] = palette([
    0xfffaf3, 0xfaf4ed, 0xfffaf3, 0xf2e9e1, 0xdfdad9, 0xcecacd, 0x575279, 0x797593, 0x9893a5,
    0x907aa9, 0xd7827e, 0x286983, 0xb4637a,
]);
const TOKYO_DAY: [u32; 13] = palette([
    0xe9e9ed, 0xd0d5e3, 0xe1e2e7, 0xc4c8da, 0xb7bcd2, 0xa8aecb, 0x2f51a2, 0x6172b0, 0x848cb5,
    0x2e7de9, 0x9854f1, 0x007197, 0xf52a65,
]);

const FAMILIES: &[Family] = &[
    Family {
        id: "default",
        name: "Default",
        light: [
            0xffffff, 0xf2f2f3, 0xffffff, 0xebebec, 0xe1e1e3, 0xd0d0d3, 0x1b1b1d, 0x5c5c61, 0x8a8a90,
            0x1b1b1d, 0x5c5c61, 0x6f6f75, 0xc62828,
        ],
        dark: [
            0x0c0c0d, 0x111113, 0x19191b, 0x232326, 0x2e2e31, 0x3b3b3f, 0xececee, 0xa9a9ae, 0x7c7c82,
            0xececee, 0xb4b4b9, 0x9a9aa0, 0xef5350,
        ],
    },
    Family {
        id: "catppuccin-mocha",
        name: "Catppuccin Mocha",
        light: LATTE,
        dark: [
            0x11111b, 0x181825, 0x1e1e2e, 0x313244, 0x45475a, 0x585b70, 0xcdd6f4, 0xa6adc8, 0x7f849c,
            0xcba6f7, 0xf5c2e7, 0x94e2d5, 0xf38ba8,
        ],
    },
    Family {
        id: "catppuccin-macchiato",
        name: "Catppuccin Macchiato",
        light: LATTE,
        dark: [
            0x181926, 0x1e2030, 0x24273a, 0x363a4f, 0x494d64, 0x5b6078, 0xcad3f5, 0xa5adcb, 0x8087a2,
            0xc6a0f6, 0xf5bde6, 0x8bd5ca, 0xed8796,
        ],
    },
    Family {
        id: "catppuccin-frappe",
        name: "Catppuccin Frappé",
        light: LATTE,
        dark: [
            0x232634, 0x292c3c, 0x303446, 0x414559, 0x51576d, 0x626880, 0xc6d0f5, 0xa5adce, 0x838ba7,
            0xca9ee6, 0xf4b8e4, 0x81c8be, 0xe78284,
        ],
    },
    Family {
        id: "rosepine",
        name: "Rosé Pine",
        light: ROSE_PINE_DAWN,
        dark: [
            0x191724, 0x191724, 0x1f1d2e, 0x26233a, 0x403d52, 0x524f67, 0xe0def4, 0x908caa, 0x6e6a86,
            0xc4a7e7, 0xebbcba, 0x9ccfd8, 0xeb6f92,
        ],
    },
    Family {
        id: "rosepine-moon",
        name: "Rosé Pine Moon",
        light: ROSE_PINE_DAWN,
        dark: [
            0x232136, 0x232136, 0x2a273f, 0x393552, 0x44415a, 0x56526e, 0xe0def4, 0x908caa, 0x6e6a86,
            0xc4a7e7, 0xea9a97, 0x9ccfd8, 0xeb6f92,
        ],
    },
    Family {
        id: "gruvbox",
        name: "Gruvbox",
        light: [
            0xf9f5d7, 0xf2e5bc, 0xfbf1c7, 0xebdbb2, 0xd5c4a1, 0xbdae93, 0x3c3836, 0x7c6f64, 0x928374,
            0xaf3a03, 0xb57614, 0x427b58, 0x9d0006,
        ],
        dark: [
            0x1d2021, 0x1d2021, 0x282828, 0x3c3836, 0x504945, 0x665c54, 0xebdbb2, 0xa89984, 0x928374,
            0xfe8019, 0xfabd2f, 0x8ec07c, 0xfb4934,
        ],
    },
    Family {
        id: "nord",
        name: "Nord",
        light: [
            0xeceff4, 0xe5e9f0, 0xeceff4, 0xd8dee9, 0xc8d0dc, 0xb9c2d0, 0x2e3440, 0x4c566a, 0x7b8394,
            0x5e81ac, 0xb48ead, 0x8fbcbb, 0xbf616a,
        ],
        dark: [
            0x2e3440, 0x2e3440, 0x3b4252, 0x434c5e, 0x4c566a, 0x4c566a, 0xeceff4, 0xd8dee9, 0x929aaa,
            0x88c0d0, 0x81a1c1, 0xa3be8c, 0xbf616a,
        ],
    },
    Family {
        id: "tokyonight",
        name: "Tokyo Night",
        light: TOKYO_DAY,
        dark: [
            0x16161e, 0x16161e, 0x1a1b26, 0x292e42, 0x3b4261, 0x414868, 0xc0caf5, 0xa9b1d6, 0x565f89,
            0x7aa2f7, 0xbb9af7, 0x7dcfff, 0xf7768e,
        ],
    },
    Family {
        id: "tokyonight-storm",
        name: "Tokyo Night Storm",
        light: TOKYO_DAY,
        dark: [
            0x1f2335, 0x1f2335, 0x24283b, 0x292e42, 0x3b4261, 0x414868, 0xc0caf5, 0xa9b1d6, 0x565f89,
            0x7aa2f7, 0xbb9af7, 0x7dcfff, 0xf7768e,
        ],
    },
    Family {
        id: "dracula",
        name: "Dracula",
        light: [
            0xfffbeb, 0xf2eedc, 0xfffbeb, 0xe8e4d0, 0xcfcfde, 0xcfcfde, 0x1f1f1f, 0x45423a, 0x6c664b,
            0x644ac9, 0xa3144d, 0x036a96, 0xcb3a2a,
        ],
        dark: [
            0x191a21, 0x21222c, 0x282a36, 0x343746, 0x44475a, 0x44475a, 0xf8f8f2, 0xc3c6d7, 0x6272a4,
            0xbd93f9, 0xff79c6, 0x8be9fd, 0xff5555,
        ],
    },
    Family {
        id: "everforest",
        name: "Everforest",
        light: [
            0xfdf6e3, 0xefebd4, 0xfdf6e3, 0xe6e2cc, 0xe0dcc7, 0xbdc3af, 0x5c6a72, 0x829181, 0x939f91,
            0x8da101, 0x35a77c, 0xdfa000, 0xf85552,
        ],
        dark: [
            0x232a2e, 0x232a2e, 0x2d353b, 0x3d484d, 0x475258, 0x4f585e, 0xd3c6aa, 0x9da9a0, 0x859289,
            0xa7c080, 0x83c092, 0xdbbc7f, 0xe67e80,
        ],
    },
    Family {
        id: "kanagawa",
        name: "Kanagawa",
        light: [
            0xf2ecbc, 0xe5ddb0, 0xf2ecbc, 0xe7dba0, 0xe4d794, 0xc9c19a, 0x545464, 0x716e61, 0x8a8980,
            0x4d699b, 0x624c83, 0x6f894e, 0xc84053,
        ],
        dark: [
            0x16161d, 0x16161d, 0x1f1f28, 0x2a2a37, 0x363646, 0x54546d, 0xdcd7ba, 0xc8c093, 0x727169,
            0x7e9cd8, 0x957fb8, 0x98bb6c, 0xe46876,
        ],
    },
    Family {
        id: "solarized",
        name: "Solarized",
        light: [
            0xfdf6e3, 0xeee8d5, 0xfdf6e3, 0xe6dfcb, 0xddd6c1, 0x93a1a1, 0x073642, 0x586e75, 0x93a1a1,
            0x268bd2, 0x2aa198, 0xb58900, 0xdc322f,
        ],
        dark: [
            0x00212b, 0x00212b, 0x002b36, 0x073642, 0x0d4452, 0x586e75, 0x93a1a1, 0x839496, 0x586e75,
            0x268bd2, 0x2aa198, 0xb58900, 0xdc322f,
        ],
    },
    Family {
        id: "one",
        name: "One",
        light: [
            0xffffff, 0xeaeaeb, 0xfafafa, 0xe5e5e6, 0xdbdbdc, 0xd4d4d4, 0x383a42, 0x696c77, 0xa0a1a7,
            0x4078f2, 0xa626a4, 0x50a14f, 0xe45649,
        ],
        dark: [
            0x1b1d23, 0x21252b, 0x282c34, 0x2c313a, 0x3e4451, 0x4b5263, 0xabb2bf, 0x9da5b4, 0x5c6370,
            0x61afef, 0xc678dd, 0x98c379, 0xe06c75,
        ],
    },
];

/// A rice palette as Material roles. Text keeps at least 4.5:1 against what
/// it sits on: where the palette's own ink falls short, a blend toward the
/// text (or plain black or white) stands in.
fn roles_from_palette(p: &Palette, dark: bool) -> Roles {
    let mut roles = [Rgb::new(0, 0, 0); 49];
    let mut set = |role: Role, value: Rgb| roles[role as usize] = value;
    let (lowest, window, page) = (p.lowest.0, p.window.0, p.page.0);
    let (raised, raised_more, text) = (p.raised.0, p.raised_more.0, p.text.0);
    let (black, white) = (Rgb::new(0, 0, 0), Rgb::new(255, 255, 255));

    set(Role::Background, window);
    set(Role::OnBackground, text);
    set(Role::Surface, page);
    set(Role::SurfaceDim, if dark { lowest } else { mix(page, raised_more, 0.4) });
    set(Role::SurfaceBright, if dark { raised_more } else { lowest });
    set(Role::SurfaceContainerLowest, lowest);
    set(Role::SurfaceContainerLow, page);
    set(Role::SurfaceContainer, mix(page, raised, 0.5));
    set(Role::SurfaceContainerHigh, raised);
    set(Role::SurfaceContainerHighest, raised_more);
    set(Role::OnSurface, text);
    set(Role::SurfaceVariant, raised);
    set(Role::OnSurfaceVariant, p.subtext.0);
    set(Role::InverseSurface, text);
    set(Role::InverseOnSurface, page);
    set(Role::Outline, p.muted.0);
    set(Role::OutlineVariant, p.line.0);
    set(Role::Shadow, black);
    set(Role::Scrim, black);
    set(Role::SurfaceTint, p.primary.0);

    let (dark_ink, light_ink) = if dark { (lowest, text) } else { (text, lowest) };
    let onto = |ground: Rgb| {
        let themed = if contrast(ground, dark_ink) >= contrast(ground, light_ink) {
            dark_ink
        } else {
            light_ink
        };
        if contrast(ground, themed) >= 4.5 {
            themed
        } else if contrast(ground, black) >= contrast(ground, white) {
            black
        } else {
            white
        }
    };
    let ink_on = |tone: Rgb, ground: Rgb| {
        let mut amount = 0.45;
        while amount > 0.001 {
            let candidate = mix(tone, text, amount);
            if contrast(candidate, ground) >= 4.5 {
                return candidate;
            }
            amount -= 0.05;
        }
        // Not even the text reads on this wash: black or white does.
        if contrast(text, ground) >= 4.5 {
            text
        } else if contrast(ground, black) >= contrast(ground, white) {
            mix(text, black, 0.3)
        } else {
            mix(text, white, 0.3)
        }
    };
    let accents = [
        (p.primary.0, Role::Primary, Role::OnPrimary, Role::PrimaryContainer, Role::OnPrimaryContainer),
        (p.secondary.0, Role::Secondary, Role::OnSecondary, Role::SecondaryContainer, Role::OnSecondaryContainer),
        (p.tertiary.0, Role::Tertiary, Role::OnTertiary, Role::TertiaryContainer, Role::OnTertiaryContainer),
        (p.error.0, Role::Error, Role::OnError, Role::ErrorContainer, Role::OnErrorContainer),
    ];
    for (tone, role, on, container, on_container) in accents {
        let wash = mix(tone, page, if dark { 0.3 } else { 0.22 });
        set(role, tone);
        set(on, onto(tone));
        set(container, wash);
        set(on_container, ink_on(tone, wash));
    }
    set(Role::InversePrimary, mix(p.primary.0, if dark { black } else { white }, 0.6));

    // The fixed roles are the same in light and dark: a pale and a deeper
    // tint of the accent, and dark ink for each.
    let fixed = [
        (p.primary.0, Role::PrimaryFixed, Role::PrimaryFixedDim, Role::OnPrimaryFixed, Role::OnPrimaryFixedVariant),
        (p.secondary.0, Role::SecondaryFixed, Role::SecondaryFixedDim, Role::OnSecondaryFixed, Role::OnSecondaryFixedVariant),
        (p.tertiary.0, Role::TertiaryFixed, Role::TertiaryFixedDim, Role::OnTertiaryFixed, Role::OnTertiaryFixedVariant),
    ];
    for (tone, pale, deeper, on, on_variant) in fixed {
        set(pale, mix(tone, white, 0.3));
        set(deeper, mix(tone, white, 0.6));
        set(on, mix(tone, black, 0.2));
        set(on_variant, mix(tone, black, 0.5));
    }
    roles
}

// --- Matugen --------------------------------------------------------------------

fn xdg_dir(variable: &str, fallback: &str) -> Option<PathBuf> {
    match std::env::var_os(variable).map(PathBuf::from) {
        Some(path) if path.is_absolute() => Some(path),
        _ => std::env::var_os("HOME").map(|home| PathBuf::from(home).join(fallback)),
    }
}

/// Where desktops leave their generated Material scheme, most specific first:
/// end4's quickshell, then Matugen's own folders.
fn matugen_roots() -> Vec<PathBuf> {
    let config = xdg_dir("XDG_CONFIG_HOME", ".config");
    let cache = xdg_dir("XDG_CACHE_HOME", ".cache");
    let state = xdg_dir("XDG_STATE_HOME", ".local/state");
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let mut roots: Vec<PathBuf> = [
        state.as_ref().map(|s| s.join("quickshell/user/generated")),
        config.as_ref().map(|c| c.join("matugen")),
        cache.as_ref().map(|c| c.join("matugen")),
        state.as_ref().map(|s| s.join("matugen")),
    ]
    .into_iter()
    .flatten()
    .collect();
    // A GUI launch can have other XDG variables than the shell that ran Matugen.
    if let Some(home) = home {
        for base in [".config", ".cache", ".local/state"] {
            roots.push(home.join(base).join("matugen"));
        }
    }
    let mut seen = HashSet::new();
    roots.retain(|root| seen.insert(root.clone()));
    roots
}

/// Desktop shells that run Matugen into a folder of their own, among files
/// that are not schemes, so they are named rather than searched for.
fn shell_scheme_files() -> Vec<PathBuf> {
    xdg_dir("XDG_CACHE_HOME", ".cache")
        .map(|cache| vec![cache.join("DankMaterialShell/dms-colors.json")])
        .unwrap_or_default()
}

fn existing_ancestor(path: &Path) -> PathBuf {
    let mut current = path.to_path_buf();
    while !current.exists() {
        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => break,
        }
    }
    current
}

/// Every JSON file that might be a scheme, `colors.json` first in each folder,
/// two levels deep; and the folders to watch for new ones.
fn matugen_candidates() -> (Vec<PathBuf>, Vec<PathBuf>) {
    fn scan(dir: &Path, depth: u32, files: &mut Vec<PathBuf>, dirs: &mut Vec<PathBuf>) {
        if files.len() >= 512 || dirs.len() >= 64 {
            return;
        }
        let ancestor = existing_ancestor(dir);
        if !dirs.contains(&ancestor) {
            dirs.push(ancestor);
        }
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
        entries.sort();
        let colors = dir.join("colors.json");
        if colors.is_file() {
            files.push(colors.clone());
        }
        for path in &entries {
            if path.is_file() && path.extension().is_some_and(|e| e == "json") && *path != colors {
                files.push(path.clone());
            }
        }
        if depth >= 2 {
            return;
        }
        for path in &entries {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
            if path.is_dir()
                && !path.is_symlink()
                && !matches!(name.as_str(), "templates" | "template" | "images")
                && !name.starts_with('.')
            {
                scan(path, depth + 1, files, dirs);
            }
        }
    }

    let mut files = Vec::new();
    let mut dirs = Vec::new();
    for root in matugen_roots() {
        scan(&root, 0, &mut files, &mut dirs);
    }
    // After end4's generated scheme, before Matugen's own folders.
    let shell_index = files
        .iter()
        .take_while(|f| f.to_string_lossy().contains("/quickshell/user/generated/"))
        .count();
    for (offset, path) in shell_scheme_files().into_iter().enumerate() {
        if path.is_file() {
            files.insert(shell_index + offset, path);
        } else if let Some(parent) = path.parent() {
            dirs.push(existing_ancestor(parent));
        }
    }
    let mut seen = HashSet::new();
    files.retain(|f| seen.insert(f.clone()));
    (files, dirs)
}

/// A colour as Matugen writes it: hex (with or without `#`), CSS `rgb()` or
/// `hsl()`, or an object holding one per mode.
fn matugen_color(mut value: &Json, dark: bool) -> Option<Rgb> {
    let mode = if dark { "dark" } else { "light" };
    let other = if dark { "light" } else { "dark" };
    for _ in 0..6 {
        let Json::Object(object) = value else { break };
        value = [mode, "default", "color", "hex", "value", other]
            .iter()
            .find_map(|key| object.get(*key))?;
    }
    let text = value.as_str()?.trim();
    if let Some(color) = parse_hex(text) {
        return Some(color);
    }
    if text.len() == 6 && text.chars().all(|c| c.is_ascii_hexdigit()) {
        return parse_hex(&format!("#{text}"));
    }
    let lower = text.to_ascii_lowercase();
    let (format, args) = lower.split_once('(')?;
    let args = args.strip_suffix(')')?;
    let parts: Vec<&str> = args
        .split(|c: char| c == ',' || c == '/' || c.is_whitespace())
        .filter(|s| !s.is_empty())
        .collect();
    if parts.len() != if format.ends_with('a') { 4 } else { 3 } {
        return None;
    }
    let unit = |part: &str, maximum: f64| -> Option<f64> {
        let (number, maximum) = match part.strip_suffix('%') {
            Some(number) => (number, 100.0),
            None => (part, maximum),
        };
        let value: f64 = number.parse().ok()?;
        (value.is_finite() && (0.0..=maximum).contains(&value)).then_some(value / maximum)
    };
    match format {
        "rgb" | "rgba" => {
            let (r, g, b) = (unit(parts[0], 255.0)?, unit(parts[1], 255.0)?, unit(parts[2], 255.0)?);
            let byte = |v: f64| (v * 255.0).round() as u8;
            Some(Rgb::new(byte(r), byte(g), byte(b)))
        }
        "hsl" | "hsla" => {
            let hue: f64 = parts[0].trim_end_matches("deg").parse().ok()?;
            let (s, l) = (unit(parts[1], 1.0)?, unit(parts[2], 1.0)?);
            let h = hue.rem_euclid(360.0) / 60.0;
            let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
            let x = c * (1.0 - (h % 2.0 - 1.0).abs());
            let (r, g, b) = match h as u32 {
                0 => (c, x, 0.0),
                1 => (x, c, 0.0),
                2 => (0.0, c, x),
                3 => (0.0, x, c),
                4 => (x, 0.0, c),
                _ => (c, 0.0, x),
            };
            let m = l - c / 2.0;
            let byte = |v: f64| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8;
            Some(Rgb::new(byte(r), byte(g), byte(b)))
        }
        _ => None,
    }
}

fn snake_case(camel: &str) -> String {
    let mut out = String::with_capacity(camel.len() + 4);
    for c in camel.chars() {
        if c.is_ascii_uppercase() {
            out.push('_');
        }
        out.push(c.to_ascii_lowercase());
    }
    out
}

/// A Material scheme from a Matugen export or a desktop template. Anything
/// that isn't one (an unrelated JSON file, a template with unresolved
/// placeholders) is turned down.
fn roles_from_matugen(text: &str, dark: bool) -> Option<(Roles, bool)> {
    let document: Json = serde_json::from_str(text).ok()?;
    let mut object = document.as_object()?;
    let mode = if dark { "dark" } else { "light" };
    let other = if dark { "light" } else { "dark" };
    // Both the CLI export and custom templates can wrap their schemes.
    for _ in 0..4 {
        match ["colors", "schemes", mode, "default", other]
            .iter()
            .find_map(|key| object.get(*key).and_then(Json::as_object))
        {
            Some(inner) => object = inner,
            None => break,
        }
    }

    let mut found: [Option<Rgb>; 49] = [None; 49];
    for (index, (camel, _)) in ROLES.iter().enumerate() {
        let mut prefixed = String::from("m3");
        prefixed.push_str(&camel[..1].to_ascii_uppercase());
        prefixed.push_str(&camel[1..]);
        let value = [snake_case(camel), (*camel).to_owned(), prefixed]
            .iter()
            .find_map(|key| object.get(key));
        if let Some(value) = value {
            found[index] = Some(matugen_color(value, dark)?);
        }
    }
    let required = [
        Role::Background, Role::OnBackground, Role::Surface, Role::OnSurface, Role::Primary,
        Role::OnPrimary, Role::Secondary, Role::OnSecondary, Role::Tertiary, Role::OnTertiary,
        Role::Error, Role::OnError, Role::Outline,
    ];
    if required.iter().any(|role| found[*role as usize].is_none()) {
        return None;
    }
    let background = found[Role::Background as usize]?;
    let is_dark = luminance709(background) < 0.5;
    let fallback = from_seed(found[Role::Primary as usize]?, Variant::TonalSpot, is_dark);
    let roles = std::array::from_fn(|i| found[i].unwrap_or(fallback[i]));
    Some((roles, is_dark))
}

fn read_matugen(path: &Path, dark: bool) -> Option<(Roles, bool)> {
    let meta = fs::metadata(path).ok()?;
    // Schemes are small; an unrelated, huge JSON cache is not one.
    if !meta.is_file() || meta.len() > 4 * 1024 * 1024 {
        return None;
    }
    roles_from_matugen(&fs::read_to_string(path).ok()?, dark)
}

/// The desktop's scheme, if it has one, and what to watch for it changing.
fn system_scheme(dark: bool) -> (Option<(Roles, bool)>, Vec<PathBuf>) {
    let (files, mut watch) = matugen_candidates();
    for path in files {
        if let Some(found) = read_matugen(&path, dark) {
            watch.push(path);
            return (Some(found), watch);
        }
    }
    (None, watch)
}

// --- theme files ------------------------------------------------------------------

/// A theme file: a seed to generate a scheme from, or rice palettes for light
/// and dark (one is enough; it is used for both).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ThemeFile {
    name: Option<String>,
    seed: Option<HexColor>,
    #[serde(default)]
    variant: VariantName,
    light: Option<Palette>,
    dark: Option<Palette>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "snake_case")]
enum VariantName {
    #[default]
    TonalSpot,
    Content,
    Fidelity,
    Vibrant,
    Expressive,
    Neutral,
    Monochrome,
    Rainbow,
    FruitSalad,
}

impl From<&VariantName> for Variant {
    fn from(name: &VariantName) -> Self {
        match name {
            VariantName::TonalSpot => Variant::TonalSpot,
            VariantName::Content => Variant::Content,
            VariantName::Fidelity => Variant::Fidelity,
            VariantName::Vibrant => Variant::Vibrant,
            VariantName::Expressive => Variant::Expressive,
            VariantName::Neutral => Variant::Neutral,
            VariantName::Monochrome => Variant::Monochrome,
            VariantName::Rainbow => Variant::Rainbow,
            VariantName::FruitSalad => Variant::FruitSalad,
        }
    }
}

fn read_theme_file(path: &Path) -> Result<ThemeFile> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let table: toml::Table = text.parse().with_context(|| format!("{}", path.display()))?;
    if ["colors", "accents", "shape", "effects", "extends"]
        .iter()
        .any(|key| table.contains_key(*key))
    {
        bail!(
            "{} is written for the old theme format; give it a `seed` colour or \
             [dark]/[light] palettes instead",
            path.display()
        );
    }
    let theme: ThemeFile = toml::Value::Table(table)
        .try_into()
        .with_context(|| format!("{}", path.display()))?;
    if theme.seed.is_none() && theme.light.is_none() && theme.dark.is_none() {
        bail!("{} needs a `seed` colour or a [dark] or [light] palette", path.display());
    }
    Ok(theme)
}

fn roles_from_file(theme: &ThemeFile, dark: bool) -> Roles {
    let palette = if dark {
        theme.dark.as_ref().or(theme.light.as_ref())
    } else {
        theme.light.as_ref().or(theme.dark.as_ref())
    };
    match (palette, theme.seed) {
        (Some(palette), _) => roles_from_palette(palette, dark),
        (None, Some(seed)) => from_seed(seed.0, (&theme.variant).into(), dark),
        (None, None) => from_seed(rgb(ACCENT), Variant::Content, dark),
    }
}

fn user_theme_ids(user_dir: Option<&Path>) -> Vec<String> {
    let mut ids: Vec<String> = user_dir
        .and_then(|dir| fs::read_dir(dir).ok())
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            (path.extension()? == "toml").then(|| path.file_stem()?.to_str().map(str::to_owned))?
        })
        .collect();
    ids.sort();
    ids
}

// --- resolving --------------------------------------------------------------------

fn accent_scheme(dark: bool, source: &str) -> Scheme {
    Scheme::new(from_seed(rgb(ACCENT), Variant::Content, dark), dark, source)
}

/// The scheme for `id` in the given mode. A missing or broken theme falls back
/// to the accent; the error says why.
pub fn resolve(id: &str, dark: bool, user_dir: Option<&Path>) -> (Resolved, Option<anyhow::Error>) {
    if id == DEFAULT_SOURCE {
        let (found, watch) = system_scheme(dark);
        let scheme = match found {
            // The desktop decides light or dark; a Matugen export holding both
            // modes gives the one asked for.
            Some((roles, is_dark)) => Scheme::new(roles, is_dark, id),
            None => accent_scheme(dark, id),
        };
        return (Resolved { scheme, watch }, None);
    }
    if id == ACCENT_SOURCE {
        return (Resolved { scheme: accent_scheme(dark, id), watch: Vec::new() }, None);
    }
    if let Some(path) = user_dir.map(|dir| dir.join(format!("{id}.toml")))
        && path.is_file()
    {
        let watch = vec![path.clone()];
        return match read_theme_file(&path) {
            Ok(theme) => {
                let scheme = Scheme::new(roles_from_file(&theme, dark), dark, id);
                (Resolved { scheme, watch }, None)
            }
            Err(e) => (Resolved { scheme: accent_scheme(dark, ACCENT_SOURCE), watch }, Some(e)),
        };
    }
    if let Some(family) = FAMILIES.iter().find(|f| f.id == id) {
        let palette = palette_from(if dark { family.dark } else { family.light });
        let scheme = Scheme::new(roles_from_palette(&palette, dark), dark, id);
        return (Resolved { scheme, watch: Vec::new() }, None);
    }
    (
        Resolved { scheme: accent_scheme(dark, ACCENT_SOURCE), watch: Vec::new() },
        Some(anyhow::anyhow!("there is no theme named {id:?}")),
    )
}

/// Every theme with a swatch in the given mode. Broken user themes are listed
/// too, with their error, so they can be fixed.
pub fn list(dark: bool, user_dir: Option<&Path>) -> Vec<Json> {
    let entry = |id: &str, name: &str, kind: &str, scheme: &Scheme| {
        let mut value = scheme.swatch();
        let object = value.as_object_mut().expect("a swatch is an object");
        object.insert("id".into(), id.into());
        object.insert("name".into(), name.into());
        object.insert("kind".into(), kind.into());
        value
    };
    let mut themes = Vec::new();
    let (system, _) = resolve(DEFAULT_SOURCE, dark, None);
    themes.push(entry(DEFAULT_SOURCE, "Desktop", "system", &system.scheme));
    themes.push(entry(ACCENT_SOURCE, "PluSystemMonitor", "builtin", &accent_scheme(dark, ACCENT_SOURCE)));
    for family in FAMILIES {
        let palette = palette_from(if dark { family.dark } else { family.light });
        let scheme = Scheme::new(roles_from_palette(&palette, dark), dark, family.id);
        themes.push(entry(family.id, family.name, "builtin", &scheme));
    }
    for id in user_theme_ids(user_dir) {
        let path = user_dir.expect("ids come from the folder").join(format!("{id}.toml"));
        match read_theme_file(&path) {
            Ok(theme) => {
                let scheme = Scheme::new(roles_from_file(&theme, dark), dark, &id);
                let name = theme.name.clone().unwrap_or_else(|| id.clone());
                themes.push(entry(&id, &name, "custom", &scheme));
            }
            Err(e) => themes.push(json!({ "id": id, "name": id, "kind": "custom", "error": format!("{e:#}") })),
        }
    }
    themes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("plusysmon-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn role(scheme: &Scheme, name: &str) -> String {
        scheme.to_json()["roles"][name].as_str().unwrap().to_owned()
    }

    #[test]
    fn parses_hex_and_css_colours() {
        let c = |s: &str| matugen_color(&Json::String(s.into()), true).map(hex);
        assert_eq!(c("#abc").as_deref(), Some("#aabbcc"));
        assert_eq!(c("8B5CF6").as_deref(), Some("#8b5cf6"));
        assert_eq!(c("rgb(255, 0, 128)").as_deref(), Some("#ff0080"));
        assert_eq!(c("hsl(120, 100%, 50%)").as_deref(), Some("#00ff00"));
        assert_eq!(c("{{colors.primary}}"), None);
        let nested = json!({ "dark": "#010203", "light": "#fefefe" });
        assert_eq!(matugen_color(&nested, true).map(hex).as_deref(), Some("#010203"));
        assert_eq!(matugen_color(&nested, false).map(hex).as_deref(), Some("#fefefe"));
    }

    #[test]
    fn reads_matugen_exports() {
        let mut colors = Map::new();
        for (key, value) in [
            ("background", "#101010"), ("on_background", "#eeeeee"), ("surface", "#121212"),
            ("on_surface", "#eeeeee"), ("primary", "#d0bcff"), ("on_primary", "#381e72"),
            ("secondary", "#ccc2dc"), ("on_secondary", "#332d41"), ("tertiary", "#efb8c8"),
            ("on_tertiary", "#492532"), ("error", "#f2b8b5"), ("on_error", "#601410"),
            ("outline", "#938f99"),
        ] {
            colors.insert(key.into(), json!({ "dark": value, "light": "#ffffff" }));
        }
        let text = json!({ "colors": colors }).to_string();
        let (roles, dark) = roles_from_matugen(&text, true).unwrap();
        assert!(dark);
        assert_eq!(hex(roles[Role::Primary as usize]), "#d0bcff");
        // Missing roles come from a scheme seeded with the primary.
        assert_ne!(hex(roles[Role::SurfaceContainerHigh as usize]), "#000000");
        assert!(roles_from_matugen("{\"wallpaper\": \"x.png\"}", true).is_none());
    }

    #[test]
    fn builtins_resolve_in_both_modes() {
        for dark in [false, true] {
            for id in [ACCENT_SOURCE].into_iter().chain(FAMILIES.iter().map(|f| f.id)) {
                let (resolved, error) = resolve(id, dark, None);
                assert!(error.is_none(), "{id}");
                assert_eq!(resolved.scheme.source, id);
                let s = &resolved.scheme;
                // Text reads on every ground it sits on.
                for (ink, ground) in [
                    (Role::OnSurface, Role::SurfaceContainerLow),
                    (Role::OnPrimary, Role::Primary),
                    (Role::OnPrimaryContainer, Role::PrimaryContainer),
                    (Role::OnSecondaryContainer, Role::SecondaryContainer),
                ] {
                    let ratio = contrast(s.role(ink), s.role(ground));
                    assert!(ratio >= 4.4, "{id} dark={dark}: {ratio:.2} for role {}", ink as usize);
                }
            }
        }
    }

    #[test]
    fn data_colours_follow_the_mode() {
        let dark = resolve(ACCENT_SOURCE, true, None).0.scheme.to_json();
        let light = resolve(ACCENT_SOURCE, false, None).0.scheme.to_json();
        assert_eq!(dark["data"].as_object().unwrap().len(), DATA_HUES.len());
        assert_ne!(dark["data"]["cpu"], light["data"]["cpu"]);
        assert_eq!(dark["dark"], true);
    }

    #[test]
    fn user_themes_from_seed_or_palette() {
        let dir = temp_dir("user-themes");
        fs::write(dir.join("seeded.toml"), "name = \"Seeded\"\nseed = \"#ff3d8a\"\nvariant = \"vibrant\"\n")
            .unwrap();
        let palette = "lowest = \"#000\"\nwindow = \"#111\"\npage = \"#222\"\nraised = \"#333\"\n\
                       raised_more = \"#444\"\nline = \"#555\"\ntext = \"#eee\"\nsubtext = \"#bbb\"\n\
                       muted = \"#888\"\nprimary = \"#f0a\"\nsecondary = \"#0af\"\ntertiary = \"#af0\"\n\
                       error = \"#f44\"\n";
        fs::write(dir.join("rice.toml"), format!("[dark]\n{palette}")).unwrap();
        fs::write(dir.join("old.toml"), "[colors]\nbackground = \"#000\"\n").unwrap();

        let (seeded, error) = resolve("seeded", true, Some(&dir));
        assert!(error.is_none());
        assert_eq!(seeded.watch, vec![dir.join("seeded.toml")]);
        let (rice, _) = resolve("rice", false, Some(&dir));
        // One palette serves both modes.
        assert_eq!(role(&rice.scheme, "primary"), "#ff00aa");

        let (fallback, error) = resolve("old", true, Some(&dir));
        assert!(format!("{:#}", error.unwrap()).contains("old theme format"));
        assert_eq!(fallback.scheme.source, ACCENT_SOURCE);

        let list = list(true, Some(&dir));
        let custom: Vec<_> = list.iter().filter(|t| t["kind"] == "custom").collect();
        assert_eq!(custom.len(), 3);
        assert_eq!(custom[2]["name"], "Seeded");
        assert!(custom[0]["error"].is_string());
    }

    #[test]
    fn unknown_theme_falls_back_to_the_accent() {
        let (resolved, error) = resolve("aurora", true, None);
        assert!(error.is_some());
        assert_eq!(resolved.scheme.source, ACCENT_SOURCE);
    }
}
