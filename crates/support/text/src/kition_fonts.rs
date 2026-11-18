//! Fonts from the optional kition-fonts build input (<https://github.com/KitionAI/kition-fonts>).
//!
//! Built with `KITION_FONTS_DIR=<kition-fonts checkout>`, [`KITION_FONTS`] holds every font in its
//! manifest (Japanese UI and document fonts, and Arabic fonts); built without it, it is empty and Japanese
//! text falls back to the installed system fonts. Release builds always set it. See `build.rs`.

/// A font from the optional kition-fonts build input (empty unless built with `KITION_FONTS_DIR`).
pub struct KitionFont {
    pub family: &'static str,
    pub style: &'static str,
    /// ISO 15924 scripts the font is for, e.g. `"Jpan"`.
    pub scripts: &'static [&'static str],
    pub bytes: &'static [u8],
}

include!(concat!(env!("OUT_DIR"), "/kition_fonts.rs"));

impl KitionFont {
    /// Is the font for Japanese text?
    pub fn is_japanese(&self) -> bool {
        self.scripts.contains(&"Jpan")
    }
    /// Is the font for Arabic text?
    pub fn is_arabic(&self) -> bool {
        self.scripts.contains(&"Arab")
    }
    /// A serif (Mincho) family, for document text.
    pub fn is_mincho(&self) -> bool {
        self.family.contains("Mincho")
    }
    /// A sans (Gothic) family, for UI text.
    pub fn is_gothic(&self) -> bool {
        self.family.contains("Gothic")
    }
    /// A bold face.
    pub fn is_bold(&self) -> bool {
        crate::style_weight(self.style) >= 600.0
    }
}

/// The kition-fonts faces for Japanese text, best for document text first: Mincho (serif) families,
/// then the others, regular weights before bold. Empty when built without kition-fonts.
pub fn japanese_document_fonts() -> Vec<&'static KitionFont> {
    let mut v: Vec<&'static KitionFont> = KITION_FONTS.iter().filter(|f| f.is_japanese()).collect();
    v.sort_by_key(|f| (!f.is_mincho(), f.is_bold()));
    v
}

/// The kition-fonts faces the document font database loads: the Japanese ones in
/// [`japanese_document_fonts`] order, then the Arabic ones in manifest order (Noto Sans Arabic
/// first: the fallback for Arabic text; the other families are picked by name). Empty when built
/// without kition-fonts.
pub fn document_fonts() -> Vec<&'static KitionFont> {
    let mut v = japanese_document_fonts();
    v.extend(KITION_FONTS.iter().filter(|f| f.is_arabic() && !f.is_japanese()));
    v
}

/// The kition-fonts faces for Japanese text, best for UI text first: Gothic (sans) families, then
/// the others; `bold` puts bold faces before regular ones. Empty when built without kition-fonts.
pub fn japanese_ui_fonts(bold: bool) -> Vec<&'static KitionFont> {
    let mut v: Vec<&'static KitionFont> = KITION_FONTS.iter().filter(|f| f.is_japanese()).collect();
    v.sort_by_key(|f| (!f.is_gothic(), f.is_bold() != bold));
    v
}
