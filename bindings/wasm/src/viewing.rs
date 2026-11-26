//! The editor's view of the drawing (plan 2026-11-25, stage 6): the View menu's toggles (Outline,
//! Pixel Preview, Smart Guides, the snaps, the bounding box, the selection's edges, corner widgets,
//! guides, the grid, the rulers), where the canvas window is, and a guide dragged out of a ruler.
//!
//! They are the editing session's, not the drawing's: they change no revision, no undo history and
//! nothing saved, and an agent reads them in `inspect()` (`view`). The page sets them with a key
//! that is a request ([`REQUEST`], as the Type tool's `Type:`), so the engine's command list stays
//! as it is.

use kition_vector_engine::{DocState, Session, ViewInfo};
use kition_vector_geom::{Point, Vec2};
use kition_vector_tools::{Mods, PointerEvent, PointerKind, ScreenFrame};
use serde_json::{Map, Value, json};

/// A key the page gives that is a request about the view, not a key: `View:` and a JSON object,
/// `{"set": {toggle: bool, …}}`, `{"screen": {x, y, width, height, zoom}}` or
/// `{"ruler": {vertical, kind: "drag"|"up"|"cancel", x, y, onCanvas, shift?}}`.
pub const REQUEST: &str = "View:";

/// The View menu's toggles the canvas and the tools follow.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Toggles {
    /// View › Outline: paths as thin lines on white, no paint; the tools pick by path.
    pub outline: bool,
    /// View › Pixel Preview: the art drawn as the pixels it rasterizes to at 72 ppi.
    pub pixel_preview: bool,
    pub smart_guides: bool,
    pub snap_to_grid: bool,
    pub snap_to_point: bool,
    pub snap_to_pixel: bool,
    /// View › Show Bounding Box: the Selection tool's box and its handles.
    pub bounding_box: bool,
    /// View › Show Edges: the selection's outlines, anchors and handles.
    pub edges: bool,
    pub corner_widgets: bool,
    /// View › Guides › Show Guides: the ruler guides show and can be picked.
    pub guides: bool,
    /// View › Show Grid (the page draws it).
    pub grid: bool,
    /// View › Rulers › Show Rulers (the page draws them).
    pub rulers: bool,
}

impl Default for Toggles {
    fn default() -> Self {
        let view = ViewInfo::default();
        Self {
            outline: view.outline,
            pixel_preview: false,
            smart_guides: view.smart_guides,
            snap_to_grid: view.snap_to_grid,
            snap_to_point: view.snap_to_point,
            snap_to_pixel: view.snap_to_pixel,
            bounding_box: view.show_bbox,
            edges: true,
            corner_widgets: view.corner_widgets,
            guides: view.guides,
            grid: false,
            rulers: false,
        }
    }
}

/// The toggles by the names the page and `inspect()` use.
const NAMES: &[&str] = &[
    "outline",
    "pixelPreview",
    "smartGuides",
    "snapToGrid",
    "snapToPoint",
    "snapToPixel",
    "boundingBox",
    "edges",
    "cornerWidget",
    "guides",
    "grid",
    "rulers",
];

impl Toggles {
    fn slot(&mut self, name: &str) -> Option<&mut bool> {
        Some(match name {
            "outline" => &mut self.outline,
            "pixelPreview" => &mut self.pixel_preview,
            "smartGuides" => &mut self.smart_guides,
            "snapToGrid" => &mut self.snap_to_grid,
            "snapToPoint" => &mut self.snap_to_point,
            "snapToPixel" => &mut self.snap_to_pixel,
            "boundingBox" => &mut self.bounding_box,
            "edges" => &mut self.edges,
            "cornerWidget" => &mut self.corner_widgets,
            "guides" => &mut self.guides,
            "grid" => &mut self.grid,
            "rulers" => &mut self.rulers,
            _ => return None,
        })
    }

    /// Sets the toggles `set` names; refuses a name it does not know or a value that is not a
    /// boolean, setting none of them then.
    pub fn set(&mut self, set: &Map<String, Value>) -> Result<(), String> {
        let mut next = *self;
        for (name, value) in set {
            let on = value.as_bool().ok_or_else(|| format!("invalid-params: view toggle `{name}` takes true or false, not {value}"))?;
            *next.slot(name).ok_or_else(|| format!("invalid-params: no view toggle `{name}` (there are {})", NAMES.join(", ")))? = on;
        }
        *self = next;
        Ok(())
    }

    fn json(mut self) -> Map<String, Value> {
        NAMES.iter().filter_map(|name| Some(((*name).to_owned(), Value::Bool(*self.slot(name)?)))).collect()
    }
}

/// Where the canvas window is: the document pixel at its top-left, its size in screen pixels and
/// the zoom (screen pixels per point).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Screen {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub zoom: f64,
}

impl Screen {
    fn of(value: &Value) -> Result<Self, String> {
        let number = |key: &str| {
            value
                .get(key)
                .and_then(Value::as_f64)
                .filter(|v| v.is_finite())
                .ok_or_else(|| format!("invalid-params: the screen's `{key}` is not a number"))
        };
        let screen = Self { x: number("x")?, y: number("y")?, width: number("width")?, height: number("height")?, zoom: number("zoom")? };
        if screen.zoom <= 0.0 || screen.width < 0.0 || screen.height < 0.0 {
            return Err("invalid-params: the screen needs a positive zoom and a size".to_owned());
        }
        Ok(screen)
    }

    /// The window as the tools see it, the drawing's point at document pixel (0, 0) being `origin`.
    pub fn frame(&self, origin: Point) -> ScreenFrame {
        ScreenFrame {
            origin: Point::new(origin.x + self.x, origin.y + self.y),
            right: Vec2::new(1.0 / self.zoom, 0.0),
            down: Vec2::new(0.0, 1.0 / self.zoom),
            size: (self.width, self.height),
        }
    }
}

/// What the tools see of the view at `zoom`.
pub fn info(toggles: &Toggles, screen: Option<&Screen>, origin: Point, zoom: f64) -> ViewInfo {
    ViewInfo {
        zoom: if zoom.is_finite() && zoom > 0.0 { zoom } else { 1.0 },
        outline: toggles.outline,
        smart_guides: toggles.smart_guides,
        guides: toggles.guides,
        snap_to_grid: toggles.snap_to_grid,
        show_bbox: toggles.bounding_box,
        snap_to_pixel: toggles.snap_to_pixel,
        snap_to_point: toggles.snap_to_point,
        corner_widgets: toggles.corner_widgets,
        screen: screen.map(|screen| screen.frame(origin)),
    }
}

/// A request of the page ([`REQUEST`]), parsed.
pub enum Request {
    Set(Map<String, Value>),
    Screen(Screen),
    /// A guide out of a ruler: vertical (out of the left ruler) or not, the pointer event in
    /// document pixels, and whether it is over the canvas.
    Ruler {
        vertical: bool,
        kind: PointerKind,
        x: f64,
        y: f64,
        on_canvas: bool,
        shift: bool,
    },
}

pub fn request(body: &str) -> Result<Request, String> {
    let value: Value = serde_json::from_str(body).map_err(|error| format!("invalid-params: a view request is a JSON object: {error}"))?;
    if let Some(set) = value.get("set") {
        return set.as_object().cloned().map(Request::Set).ok_or_else(|| "invalid-params: `set` takes an object of toggles".to_owned());
    }
    if let Some(screen) = value.get("screen") {
        return Screen::of(screen).map(Request::Screen);
    }
    if let Some(ruler) = value.get("ruler") {
        let number = |key: &str| {
            ruler
                .get(key)
                .and_then(Value::as_f64)
                .filter(|v| v.is_finite())
                .ok_or_else(|| format!("invalid-params: the ruler's `{key}` is not a number"))
        };
        let flag = |key: &str| ruler.get(key).and_then(Value::as_bool).unwrap_or(false);
        let kind = match ruler.get("kind").and_then(Value::as_str) {
            Some("drag") => PointerKind::Drag,
            Some("up") => PointerKind::Up,
            other => return Err(format!("invalid-params: a ruler drag's `kind` is \"drag\" or \"up\", not {other:?}")),
        };
        return Ok(Request::Ruler {
            vertical: flag("vertical"),
            kind,
            x: number("x")?,
            y: number("y")?,
            on_canvas: flag("onCanvas"),
            shift: flag("shift"),
        });
    }
    Err("invalid-params: a view request has `set`, `screen` or `ruler`".to_owned())
}

/// The pointer event of a ruler drag at document point `at`.
pub fn ruler_event(kind: PointerKind, at: Point, shift: bool) -> PointerEvent {
    PointerEvent { kind, pos: at, mods: Mods { shift, ..Mods::default() }, pressure: 1.0 }
}

/// Whether the transparency grid shows behind the artboards of `state` (View › Show Transparency
/// Grid with the artboards' background transparent, Document Setup).
pub fn transparency_grid(state: &DocState) -> bool {
    state.transparency_grid && state.doc.setup.background == kition_vector_doc::setup::Background::Transparent
}

/// `view` in `inspect()`: the toggles, the menu's other view states (Lock Guides, the slices', the
/// transparency grid, Proof Colors, Overprint Preview), and how the page draws the grid and the
/// guides (Preferences › Guides & Grid, the document's grid).
pub fn report(toggles: &Toggles, session: &Session, state: &DocState) -> Value {
    let mut out = toggles.json();
    let (slices_hidden, slices_locked) = session.slice_view();
    let proof = kition_vector_render::proof::view();
    let prefs = &session.prefs;
    out.insert("guidesLocked".into(), Value::Bool(session.guides_locked()));
    out.insert("slicesHidden".into(), Value::Bool(slices_hidden));
    out.insert("slicesLocked".into(), Value::Bool(slices_locked));
    out.insert("transparencyGrid".into(), Value::Bool(state.transparency_grid));
    out.insert("proofColors".into(), Value::Bool(proof.proof_colors));
    out.insert("overprintPreview".into(), Value::Bool(proof.overprint));
    out.insert(
        "gridLook".into(),
        json!({
            "spacing": state.doc.grid.spacing,
            "subdivisions": state.doc.grid.subdivisions,
            "color": prefs.grid_color,
            "style": prefs.grid_style,
            "pixelGrid": prefs.show_pixel_grid,
        }),
    );
    out.insert("guideLook".into(), json!({ "color": prefs.guide_color, "style": prefs.guide_style }));
    Value::Object(out)
}

/// An area the canvas asks for: document pixel (`x`, `y`) at its top-left, `scale` screen pixels
/// per point, `width × height` pixels.
#[derive(Clone, Copy, Debug)]
pub struct Tile {
    pub x: f64,
    pub y: f64,
    pub scale: f64,
    pub width: u32,
    pub height: u32,
}

/// The transparency grid under the premultiplied `pixels` of `tile`, on the artboards of `doc`
/// (the drawing's point at document pixel (0, 0) being `origin`): Document Setup's two colours in
/// cells of its size in screen points (`dpr` device pixels each), fixed to the screen so the
/// canvas's tiles meet.
pub fn checker_under(pixels: &mut [u8], tile: Tile, doc: &kition_vector_doc::Document, origin: Point, dpr: f64) {
    let setup = &doc.setup;
    let [light, dark] = setup.grid_colors.map(|color| color.to_rgba8(1.0));
    let dpr = if dpr.is_finite() && dpr > 0.0 { dpr.clamp(0.5, 8.0) } else { 1.0 };
    let cell = (f64::from(setup.grid_size.cell()) * dpr).round().max(1.0) as i64;
    let (left, top) = ((tile.x * tile.scale).floor() as i64, (tile.y * tile.scale).floor() as i64);
    let to_px = |v: f64, from: f64, size: u32| ((v - from) * tile.scale).round().clamp(0.0, f64::from(size)) as usize;
    let width = tile.width as usize;
    for board in &doc.artboards {
        let r = board.rect;
        let (x0, x1) = (to_px(r.x0 - origin.x, tile.x, tile.width), to_px(r.x1 - origin.x, tile.x, tile.width));
        let (y0, y1) = (to_px(r.y0 - origin.y, tile.y, tile.height), to_px(r.y1 - origin.y, tile.y, tile.height));
        for row in y0..y1 {
            let band = (top + row as i64).div_euclid(cell);
            for column in x0..x1 {
                let back = if (band + (left + column as i64).div_euclid(cell)) % 2 == 0 { light } else { dark };
                let at = (row * width + column) * 4;
                let Some([r, g, b, a]) = pixels.get_mut(at..at + 4).and_then(|px| <&mut [u8; 4]>::try_from(px).ok()) else { continue };
                let rest = 255 - u32::from(*a);
                for (channel, under) in [r, g, b].into_iter().zip(back) {
                    *channel = (u32::from(*channel) + (u32::from(under) * rest + 127) / 255).min(255) as u8;
                }
                *a = 255;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggles_are_set_by_name_all_or_none() {
        let mut toggles = Toggles::default();
        assert!(toggles.smart_guides && toggles.bounding_box && toggles.edges && !toggles.outline && !toggles.grid && !toggles.rulers);
        let set = |text: &str| serde_json::from_str::<Map<String, Value>>(text).unwrap_or_default();
        toggles.set(&set(r#"{"outline": true, "rulers": true}"#)).unwrap_or_else(|e| panic!("{e}"));
        assert!(toggles.outline && toggles.rulers);
        assert!(toggles.set(&set(r#"{"grid": true, "nonsense": true}"#)).is_err());
        assert!(toggles.set(&set(r#"{"grid": 1}"#)).is_err());
        assert!(!toggles.grid, "a refused request sets nothing");
    }

    #[test]
    fn requests_parse() {
        assert!(matches!(request(r#"{"set": {"outline": true}}"#), Ok(Request::Set(_))));
        assert!(matches!(request(r#"{"screen": {"x": 1, "y": 2, "width": 300, "height": 200, "zoom": 2}}"#), Ok(Request::Screen(_))));
        assert!(matches!(
            request(r#"{"ruler": {"vertical": true, "kind": "drag", "x": 10, "y": 20, "onCanvas": true}}"#),
            Ok(Request::Ruler { vertical: true, kind: PointerKind::Drag, on_canvas: true, .. })
        ));
        assert!(request(r#"{"ruler": {"kind": "down", "x": 1, "y": 1}}"#).is_err());
        assert!(request("[]").is_err());
        assert!(request(r#"{"screen": {"x": 0, "y": 0, "width": 1, "height": 1, "zoom": 0}}"#).is_err());
    }
}
