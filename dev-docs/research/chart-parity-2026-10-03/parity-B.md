# Parity B: Pie, Radar, Radial vs shadcn (new-york-v4, Recharts 3.8.0)

> **Snapshot 2026-10-03.** Evidence files this report cites by relative name (screenshots, JSON probe dumps, probe scripts) were scratch artifacts of the session and are **not committed**; the findings and numbers are kept. See [README.md](./README.md).

Evidence: shadcn TSX in `parityB/src/*.tsx`, rendered shadcn SVG dumps in `parityB/shadcn/*.json|png`, ours in `parityB/ours/{pie,radar,radial}.json|-full.png` (built `docs/` served locally, HEAD be85681), arc parser `parityB/arcs.py` / `ours_arcs.py`.
Class: (P) port error in our demo, (E) engine semantics differ from Recharts, (D) deliberate.
Angle notation below: "ccw from 3" = Recharts convention (degrees, 0 = 3 o'clock, positive = counter-clockwise on screen).

---------------------------------------------------------------------
## 0. RADIAL (the user's complaint) — measured first

### 0.1 Root facts about Recharts RadialBarChart (all measured from the SVG paths)

1. Angles: `startAngle` default 0, `endAngle` default 360, direction is COUNTER-CLOCKWISE from 3 o'clock. Every shadcn radial arc is an SVG `A ... 0,0` (sweep-flag 0 = ccw). Simple: Chrome bar starts at (168.6,125) = 3 o'clock and runs ccw. Label (`startAngle=-90`): starts at (125,168.6) = 6 o'clock, runs ccw (toward 3 o'clock).
2. Value to sweep: `sweep = value / domainMax * (endAngle - startAngle)`, domain = `[0, max]` where max is the **max value in the data** (no "nice" rounding). Verified: Simple, Safari 200/275*360 = 261.8deg (path end at 262deg ccw), Other 90/275*360 = 117.8deg (measured 117.8). Label: Safari 200/275*470 = 341.8deg (measured 251.8 - (-90) = 341.8). So for Text (one datum, value 200) and Shape (one datum, value 1260) the value is the max, so the bar fills the WHOLE `endAngle` arc: Text = 250deg, Shape = 100deg. The number only matters as a label. (Our `1999` is fabricated; shadcn shows 200 and 1,260.)
3. Stacked (`stackId`): measured mobile 0 -> 81.4deg, desktop 81.4 -> 180deg (clamped at the endAngle). 81.4 = 570/1260*180, i.e. the domain max is the largest SINGLE value (1260), NOT the stack total (1830); the stack overflows the domain and is clamped to `endAngle`. Net visual: half-donut completely filled, boundary at 45% (not 31%). Series draw order = JSX order (mobile first, from 3 o'clock). This is a Recharts quirk; to match shadcn you need an explicit domain + clamp.
4. Ring geometry: bands split `[innerRadius, outerRadius]` equally, one band per datum, datum 0 INNERMOST. Within a band the bar is inset: Simple (inner 30, outer 110, step 16): bar width 12, offset 1.6 (rings r 31.6-43.6, 47.6-59.6, ... 95.6-107.6). Grid (outer 100, step 14): width 11, offset 1.4. Single-bar charts: bar = 80% of band (Text band 80-90 -> bar 81-89; Shape band 65-95 -> bar 68-92; Stacked 80-110 -> 83-107).
5. `<RadialBar background>`: each bar gets a FULL-sweep (startAngle->endAngle) track with exactly the bar's radii, fill `#eee` (`--muted`). Text/Shape/Stacked: sweeps are partial so the track is partial too (250deg / 100deg) PLUS the PolarGrid circles (see below).
6. `cornerRadius`: clamped to half the bar thickness (Text: bar 8 wide -> rc 4; Stacked rc 5 on every stacked segment, both ends of each segment rounded).
7. `<PolarGrid gridType="circle" polarRadius=[90,80] className="first:fill-muted last:fill-background">` in Text/Shape draws two full 360deg circles (r 90 muted, r 80 background) = a full muted ANNULUS under the bar. So the visible track is a complete ring, with the colored bar over part of it. (Shape: annulus r 74-86 inside a bar band 68-92: a thinner gray ring, narrower than the bar, visible beyond the 100deg sweep.)
8. `<PolarGrid gridType="circle">` (Radial Grid): NO background tracks. Draws 5 stroke circles at the ring centers (37, 51, 65, 79, 93 = band centre approx) plus 14 radial spoke lines (every 20 domain units = 26.2deg, from r=30 to r=100), stroke `#ccc`.

### 0.2 Per chart

Normalization: shadcn svg 250x250 (half 125), ours svg viewBox 300x300 (half 150, shown at 350px in the gallery).

| chart | shadcn | ours | difference | class |
|---|---|---|---|---|
| Simple | start 3 o'clock, ccw; domain [0,275]; 5 rings inner 30 / outer 110 (0.24 / 0.88 of half-size); bar 12/16 of band; `background` track on every ring | start 12 o'clock, clockwise (`start_angle 0.0` default); domain [0,max] OK; inner 30 / outer auto 135 (0.20 / 0.90); bar width 17.3 of 21 step (RING_PADDING 0.15); NO track | direction + start; no track (our demo omits `grid: true`; and radial.rs:80 `grid` is documented as the `background` equivalent); proportions; card shows 350px vs shadcn 250px max | direction/start: (E) no Recharts-convention angle API (radians cw from 12). track: (P). radii/size: (P)+(E) |
| Label | `startAngle -90, endAngle 380` (sweep 470deg, starts 6 o'clock, ccw); `LabelList position="insideStart"` fontSize 11, white (`fill-white mix-blend-luminosity`), text path on ring centreline radius = band centre (37.6, 53.6, ...), starts 5px in; track on every ring | no start/end set => 0..360 cw from 12; labels via `PieLabels::List` on textPath start 8px inside arc, text colour = chart text colour with stroke halo (`chart-arc-label` CSS, font min(xs, 0.85*thickness)); no track | angles not ported at all; label colour/halo differ; track missing | angles (P) + (E); label styling (D)/(P) |
| Grid | circle PolarGrid: 5 stroke circles + 14 spokes, no tracks, outer 100 | `grid: true` => 5 filled `#eee`-ish full-sweep TRACK annuli (data-slot chart-polar-grid, fill) | different feature: shadcn "grid" = PolarGrid lines (circles + spokes), ours = background tracks (what shadcn calls `background`). Our Grid card is visually the shadcn Simple card | (E) no PolarGrid-lines option in RadialOptions; (P) demo maps wrong feature |
| Text | `startAngle 0, endAngle 250`, inner 80 / outer 90, value 200 = max so bar fills 250deg, ccw from 3 o'clock; cornerRadius 10 (clamped to 4); full 360deg muted annulus r 80-90 underneath; center text "200" (text-4xl bold) + "Visitors" at y+24 | full 360deg ring (end_angle default TAU), inner 80 / outer auto 127.8 (ring is 40 thick vs 8), value fabricated 1999, center text "1,999" | whole shape wrong: full ring instead of 250deg arc, ring 5x too thick (inner_radius 80 of a 300 box with auto outer 0.9*150 ignores that Recharts outer is explicit 90), no rounded ends, wrong number | (P) endAngle/outer_radius/cornerRadius/value not ported; (E) `corner_radius` only applied if set; no "full annulus under partial bar" notion (our `grid` track uses the sweep of the bar) |
| Shape | `endAngle 100`, inner 65 / outer 95 -> bar 68-92, 100deg, **no cornerRadius** (sharp ends), track = 100deg `#eee` bar shape + full grey annulus r 74-86; center "1,260" | full ring, inner 80 / outer 127.8, `corner_radius: 10` (irrelevant on a full ring), no center text, no number | no partial sweep; ours ADDS cornerRadius that shadcn does not have; center text missing (shadcn shows "1,260 / Visitors") | (P) |
| Stacked | `endAngle 180`, inner 80 / outer 110 -> bar 83-107, rc 5 on each segment (both ends), order mobile then desktop, mobile = 0-81.4deg, desktop = 81.4-180deg (domain [0,1260] clamp, see 0.1.3), half-donut sits above centre, "1,830" (toLocaleString), no legend, tooltip hideLabel | full circle (0..TAU), inner 80 / outer 135, desktop first (0-248deg = 1260/1830) then mobile, no rounding, center text "1830" (no thousands separator), `ChartLegend` shown, tooltip has label | arc 360 vs 180, order reversed (config order), proportions 69/31 vs 55/45, no rounded ends, number format, legend that shadcn doesn't have | (P) endAngle, corner_radius, legend, format; (E) series order = config order (Recharts = element order) and no domain/clamp |

Our tooltip in radial: shadcn `hideLabel nameKey="browser"`; ours Simple/Grid `hide_label: true` OK.

---------------------------------------------------------------------
## 1. PIE

Recharts Pie facts (measured): default `startAngle 0 / endAngle 360`, slices run COUNTER-CLOCKWISE from 3 o'clock. Chrome (107deg) occupies 0 -> 107deg ccw (upper right through top to upper left); ours Chrome occupies the upper-right quadrant + 3 o'clock -> about 4 o'clock (cw from 12). Default `outerRadius` = 80% of `(min(w,h) - 2*margin(5))/2` = 96 in a 250 box (0.768 of half-size); ours 0.9 of half-size. `innerRadius 60` is absolute px (0.625 of outer 96).

Size: shadcn container `aspect-square max-h-[250px]` (svg 250x250, pie diameter 192px); our demos use the default `Chart` 600x300 viewBox, rendered 350x175 in the card: pie r = 135*350/600 = 78.8px, diameter 157px, svg only 175px tall. (P: demos should set `width/height 250x250`; E: OUTER_RADIUS_FRACTION 0.9 vs 0.768.)

| chart | shadcn | ours | difference | class |
|---|---|---|---|---|
| Simple (main) | ccw from 3, r 96, slice angles 107/77.9/72.7/67.4/35.0 | cw from 12, r 135/150 (0.9), same sizes | direction/start differ (slice order is a mirror image rotated), radius ratio, svg shape 2:1 | (E) angle convention, radius default; (P) viewBox 600x300 |
| Separator None | same as Simple, `stroke="0"` | same as ours Simple | same | same |
| Label | `label` default: leader line (r 96 -> 116, stroke = slice colour) + value text at r+20 outside, anchor start/end by side, `.recharts-pie-label-text` fill foreground | value text inside at centroid (halo stroke), no leader | outside-with-leader vs inside-centroid (pie.rs header documents it as a simplification) | (D) documented, but it is a visible parity gap -> (E) feature |
| Custom Label | `labelLine={false}`, raw `payload.visitors` (275 ...) at r+20 outside, no legend | percent text ("30%") inside at centroid, `ChartLegend` rendered | text content (percent vs raw), position (outside), extra legend | (P) text/legend; (E) outside placement |
| Label List | `LabelList dataKey="browser"` inside at centroid (r = 48 for outer 96, i.e. (inner+outer)/2), fontSize 12 | names via `PieLabels::List` at centroid (lowercase browser keys "chrome"), halo stroke, no legend | position matches (centroid) but ours renders raw ids ("chrome") where shadcn renders config labels ("Chrome") | (P) label strings |
| Legend | `max-h-[300px]`, legend below, pie centre shifted up, r 95.2 | small pie (r 78.8px) + legend | geometry as Simple | (P)/(E) |
| Donut | inner 60, outer 96, ccw from 3 | inner 60 (of 150-half), outer 135 => hole ratio 0.44 vs 0.625 (ring much thicker) | hole too small relative: `inner_radius` px is not scaled with the 0.9 auto-outer | (P)+(E) |
| Donut Active | index 0 outer 106 (= outer + 10), inner stays 60, `strokeWidth 5` transparent | CSS `transform: scale(1.05)` about centre => outer +6.75 and inner 63 (style.css:610) | pop-out is a global 5% scale (inner radius moves too) instead of outerRadius + 10 | (E) active semantics, (D) partly |
| Donut Text | centre "1,125" = 275+200+287+173+190; slice angles 88/64/91.8/55.4/60.8 (different data from the other pies: firefox 287, other 190) | "925" and the generic 275/200/187/173/90 data | data not ported (shadcn donut-text uses its own `chartData`) | (P) |
| Stacked | two separate Pies: inner full disc r 0-60 (desktopData, 5 months), outer ring r 70-90 (mobileData, 5 months, different slice angles 40/100/60/95/65), colours per month, tooltip line indicator, legend none | ONE data table with 6 months (June added) x 2 series; BandScale over [30,135] gives rings 34-80 and 84.5-131, 30px hole, ring order by config series; `ChartLegend` shown (legend of series names desktop/mobile) | geometry (disc vs ring, gap 10 vs 4), data (6 vs 5 months, mobile per-month angles come from the same categories), legend, hole | (P) data/legend/hole; (E) `PieOptions` cannot express two independent rings with explicit radii; ring radii come from a BandScale instead of explicit `outerRadius=60` / `innerRadius=70..90` |
| Interactive | 5 months (desktop), inner 60/outer 116 in a 300 box, active slice outer +10 AND a halo ring r 128-141 (`renderPieShape`), centre text = active value ("186"), Select | 6 slices (incl. June), `active_index` via Select, scale 1.05, centre text | slice count (5 vs 6), no halo ring, scale vs +10 | (P) data; (E) active shape |

Pie angle layout code: `pie_layout` (primitives/src/chart/engine/polar.rs:515) is fixed to a forward full turn (`da_full = TAU`, line 528, `a1 = a0 + ...` line 563); PieOptions has no start/end angle at all (pie.rs:96-125), and every call passes start `0.0` (pie.rs:155, 185, 258 ...).

---------------------------------------------------------------------
## 2. RADAR

Recharts Radar facts (measured from `chart-radar-default`): outer radius = 96 in a 250 box (0.768 of half-size), 6 spokes, first category at 12 o'clock, clockwise (same as ours). PolarGrid: 5 concentric polygons incl. a degenerate r=0 one: radii 0, 24, 48, 72, 96. Radius domain: nice-tick domain with tickCount 5 = [0, 320] for max 305 (ticks 0,80,160,240,320): vertex Jan 186 -> r 55.8 = 186/320*96. Radar polygons: NO stroke (computed stroke none), `fill-opacity` 0.6 only where `fillOpacity` is given; the second `<Radar>` without `fillOpacity` is OPAQUE (computed fill-opacity 1) and drawn on top. Angle-axis tick labels: placed at r+8 along the spoke, `text-anchor` start for right-side ticks, end for left-side, middle at the poles. Angle-axis line = outer polygon (no stroke), tick lines no stroke.

| chart | shadcn | ours | difference | class |
|---|---|---|---|---|
| Default (main) | outer 96/125 = 0.768; grid rings 0,24,48,72,96 (5 incl. 0); domain [0,320] (Jan r = 0.581 of outer) | outer 103.5/150 = 0.69 (outer_radius 0.8 * (half - 24*text_scale)); rings 0,14.8,...,103.5 = 8 rings (domain 350, `ticks(5)` -> step 50); Jan r = 0.531 of outer; 2px series-coloured outline on the polygon (CSS `.dx-chart [data-slot=chart-radar-area]` stroke 2px); rim labels centred (text-anchor middle) at outer+12 | ring count 5 vs 8, domain 320 vs 350 (vertex positions shift by 9%), outline that shadcn doesn't draw, label anchoring (our "February"/"March" sit centred on the ray end so they overlap the grid; shadcn anchors start/end so they sit outside), size ratio | (E) tick algorithm (d3 `ticks(5)` + `nice_domain` instead of Recharts' exactly-tickCount nice ticks) radar.rs:162,197; (E) text-anchor radar.rs:365; (E) CSS stroke style.css:556; (D) size |
| Dots | `dot={{r:4, fillOpacity:1}}` | r = 3 | 3 vs 4 | (E) fixed r in CSS/engine |
| Lines Only | two Radars, `fillOpacity 0`, stroke 2px each, `radialLines={false}` (rings only, NO spokes) | `lines_only: true`, spokes still drawn (screenshot shows spokes) | spokes present in ours, absent in shadcn | (P/E) `lines_only` does not map to `radialLines={false}`; no per-grid `radial_lines` flag except `Custom{spokes}` |
| Multiple | desktop `fillOpacity .6` + mobile with default (opaque, drawn on top); no strokes | both series 0.6 + 2px outlines (one chart-wide `fill_opacity`, radar.rs:124) | second series opaque in shadcn | (E) per-series fill opacity impossible (`RadarOptions.fill_opacity` is chart-wide) |
| Legend / Icons | `margin top -40, bottom -10` (pie shifted up), legend `mt-8`; icons variant uses icon legend (config `icon`) | no margin option; legend below; icons present but different glyph | layout offset | (D)/(P) |
| Radius Axis | NO angle axis, `PolarRadiusAxis angle=60 orientation=middle` -> numeric ticks 0,80,160,240,320 along the 60deg spoke (text only; axisLine false) | `axis_labels: false`; no radius axis ticks at all | radius tick numbers missing entirely (feature gap); also ring count 7 vs 4 + same domain issue | (E) no PolarRadiusAxis |
| Label Custom | custom tick: two-line "desktop/mobile" + month, margin 10 | default centred labels; docstring says "no hook to replace the label markup" | custom ticks not expressible | (E) no tick render hook (documented) |
| Grid Custom | `radialLines={false} polarRadius={[90]}` = ONE ring at radius 90 PIXELS (of 96 outer, essentially the outer edge), no spokes | `RadarGrid::Custom { values: vec![90.0], spokes: false }` where values are in DATA units => ring at 90/350 of outer (tiny hexagon near centre, see screenshot) | unit mismatch: Recharts `polarRadius` is pixels, ours is data-domain | (P) port; (E) unit semantics (demo should use ~300 or the engine take a fraction) |
| Grid None | PolarAngleAxis labels + dots, no grid | same | ok (except outer/size, dot r) | ok |
| Grid Circle | circle rings + spokes, dots r 4 | circle rings (8) + spokes, dots r 3 | ring count, dot r | (E) as Default |
| Grid Circle - No lines | circle rings, no spokes | circle rings, spokes... ours renders none ("CircleNoLines") | ok except count | ok |
| Grid Circle Fill | `fill-(--color-desktop) opacity-20` circles = cumulative translucent tint, radar fill .5, no outline | cumulative tint rings (looks similar), 2px outline, 8 rings | outline, ring count | (E) CSS stroke |
| Grid Fill | same with polygons | same | same | same |

---------------------------------------------------------------------
## 3. Classes (by-construction fixes)

### Class A. Angle convention (E; subsumes Radial Simple/Label/Text/Shape/Stacked direction+start, Pie all variants)
Cause: engine angles are radians, 0 = 12 o'clock, clockwise (engine/polar.rs module doc lines 1-17); shadcn/Recharts are degrees, 0 = 3 o'clock, ccw. Port is a pure mapping: our_angle = pi/2 - to_radians(recharts_deg) (check: Recharts 0deg -> pi/2 = 3 o'clock; Recharts 250deg -> pi/2 - 4.363 = -2.79 rad, a ccw sweep, which `arc_path` (polar.rs:~255, `cw = a1 > a0`), `Sector::contains` (components/pointer.rs:81-96, handles `a1 < a0`) and radial `ArcLabel::centerline` (radial.rs ~480, `sign` handling) ALREADY support). Only `pie_layout` (polar.rs:515, fixed forward TAU, lines 528/563) and the demos' defaults don't.
Fix: introduce `engine::polar::Angle` / `RechartsAngles { start_deg, end_deg }` (one constructor `Angle::recharts_deg(f64) -> f64 (radians, engine convention)`), make `RadialOptions.start_angle/end_angle` (radial.rs:63,68) and a new `PieOptions.start_angle/end_angle` take it, and make their `Default` the Recharts defaults (0..360 ccw from 3). Extend `pie_layout` with an `end_angle` (sweep sign = sign(end-start)) instead of the constant TAU. The demos then literally copy shadcn's numbers (`startAngle=-90, endAngle=380`, `0/250`, `endAngle=100`, `endAngle=180`). Parity test: for each in-scope chart commit the shadcn arc table (start/end degrees ccw from 3, normalised radii; `parityB/arcs.py` output is the generator) as JSON and assert ours within 0.5deg / 1 unit.

### Class B. RadialBar value to sweep (E; Radial Text/Shape/Stacked, and the numbers on all)
Measured: domain = [0, max of data] mapped onto (endAngle - startAngle); stacked clamped to the sweep, domain max = largest single value (1260 not 1830).
Ours: radial.rs:166/215/291 `max_value` over `values` (matches Recharts for unstacked: OK), radial.rs:371-372 stacked uses the stack TOTAL as 100% (so 1260/570 gives 69/31 vs Recharts 55/45).
Fix: add `RadialOptions.domain_max: Option<f64>` (default = data max, as now) and clamp each stacked cumulative end to `end_angle`; stacked demo passes `domain_max: Some(1260.0)`. Single-datum Text/Shape need only `end_angle` (value = max). Series draw order for stacked: use an explicit `series_order` or document that config order = stack order and re-order the demo's config (mobile, desktop) (radial.rs:354-400 iterates `ctx.config.series`).

### Class C. Radial track vs PolarGrid (E+P; Simple/Label/Grid/Text/Shape)
Cause: radial.rs:80 `grid: bool` collapses two Recharts things: `RadialBar background` (per-bar sweep-shaped track, `#eee`) and `PolarGrid` (circles + spokes, or the Text/Shape full annulus via `polarRadius`).
Fix: split into `background: bool` (= current track, sweep-shaped like the bar; Simple/Label/Text/Shape set it), `grid: Option<RadialGrid>` with `RadialGrid::Circles { spokes: bool }` (stroke circles at ring centres + spokes every 20 value units, Grid demo) and `RadialGrid::Annulus { inner, outer }` (full-360 muted ring, Text/Shape). Port fix: Simple/Label set `background: true`.

### Class D. Explicit radii vs auto 0.9 fraction, and viewBox (E+P; every radial/pie chart)
Cause: Pie ignores any outer radius (pie.rs:63 `OUTER_RADIUS_FRACTION 0.9`, geometry pie.rs:129) and Radial auto-sizes unless `outer_radius` set (radial.rs:36,127); demos use 300x300 (radial) or the default 600x300 (pie) while shadcn is 250x250 with absolute px radii (inner 30, outer 110, ...). Same px `inner_radius` therefore means a different hole ratio (Donut 0.44 vs 0.625).
Fix by construction: make the demos' viewBox 250x250 and give both `PieOptions` and `RadialOptions` an `outer_radius: Option<Radius>` with `Radius::Px(f64)` / `Radius::Percent(f64)`, default Pie = `Percent(80)` of `(min(w,h) - 10)/2` (Recharts default), so a shadcn snippet's numbers apply 1:1. Plus a card-level `max-height: 250px; aspect-ratio: 1` for polar charts (shadcn `aspect-square max-h-[250px]`); ours is stretched to card width (350px), so pies/radials render 40% larger in px than shadcn.

### Class E. Active shape semantics (E; Donut Active, Interactive)
Cause: style.css:610 `[data-slot=chart-arc][data-active=true]{transform:scale(1.05)}`. Recharts `Sector outerRadius+10` keeps the hole; interactive adds a halo ring (128-141).
Fix: render the active slice's geometry in the engine (`PieOptions.active_outer_extra: f64 = 10.0`, optional `active_halo: Option<(f64,f64)>`) instead of CSS scale.

### Class F. Pie labels (E; Label, Custom Label)
pie.rs header documents the simplification: labels at the centroid. Add `PieLabels::Outside { offset: 20.0, line: bool }` (leader line r -> r+offset, text anchor start/end by side, fill foreground). Label List stays centroid (already matches, r = 48).

### Class G. Radar scale/ticks/labels/strokes (E)
1. radar.rs:162,197 `ticks(5)`+`nice_domain` gives 8 rings / 350; Recharts yields exactly 5 ticks 0..320 (nice-tick with tickCount 5). Use a Recharts-style `get_nice_tick_values(0, max, 5)` for the radar (and likely the Cartesian y-axis, other lane).
2. radar.rs:365 `text-anchor: "middle"` for all labels at outer+12 -> start/end by sign of x (middle within +/-eps), at outer+8 like Recharts.
3. radar.rs:124 `fill_opacity` chart-wide, plus CSS stroke 2px (style.css:556): make it per-series (`Vec<f64>`/config field, default 0.6 for the first series, 1.0 otherwise as Recharts) and remove the default outline (stroke only for `lines_only`).
4. `lines_only` should imply `radialLines=false`; add `RadarGrid::radial_lines: bool` to match `PolarGrid radialLines`.
5. `PolarRadiusAxis` (angle 60, orientation middle) missing; add `RadarOptions.radius_axis: Option<{angle_deg}>` drawing tick texts.
6. `RadarGrid::Custom.values` are in data units while Recharts `polarRadius` is px: either rename to `radii_fraction` or document; fix the grid_custom demo to ~300 (=90/96*320).
7. Dots r 4 (`dot={{r:4}}`) vs 3 hard-coded.

### Class H. Port errors in demos (P)
- Radial Text: `end_angle` 250deg ccw, `outer_radius 90`, `inner 80`, `corner_radius 10`, value 200 (center text "200"), annulus 80-90 (class C).
- Radial Shape: `end_angle` 100deg, inner 65 / outer 95, NO corner_radius, value 1260, center text "1,260".
- Radial Stacked: end 180deg, inner 80 / outer 110, corner_radius 5, no legend, center "1,830" (thousands separator), config order mobile, desktop.
- Radial Label: start -90 / end 380 (Recharts), white labels.
- Radial Simple/Label: `background` tracks on.
- Pie Donut Text: own dataset (275,200,287,173,190), total 1,125. Pie Interactive/Stacked: 5 months, not 6. Pie Stacked: two independent rings (disc 0-60, ring 70-90) and no legend/hole. Custom Label: raw values, no legend. Label List: config labels (capitalised). Donut: inner 60 on a 250 box.
- Radar Grid Custom: ring 90px => data value ~300. Lines Only: no spokes. Multiple/Legend/Icons/Radius/LabelCustom: mobile series opaque. Radius Axis: needs radius axis ticks.

### Parity test (prevents recurrence of the whole class)
Commit `tests/shadcn-geometry/<chart>.json` (from `parityB/shadcn/*.json` run through `arcs.py`: per arc `{outerR/halfsize, innerR/halfsize, startDeg, endDeg}`; radar `{ringRadii/outer, vertexRadius/outer}`) and a Playwright/`cargo test` that renders each demo and asserts the same numbers after normalising by half-size and converting engine radians via `Angle::to_recharts_deg`. One test covers every class above for pie, radar and radial.

---------------------------------------------------------------------
## 4. Biggest differences (priority)
1. Every Pie/Radial arc starts at the wrong place and runs the wrong direction (cw from 12 vs ccw from 3).
2. Radial Text/Shape/Stacked are full rings with the wrong radii (ring 5x too thick), fabricated 1,999, no rounded ends; Stacked has swapped order, wrong split (69/31 vs 45/55), a legend, and no thousands separator.
3. Radial Grid is tracks not PolarGrid circles+spokes; Simple/Label lack the `background` tracks.
4. Pies are ~18% smaller in radius (0.9 of half vs 0.768 of a 250 box rendered at 250px; we render at 350px so px radius 79 vs 96), donut hole 0.44 vs 0.625, labels inside rather than outside, Stacked pie / Donut Text / Interactive use wrong data.
5. Radar: 8 rings/domain 350 vs 5 rings/domain 320, outlines shadcn lacks, centred rim labels, chart-wide fill opacity, no radius axis, Grid Custom ring at the wrong radius.
