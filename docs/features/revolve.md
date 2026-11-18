# Revolve

Revolve is a first implementation of the 3D Revolve effect, with the scope below.

Draw an open profile with the Pen tool, select it, then apply the effect: `effect.apply {"effect":
"threeD.revolve"}` (its catalogue entry, listed by `effect.list`, puts it under Effect › 3D and Materials ›
Revolve…). The profile's horizontal distance from a vertical axis is its radius. `edge` (`left` or `right`)
chooses the left or right edge of the profile's bounds; `offset` moves the axis outward. For a cylinder made from
a vertical line, give it a nonzero offset. A vertical line at zero offset lies on the axis and has no surface.

`angle` sweeps 0 to 360 degrees. `rotationX`, `rotationY` and `rotationZ` rotate the viewing direction, all three
0 degrees by default. `perspective` is zero for an orthographic view. `segments` controls radial resolution (8 to
128 for a full turn). Lighting is a directional light with azimuth, elevation, intensity and ambient contribution
(`lightAzimuth`, `lightElevation`, `lightIntensity`, `ambient`). Set `shade` to false to keep the source paint
without lighting.

`effect.setParams` changes the parameters. The source anchors remain editable, and the object's appearance keeps
the effect (`effect.list` reports it). SVG/PDF export evaluates filled vector faces; native saves retain the
profile and effect parameters.
**Object > Expand Appearance** replaces the live effect with ordinary editable vector faces.

`expandVisibleOnly` is on by default. Expansion then removes completely covered faces and trims partially covered
ones using the current rotation and perspective. Visible interior surfaces through open ends remain. Set it to
false to retain every projected face. It does not change the live preview or SVG/PDF export baking.

Trimming applies to opaque solid paint. Transparent objects, gradients, patterns, non-normal blending,
later geometry effects and surfaces exceeding bounded visibility limits keep their complete geometry
to preserve their appearance. Visibility follows the same painter order as the live effect, including
its approximation for intersecting surfaces. Expansion changes are undoable with the standard command.

## Initial limits

- Paths and compound paths only. Type, images and groups must be converted to paths first.
- Open profiles produce uncapped surfaces. Closed profile boundaries are swept as provided.
- One Revolve per source object. Edit its options rather than applying another.
- Object-level Revolve uses the top visible fill, falling back to a stroke. Per-item Revolve uses that item's paint.
- Flat shading, one directional light, no textures, cast shadows, end caps or physical materials.
- Faces use back-to-front depth sorting. Intersecting profiles can have incorrect visibility.
- Curves use sixteen samples per segment; profiles are limited to 1,024 samples and surfaces to 32,768 faces.
- Axis, bounds and ordinary selection handles continue to refer to the editable 2D source profile.
- Extrude & Bevel, Inflate, Rotate and Materials remain unimplemented.

## Architecture

`kition-vector-three-d` depends only on `kition-vector-geom`. It creates a surface of revolution,
applies Euler view rotation, projects it, shades faces and sorts their depth. It has no UI or GPU dependency.
`kition-vector-effects` turns those faces into document art. The renderer, export baking and
Expand Appearance use the same evaluator. The standard `effect.apply`, `effect.setParams`,
`effect.remove` and `effect.expandAppearance` commands expose it to the editor and to Kition's agent, through the
headless kit.

Behaviour reference: [public 3D effects documentation](https://helpx.adobe.com/illustrator/desktop/special-effects-styles/create-3d-graphics/create-3d-objects.html).
This implementation uses original mathematics and source code, with no Adobe software, assets or presets.
