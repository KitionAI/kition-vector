//! SVG basic shapes → live shapes: `<rect>` (rounded too, with equal corner radii) a live
//! rectangle, `<circle>` and `<ellipse>` a live ellipse, `<line>` a live line. usvg turns them into
//! paths; [`super::prepass`] records which elements they were ([`kind`]), and the shape is read back
//! from the path usvg made, so lengths in any unit or percentage come out as usvg resolved them.
//! Anything that doesn't read back exactly stays a path. `<polygon>` and `<polyline>` stay paths:
//! the model's live polygon is a regular one.

use kition_vector_doc::LiveShape;
use kition_vector_geom::{Affine, BezPath, Point, Rect};
use usvg::tiny_skia_path::{Path, PathSegment as S};

/// The basic shape an element is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Basic {
    Rect,
    Ellipse,
    Line,
}

/// The basic shape element `tag` is, if it is one this module reads.
pub(super) fn kind(tag: &str) -> Option<Basic> {
    match tag {
        "rect" => Some(Basic::Rect),
        "circle" | "ellipse" => Some(Basic::Ellipse),
        "line" => Some(Basic::Line),
        _ => None,
    }
}

/// The live shape of the path usvg made of a `kind` element (`data`, in the element's user space,
/// which `acc` maps into the document); `None` when the path isn't that shape as usvg writes it.
pub(super) fn live(kind: Basic, data: &Path, acc: Affine) -> Option<LiveShape> {
    let segs: Vec<S> = data.segments().collect();
    let pt = |p: usvg::tiny_skia_path::Point| Point::new(f64::from(p.x), f64::from(p.y));
    let b = data.bounds();
    let r = Rect::new(f64::from(b.left()), f64::from(b.top()), f64::from(b.right()), f64::from(b.bottom()));
    let tol = 1e-4 * r.width().max(r.height()).max(1.0);
    let near = |a: f64, b: f64| (a - b).abs() <= tol;
    let shape = match kind {
        Basic::Line => {
            let [S::MoveTo(a), S::LineTo(b)] = segs.as_slice() else { return None };
            return Some(LiveShape::Line { a: acc * pt(*a), b: acc * pt(*b) });
        }
        Basic::Rect => {
            if r.width() <= 0.0 || r.height() <= 0.0 {
                return None;
            }
            let Some(S::MoveTo(first)) = segs.first() else { return None };
            let first = pt(*first);
            let radius = if segs.iter().all(|s| matches!(s, S::MoveTo(_) | S::LineTo(_) | S::Close)) {
                // A sharp rectangle: its four corners from the top left, clockwise.
                let corners = [Point::new(r.x0, r.y0), Point::new(r.x1, r.y0), Point::new(r.x1, r.y1), Point::new(r.x0, r.y1)];
                let points: Vec<Point> = segs
                    .iter()
                    .filter_map(|s| match s {
                        S::MoveTo(p) | S::LineTo(p) => Some(pt(*p)),
                        _ => None,
                    })
                    .collect();
                if points.len() != 4 || points.iter().zip(corners).any(|(p, c)| !near(p.x, c.x) || !near(p.y, c.y)) {
                    return None;
                }
                0.0
            } else {
                // Rounded: it starts `rx` along the top edge; its right edge's line ends `ry` above
                // the bottom.
                let rx = first.x - r.x0;
                let ry = segs.iter().find_map(|s| match s {
                    S::LineTo(p) if near(f64::from(p.x), r.x1) && f64::from(p.y) > r.y0 + tol => Some(r.y1 - f64::from(p.y)),
                    _ => None,
                })?;
                if !near(first.y, r.y0) || rx <= 0.0 || !near(rx, ry) {
                    return None;
                }
                (rx + ry) / 2.0
            };
            let mut s = LiveShape::Rectangle {
                w: r.width(),
                h: r.height(),
                radii: [radius; 4],
                kinds: Default::default(),
                xf: acc * Affine::translate(r.origin().to_vec2()),
            };
            // A scale moves into the size, unless it would make elliptical corners round.
            if radius == 0.0 || uniform(acc) {
                s.fold_scale();
            }
            s
        }
        Basic::Ellipse => {
            if r.width() <= 0.0 || r.height() <= 0.0 {
                return None;
            }
            let Some(S::MoveTo(first)) = segs.first() else { return None };
            let (c, rx, ry) = (r.center(), r.width() / 2.0, r.height() / 2.0);
            let first = pt(*first);
            let on = ((first.x - c.x) / rx).powi(2) + ((first.y - c.y) / ry).powi(2);
            if (on - 1.0).abs() > 1e-3 || segs.iter().any(|s| matches!(s, S::LineTo(_))) {
                return None;
            }
            LiveShape::Ellipse { w: r.width(), h: r.height(), pie: (0.0, 360.0), xf: acc * Affine::translate(r.origin().to_vec2()) }
        }
    };
    // The shape must cover what usvg drew.
    let drawn = kurbo::Shape::bounding_box(&super::bezpath(data, acc));
    let made: BezPath = shape.to_path().to_bezpath();
    let made = kurbo::Shape::bounding_box(&made);
    let tol = 1e-3 * drawn.width().max(drawn.height()).max(1.0);
    let same = [(made.x0, drawn.x0), (made.y0, drawn.y0), (made.x1, drawn.x1), (made.y1, drawn.y1)].iter().all(|(a, b)| (a - b).abs() <= tol);
    same.then_some(shape)
}

/// Does `m` scale evenly, without shearing?
fn uniform(m: Affine) -> bool {
    let [a, b, c, d, _, _] = m.as_coeffs();
    let (sx, sy) = (a.hypot(b), c.hypot(d));
    sx > 1e-12 && (sx - sy).abs() <= 1e-9 * sx.max(sy) && (a * c + b * d).abs() <= 1e-9 * sx * sy
}
