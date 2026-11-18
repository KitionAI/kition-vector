//! SVG basic shapes come in as live shapes and go out as the same elements: `<rect>` (rounded too)
//! a live rectangle, `<circle>` and `<ellipse>` a live ellipse, `<line>` a live line. What a live
//! shape can't be (unequal corner radii, a pie, a turned shape, an edited path) is written as a
//! path; `<polygon>` and `<polyline>` stay paths.
// Integration tests: unwrapping and panicking on failure is fine here, unlike in shipped code (AGENTS.md › Robustness).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kition_vector_doc::{Appearance, Document, LiveShape, Node, NodeKind};
use kition_vector_geom::{Affine, Point, shapes::CornerKind};
use kition_vector_svg::{ExportOptions, export, import};
use kition_vector_testkit::raster::{Image, assert_similar, render_artboard};

fn svg(body: &str) -> String {
    format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="120" viewBox="0 0 200 120">{body}</svg>"#)
}

fn resvg_render(svg: &str, w: u32, h: u32) -> Image {
    let tree = resvg::usvg::Tree::from_str(svg, &resvg::usvg::Options::default()).expect("parse");
    let mut pm = resvg::tiny_skia::Pixmap::new(w, h).unwrap();
    pm.fill(resvg::tiny_skia::Color::WHITE);
    resvg::render(&tree, resvg::tiny_skia::Transform::identity(), &mut pm.as_mut());
    Image { width: w, height: h, rgba: pm.data().to_vec() }
}

/// Every path's live shape (`None`: a plain path), in paint order, clipping paths included.
fn lives(d: &Document) -> Vec<Option<LiveShape>> {
    let mut v = Vec::new();
    d.walk(|n| {
        if let NodeKind::Path { live, .. } = &n.kind {
            v.push(live.clone());
        }
    });
    v
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

fn coeffs_close(a: Affine, b: Affine) -> bool {
    a.as_coeffs().iter().zip(b.as_coeffs()).all(|(x, y)| close(*x, y))
}

/// The same live shape, within rounding.
fn same_shape(a: &LiveShape, b: &LiveShape) -> bool {
    let near = |x: f64, y: f64| (x - y).abs() < 1e-3;
    let xf_near = |a: Affine, b: Affine| a.as_coeffs().iter().zip(b.as_coeffs()).all(|(x, y)| near(*x, y));
    match (a, b) {
        (LiveShape::Rectangle { w, h, radii, kinds, xf }, LiveShape::Rectangle { w: w2, h: h2, radii: r2, kinds: k2, xf: x2 }) => {
            near(*w, *w2) && near(*h, *h2) && radii.iter().zip(r2).all(|(x, y)| near(*x, *y)) && kinds == k2 && xf_near(*xf, *x2)
        }
        (LiveShape::Ellipse { w, h, pie, xf }, LiveShape::Ellipse { w: w2, h: h2, pie: p2, xf: x2 }) => {
            near(*w, *w2) && near(*h, *h2) && pie == p2 && xf_near(*xf, *x2)
        }
        (LiveShape::Line { a, b }, LiveShape::Line { a: a2, b: b2 }) => near(a.x, a2.x) && near(a.y, a2.y) && near(b.x, b2.x) && near(b.y, b2.y),
        _ => false,
    }
}

const SHAPES: &str = r##"<rect x="10" y="20" width="50" height="30" fill="#e8573f"/>
<rect x="70" y="10" width="40" height="60" rx="8" fill="#3366cc" stroke="#000" stroke-width="2"/>
<circle cx="150" cy="40" r="25" fill="#33aa55"/>
<ellipse cx="50" cy="90" rx="30" ry="15" fill="#aa33aa"/>
<line x1="100" y1="80" x2="190" y2="110" stroke="#000" stroke-width="3"/>"##;

#[test]
fn basic_shapes_come_in_as_live_shapes() {
    let d = import(&svg(SHAPES)).unwrap();
    let got = lives(&d);
    let want = [
        LiveShape::Rectangle { w: 50.0, h: 30.0, radii: [0.0; 4], kinds: Default::default(), xf: Affine::translate((10.0, 20.0)) },
        LiveShape::Rectangle { w: 40.0, h: 60.0, radii: [8.0; 4], kinds: Default::default(), xf: Affine::translate((70.0, 10.0)) },
        LiveShape::Ellipse { w: 50.0, h: 50.0, pie: (0.0, 360.0), xf: Affine::translate((125.0, 15.0)) },
        LiveShape::Ellipse { w: 60.0, h: 30.0, pie: (0.0, 360.0), xf: Affine::translate((20.0, 75.0)) },
        LiveShape::Line { a: Point::new(100.0, 80.0), b: Point::new(190.0, 110.0) },
    ];
    assert_eq!(got.len(), want.len(), "{got:?}");
    for (g, w) in got.iter().zip(&want) {
        let g = g.as_ref().unwrap_or_else(|| panic!("not live, want {w:?}"));
        assert!(same_shape(g, w), "{g:?} != {w:?}");
    }
    // The path is the shape's, and the drawing is the SVG's.
    d.walk(|n| {
        if let NodeKind::Path { path, live: Some(live), .. } = &n.kind {
            assert_eq!(path, &live.to_path());
        }
    });
    assert_similar(&render_artboard(&d), &resvg_render(&svg(SHAPES), 200, 120), 24.0, 0.004);
}

#[test]
fn live_shapes_go_out_as_the_same_elements_and_come_back() {
    let d = import(&svg(SHAPES)).unwrap();
    let out = export(&d, &ExportOptions::default());
    for element in ["<rect", "<circle", "<ellipse", "<line"] {
        assert!(out.contains(element), "{element}: {out}");
    }
    assert_eq!(out.matches("<rect").count(), 2, "{out}");
    assert!(out.contains("rx=\"8\""), "the rounded corners: {out}");
    assert!(!out.contains("<path"), "no shape written as a path: {out}");
    let back = import(&out).unwrap();
    let (a, b) = (lives(&d), lives(&back));
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(&b) {
        assert!(same_shape(x.as_ref().unwrap(), y.as_ref().unwrap()), "{x:?} != {y:?}\n{out}");
    }
    assert_similar(&render_artboard(&back), &render_artboard(&d), 24.0, 0.002);
}

#[test]
fn transformed_and_scaled_shapes() {
    // A scale on a sharp rectangle goes into its size; on a rounded one it stays in `xf` when it
    // is uneven (elliptical corners), and goes into the size and radii when it is even.
    let body = r##"<g transform="translate(5 5) scale(2 1)"><rect width="20" height="10" fill="#000"/><rect x="40" width="20" height="10" rx="4" fill="#000"/></g>
<g transform="scale(3)"><rect x="40" y="10" width="10" height="10" rx="2" fill="#000"/></g>
<rect x="10" y="60" width="40" height="20" transform="rotate(30 30 70)" fill="#000"/>"##;
    let d = import(&svg(body)).unwrap();
    let got: Vec<LiveShape> = lives(&d).into_iter().map(Option::unwrap).collect();
    let LiveShape::Rectangle { w, h, xf, .. } = &got[0] else { panic!("{got:?}") };
    assert!(close(*w, 40.0) && close(*h, 10.0) && coeffs_close(*xf, Affine::translate((5.0, 5.0))), "{:?}", got[0]);
    let LiveShape::Rectangle { w, radii, xf, .. } = &got[1] else { panic!("{got:?}") };
    assert!(
        close(*w, 20.0)
            && close(radii[0], 4.0)
            && coeffs_close(*xf, Affine::translate((5.0, 5.0)) * Affine::scale_non_uniform(2.0, 1.0) * Affine::translate((40.0, 0.0)))
    );
    let LiveShape::Rectangle { w, radii, xf, .. } = &got[2] else { panic!("{got:?}") };
    assert!(close(*w, 30.0) && close(radii[0], 6.0) && coeffs_close(*xf, Affine::translate((120.0, 30.0))), "{:?}", got[2]);
    assert!(matches!(got[3], LiveShape::Rectangle { .. }), "a turned rectangle is still one");
    assert_similar(&render_artboard(&d), &resvg_render(&svg(body), 200, 120), 24.0, 0.004);
    // Out: the uneven one with its transform; the turned one as a path (a rounded turn would move it).
    let out = export(&d, &ExportOptions::default());
    assert_eq!(out.matches("<rect").count(), 3, "{out}");
    assert!(out.contains("transform=\"matrix(2 0 0 1 85 5)\""), "{out}");
    assert_eq!(out.matches("<path").count(), 1, "{out}");
    let back = import(&out).unwrap();
    for (x, y) in got.iter().zip(lives(&back)).take(3) {
        assert!(same_shape(x, y.as_ref().unwrap()), "{x:?} != {y:?}\n{out}");
    }
    assert_similar(&render_artboard(&back), &render_artboard(&d), 24.0, 0.002);
}

#[test]
fn what_a_live_shape_cannot_be_stays_a_path() {
    let body = r##"<rect x="10" y="10" width="40" height="30" rx="10" ry="4" fill="#000"/>
<polygon points="60,10 100,10 80,40" fill="#000"/>
<polyline points="110,10 130,40 150,10" fill="none" stroke="#000"/>
<path d="M10 60 H60 V90 H10 Z" fill="#000"/>"##;
    let d = import(&svg(body)).unwrap();
    assert_eq!(lives(&d), vec![None, None, None, None], "unequal radii, polygons, polylines and paths are paths");
    assert_similar(&render_artboard(&d), &resvg_render(&svg(body), 200, 120), 24.0, 0.004);
}

#[test]
fn a_rectangle_clip_path_is_a_live_clipping_path() {
    let body = r##"<defs><clipPath id="t"><rect width="120" height="100" rx="20"/></clipPath></defs>
<g clip-path="url(#t)"><rect width="200" height="120" fill="#e8573f"/><circle cx="100" cy="60" r="30" fill="#fff"/></g>"##;
    let d = import(&svg(body)).unwrap();
    let mut clip = None;
    d.walk(|n| {
        if let NodeKind::Path { clipping: true, live, .. } = &n.kind {
            clip = live.clone();
        }
    });
    let clip = clip.expect("a live clipping path");
    assert!(
        same_shape(&clip, &LiveShape::Rectangle { w: 120.0, h: 100.0, radii: [20.0; 4], kinds: Default::default(), xf: Affine::IDENTITY }),
        "{clip:?}"
    );
    let out = export(&d, &ExportOptions::default());
    let def = &out[out.find("<clipPath").unwrap()..out.find("</clipPath>").unwrap()];
    assert!(def.contains("<rect") && def.contains("rx=\"20\"") && !def.contains("<path"), "{out}");
    let back = import(&out).unwrap();
    assert_eq!(lives(&back).len(), lives(&d).len());
    for (x, y) in lives(&d).iter().zip(lives(&back)) {
        assert!(same_shape(x.as_ref().unwrap(), y.as_ref().unwrap()), "{x:?} != {y:?}\n{out}");
    }
    assert_similar(&render_artboard(&d), &resvg_render(&svg(body), 200, 120), 24.0, 0.004);
    assert_similar(&render_artboard(&back), &render_artboard(&d), 24.0, 0.002);
}

/// A live shape whose path no longer is its shape, one with corners `<rect>` can't draw, and a pie
/// are written as paths.
#[test]
fn edited_and_unwritable_shapes_go_out_as_paths() {
    let mut d = Document::new(200.0, 120.0);
    let layer = d.layers[0].id;
    let black = kition_vector_color::Paint::solid(kition_vector_color::Color::rgb8(0, 0, 0));
    let fill = Appearance::basic(black, kition_vector_color::Paint::None, 0.0);
    let add = |d: &mut Document, live: LiveShape, path: Option<kition_vector_geom::PathData>| {
        let id = d.alloc_id();
        let path = path.unwrap_or_else(|| live.to_path());
        let mut n = Node::new(id, NodeKind::Path { path, rule: Default::default(), live: Some(live), clipping: false, guide: false });
        n.appearance = fill.clone();
        d.insert(Some(layer), usize::MAX, n).unwrap();
    };
    let rect = |radii: [f64; 4], kinds: [CornerKind; 4]| LiveShape::Rectangle { w: 30.0, h: 20.0, radii, kinds, xf: Affine::translate((10.0, 10.0)) };
    add(&mut d, rect([0.0; 4], Default::default()), None);
    add(&mut d, rect([2.0, 4.0, 2.0, 4.0], Default::default()), None);
    add(&mut d, rect([3.0; 4], [CornerKind::InvertedRound; 4]), None);
    add(&mut d, LiveShape::Ellipse { w: 30.0, h: 30.0, pie: (0.0, 90.0), xf: Affine::translate((60.0, 10.0)) }, None);
    let mut moved = rect([0.0; 4], Default::default()).to_path();
    moved.transform(Affine::translate((0.0, 50.0)));
    add(&mut d, rect([0.0; 4], Default::default()), Some(moved));
    let out = export(&d, &ExportOptions::default());
    assert_eq!(out.matches("<rect").count(), 1, "only the plain rectangle: {out}");
    assert_eq!(out.matches("<path").count(), 4, "{out}");
}
