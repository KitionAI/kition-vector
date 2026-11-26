//! The canvas pointers (plan 2026-11-25, stage 2): the engine's [`Cursor`] as a CSS cursor. The
//! pointers a CSS name cannot say (the Pen's seven, the hollow arrow, rotation bowed toward a
//! corner, the Shape Builder, Blend, Width, Slice, the gradient stops, the corner widget, the type
//! widget and path brackets) are drawn here, as SVG made in code: original work for Kition Vector
//! (`docs/attribution.md`), each with its hotspot and a CSS name to fall back on.

use kition_vector_tools::Cursor;

/// The arrow every selection pointer starts from: tip at (3, 2).
const ARROW: &str = "M3 2 L3 23 L8.5 17.8 L12.2 26.4 L15.6 25 L11.9 16.5 L19 16.5 Z";
/// The Pen's nib, tip at (3, 3).
const NIB: &str = "M3 3 L14 7.5 L20.5 14 L14 20.5 L7.5 14 Z";
/// A crosshair centred on (11, 11).
const CROSS: &str = "M11 2 V20 M2 11 H20";

/// A small mark drawn at the pointer's lower right, saying what a click does.
#[derive(Clone, Copy)]
enum Badge {
    None,
    Plus,
    Minus,
    Ring,
    Slash,
    Caret,
    Join,
    Square,
    FilledSquare,
    Target,
    Diamond,
    Bracket,
    Type,
    Corner,
    Blade,
    Widths,
}

/// The SVG of `badge`, black on a white halo, in the 32 × 32 frame.
fn badge(badge: Badge) -> String {
    let (shape, fill) = match badge {
        Badge::None => return String::new(),
        Badge::Plus => ("M21 26 H29 M25 22 V30", false),
        Badge::Minus => ("M21 26 H29", false),
        Badge::Ring => ("M25 22.5 A3.5 3.5 0 1 1 24.99 22.5 Z", false),
        Badge::Slash => ("M22 30 L28 22", false),
        Badge::Caret => ("M21.5 29 L25 22.5 L28.5 29", false),
        Badge::Join => ("M21 26 H29 M22 23.5 V28.5 M28 23.5 V28.5", false),
        Badge::Square => ("M21.5 22.5 H28.5 V29.5 H21.5 Z", false),
        Badge::FilledSquare => ("M21.5 22.5 H28.5 V29.5 H21.5 Z", true),
        Badge::Target => ("M25 22 A4 4 0 1 1 24.99 22 Z M25 25.2 A0.8 0.8 0 1 1 24.99 25.2 Z", false),
        Badge::Diamond => ("M25 21.5 L29 25.5 L25 29.5 L21 25.5 Z", true),
        Badge::Bracket => ("M23 21.5 H27 V30 H23", false),
        Badge::Type => ("M21 22.5 H29 M25 22.5 V30", false),
        Badge::Corner => ("M21.5 30 V27 A5 5 0 0 1 26.5 22 H29.5", false),
        Badge::Blade => ("M21 30 L29 22 L29 25 L24 30 Z", true),
        Badge::Widths => ("M20.5 26 L23.5 23 V29 Z M29.5 26 L26.5 23 V29 Z", true),
    };
    let paint = if fill { "fill='black'" } else { "fill='none'" };
    format!(
        "<path d='{shape}' fill='none' stroke='white' stroke-width='4' stroke-linejoin='round' stroke-linecap='round'/><path d='{shape}' {paint} stroke='black' stroke-width='1.6' stroke-linejoin='round' stroke-linecap='round'/>"
    )
}

/// A 32 × 32 SVG cursor of `body` (and `mark`) with its hotspot at (`x`, `y`), else `fallback`.
fn svg(body: &str, mark: Badge, x: u8, y: u8, fallback: &str) -> String {
    let image = format!("<svg xmlns='http://www.w3.org/2000/svg' width='32' height='32' viewBox='0 0 32 32'>{body}{}</svg>", badge(mark));
    format!("url(\"data:image/svg+xml,{}\") {x} {y}, {fallback}", encode(&image))
}

/// The characters a data URL in CSS must not carry as they are.
fn encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + text.len() / 4);
    for c in text.chars() {
        match c {
            '<' => out.push_str("%3C"),
            '>' => out.push_str("%3E"),
            '#' => out.push_str("%23"),
            '"' => out.push_str("%22"),
            '%' => out.push_str("%25"),
            ' ' => out.push_str("%20"),
            _ => out.push(c),
        }
    }
    out
}

/// The selection arrow: black with a white edge, or hollow (Direct Selection).
fn arrow(hollow: bool, mark: Badge, fallback: &str) -> String {
    let (fill, edge) = if hollow { ("white", "black") } else { ("black", "white") };
    svg(&format!("<path d='{ARROW}' fill='{fill}' stroke='{edge}' stroke-width='1.5' stroke-linejoin='round'/>"), mark, 3, 2, fallback)
}

/// The Pen's nib with what a click does.
fn pen(mark: Badge) -> String {
    let body = format!(
        "<path d='{NIB}' fill='black' stroke='white' stroke-width='1.5' stroke-linejoin='round'/><path d='M3 3 L11 11' stroke='white' stroke-width='1.3'/><circle cx='11.5' cy='11.5' r='1.6' fill='white'/>"
    );
    svg(&body, mark, 3, 3, "crosshair")
}

/// A crosshair with what a click does.
fn cross(mark: Badge) -> String {
    let body = format!("<path d='{CROSS}' stroke='white' stroke-width='3.5'/><path d='{CROSS}' stroke='black' stroke-width='1.4'/>");
    svg(&body, mark, 11, 11, "crosshair")
}

/// The rotate pointer: an arc with an arrowhead at each end, bowed away from the box. `octant`
/// turns it as [`Cursor::RotateToward`] says (0 above the box, then clockwise in 45° steps).
fn rotate(octant: Option<u8>) -> String {
    let turn = u16::from(octant.unwrap_or(0) % 8) * 45;
    let arc = "M7 17 A10 10 0 0 1 25 17 M4.5 13.5 L7 18 L11 15 M27.5 13.5 L25 18 L21 15";
    let body = format!(
        "<g transform='rotate({turn} 16 16)'><path d='{arc}' fill='none' stroke='white' stroke-width='4.5' stroke-linecap='round' stroke-linejoin='round'/><path d='{arc}' fill='none' stroke='black' stroke-width='1.7' stroke-linecap='round' stroke-linejoin='round'/></g>"
    );
    svg(&body, Badge::None, 16, 16, "alias")
}

/// The eyedropper, tip at (3, 29).
fn eyedropper() -> String {
    let body = "<path d='M3 29 L5 23 L17 11 L21 15 L9 27 Z' fill='white' stroke='black' stroke-width='1.5' stroke-linejoin='round'/><path d='M15 9 L19 5 A2.8 2.8 0 0 1 27 13 L23 17 Z' fill='black' stroke='white' stroke-width='1'/>";
    svg(body, Badge::None, 3, 29, "copy")
}

/// The engine's cursor as a CSS cursor: a CSS name where one says it, else one of the pointers above.
pub fn css(cursor: Cursor) -> String {
    let name = match cursor {
        Cursor::Arrow => "default",
        Cursor::Move => "move",
        Cursor::Crosshair => "crosshair",
        Cursor::ResizeH => "ew-resize",
        Cursor::ResizeV => "ns-resize",
        Cursor::ResizeNwSe => "nwse-resize",
        Cursor::ResizeNeSw => "nesw-resize",
        Cursor::Text => "text",
        Cursor::Hand => "grab",
        Cursor::HandGrab => "grabbing",
        Cursor::ZoomIn => "zoom-in",
        Cursor::ZoomOut => "zoom-out",
        Cursor::NotAllowed => "not-allowed",
        Cursor::ArrowHollow => return arrow(true, Badge::None, "default"),
        Cursor::Rotate => return rotate(None),
        Cursor::RotateToward(octant) => return rotate(Some(octant)),
        Cursor::CornerRadius => return arrow(false, Badge::Corner, "default"),
        Cursor::Pen => return pen(Badge::None),
        Cursor::PenAdd => return pen(Badge::Plus),
        Cursor::PenDelete => return pen(Badge::Minus),
        Cursor::PenClose => return pen(Badge::Ring),
        Cursor::PenContinue => return pen(Badge::Slash),
        Cursor::PenJoin => return pen(Badge::Join),
        Cursor::PenConvert => return pen(Badge::Caret),
        Cursor::Eyedropper => return eyedropper(),
        Cursor::AddStop => return arrow(false, Badge::Plus, "copy"),
        Cursor::RemoveStop => return arrow(false, Badge::Minus, "default"),
        Cursor::Slice => return cross(Badge::Blade),
        Cursor::SliceSelect => return arrow(false, Badge::Blade, "default"),
        Cursor::Width => return cross(Badge::Widths),
        Cursor::WidthAdd => return cross(Badge::Plus),
        Cursor::WidthPoint => return cross(Badge::Diamond),
        Cursor::Blend => return cross(Badge::Square),
        Cursor::BlendObject => return cross(Badge::FilledSquare),
        Cursor::BlendAnchor => return cross(Badge::Target),
        Cursor::PathBracket => return arrow(false, Badge::Bracket, "default"),
        Cursor::TypeWidget => return arrow(false, Badge::Type, "default"),
        Cursor::ShapeBuilder => return cross(Badge::Plus),
        Cursor::ShapeBuilderErase => return cross(Badge::Minus),
    };
    name.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every pointer the engine has is a CSS cursor: a name, or an SVG drawn here with its hotspot
    /// and a name to fall back on; the Pen's states differ from one another.
    #[test]
    fn every_engine_cursor_is_a_css_cursor() {
        let mut all = vec![
            Cursor::Arrow,
            Cursor::ArrowHollow,
            Cursor::Move,
            Cursor::Crosshair,
            Cursor::Rotate,
            Cursor::CornerRadius,
            Cursor::Pen,
            Cursor::PenAdd,
            Cursor::PenDelete,
            Cursor::PenClose,
            Cursor::PenContinue,
            Cursor::PenJoin,
            Cursor::PenConvert,
            Cursor::Text,
            Cursor::Eyedropper,
            Cursor::AddStop,
            Cursor::RemoveStop,
            Cursor::Slice,
            Cursor::SliceSelect,
            Cursor::Width,
            Cursor::WidthAdd,
            Cursor::WidthPoint,
            Cursor::Blend,
            Cursor::BlendObject,
            Cursor::BlendAnchor,
            Cursor::PathBracket,
            Cursor::TypeWidget,
            Cursor::ShapeBuilder,
            Cursor::ShapeBuilderErase,
        ];
        all.extend((0..8).map(Cursor::RotateToward));
        for cursor in all {
            let css = css(cursor);
            if css.starts_with("url(") {
                assert!(css.contains("data:image/svg+xml,%3Csvg"), "{cursor:?}: {css}");
                let url = css.split("\")").next().unwrap_or_default();
                assert!(!url.contains('<') && !url.contains('#') && !url.contains(' '), "{cursor:?} is encoded: {url}");
                let fallback = css.rsplit(", ").next().unwrap_or_default();
                assert!(!fallback.is_empty() && !fallback.contains('('), "{cursor:?} falls back to a name: {fallback}");
            } else {
                assert!(css.chars().all(|c| c.is_ascii_lowercase() || c == '-'), "{cursor:?}: {css}");
            }
        }
        let pens: std::collections::HashSet<String> =
            [Cursor::Pen, Cursor::PenAdd, Cursor::PenDelete, Cursor::PenClose, Cursor::PenContinue, Cursor::PenJoin, Cursor::PenConvert]
                .into_iter()
                .map(css)
                .collect();
        assert_eq!(pens.len(), 7, "the Pen's seven pointers differ");
        assert_ne!(css(Cursor::RotateToward(1)), css(Cursor::RotateToward(5)), "the rotate pointer turns with the corner");
    }
}
