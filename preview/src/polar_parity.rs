//! Parity of the `pie_chart`, `radar_chart` and `radial_chart` galleries with
//! shadcn/ui's `chart-pie-*`, `chart-radar-*` and `chart-radial-*` demos.
//!
//! `preview/tests/shadcn/chart-{pie,radar,radial}-*.json` are derived by
//! `scripts/extract-shadcn-polar-fixtures.mjs` (never hand-typed) from the
//! registry source -- card text, data, Recharts props -- and from the SVG
//! ui.shadcn.com actually renders (`rendered`: every sector's radii and
//! angles, the radar's rings and vertices, the label positions, measured).
//! Each test server-renders our demo at its initial size (the 250px square
//! shadcn's `max-h-[250px] aspect-square` gives, 300px where shadcn's demo
//! is 300px) and holds what it draws to those numbers: px for radii and
//! positions, Recharts degrees (0 = three o'clock, counter-clockwise) for
//! angles. A demo whose port drifts (data, angles, radii, labels, legend)
//! or an engine change that moves the geometry fails here.
//!
//! The one deliberate difference -- a stacked radial bar's segments split
//! the sweep by the stack total, where Recharts clamps an overflowing stack
//! (`components::series::radial`'s module doc) -- is checked as such.
use dioxus::prelude::*;
use serde_json::Value;

use crate::components::{pie_chart, radar_chart, radial_chart};

type Demo = fn() -> Element;

/// (fixture name, our demo) for every shadcn polar chart.
fn demos() -> Vec<(&'static str, Demo)> {
    use pie_chart::variants as pie;
    use radar_chart::variants as radar;
    use radial_chart::variants as radial;
    vec![
        ("chart-pie-simple", pie::main::Demo),
        ("chart-pie-separator-none", pie::separator_none::Demo),
        ("chart-pie-label", pie::label::Demo),
        ("chart-pie-label-custom", pie::label_custom::Demo),
        ("chart-pie-label-list", pie::label_list::Demo),
        ("chart-pie-legend", pie::legend::Demo),
        ("chart-pie-donut", pie::donut::Demo),
        ("chart-pie-donut-active", pie::donut_active::Demo),
        ("chart-pie-donut-text", pie::donut_text::Demo),
        ("chart-pie-stacked", pie::stacked::Demo),
        ("chart-pie-interactive", pie::interactive::Demo),
        ("chart-radar-default", radar::main::Demo),
        ("chart-radar-dots", radar::dots::Demo),
        ("chart-radar-lines-only", radar::lines_only::Demo),
        ("chart-radar-multiple", radar::multiple::Demo),
        ("chart-radar-legend", radar::legend::Demo),
        ("chart-radar-icons", radar::icons::Demo),
        ("chart-radar-radius", radar::radius::Demo),
        ("chart-radar-label-custom", radar::label_custom::Demo),
        ("chart-radar-grid-custom", radar::grid_custom::Demo),
        ("chart-radar-grid-none", radar::grid_none::Demo),
        ("chart-radar-grid-circle", radar::grid_circle::Demo),
        (
            "chart-radar-grid-circle-no-lines",
            radar::grid_circle_no_lines::Demo,
        ),
        (
            "chart-radar-grid-circle-fill",
            radar::grid_circle_fill::Demo,
        ),
        ("chart-radar-grid-fill", radar::grid_fill::Demo),
        ("chart-radial-simple", radial::main::Demo),
        ("chart-radial-label", radial::label::Demo),
        ("chart-radial-grid", radial::grid::Demo),
        ("chart-radial-text", radial::text::Demo),
        ("chart-radial-shape", radial::shape::Demo),
        ("chart-radial-stacked", radial::stacked::Demo),
    ]
}

fn fixture(name: &str) -> Value {
    let path = format!("{}/tests/shadcn/{name}.json", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn render(demo: Demo) -> String {
    let mut dom = VirtualDom::new(demo);
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

// ---------------------------------------------------------------------------
// Minimal HTML reading: the opening tags of the elements we care about.
// ---------------------------------------------------------------------------

/// Every opening tag (`<name ...>`) whose attributes contain `needle`.
fn tags<'a>(html: &'a str, needle: &str) -> Vec<&'a str> {
    html.match_indices(needle)
        .filter_map(|(at, _)| {
            let start = html[..at].rfind('<')?;
            let end = html[at..].find('>')? + at;
            Some(&html[start..=end])
        })
        .collect()
}

/// Every `<name ...>` opening tag.
fn open_tags<'a>(html: &'a str, name: &str) -> Vec<&'a str> {
    let open = format!("<{name} ");
    html.match_indices(&open)
        .filter_map(|(at, _)| Some(&html[at..=html[at..].find('>')? + at]))
        .collect()
}

fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let key = format!(" {name}=\"");
    let start = tag.find(&key)? + key.len();
    tag[start..].split('"').next()
}

fn num(tag: &str, name: &str) -> f64 {
    attr(tag, name)
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| panic!("no numeric {name} in {tag}"))
}

/// The text content of the element whose opening tag is `tag`, a slice of
/// `html` (from [`tags`]/[`open_tags`], so identical tags stay distinct).
fn text_after(html: &str, tag: &str) -> String {
    let at = (tag.as_ptr() as usize)
        .checked_sub(html.as_ptr() as usize)
        .filter(|at| *at < html.len())
        .expect("tag is a slice of html")
        + tag.len();
    let mut depth = 1;
    let mut out = String::new();
    let mut rest = &html[at..];
    while depth > 0 && !rest.is_empty() {
        if let Some(stripped) = rest.strip_prefix("</") {
            depth -= 1;
            rest = &stripped[stripped.find('>').map_or(stripped.len(), |i| i + 1)..];
        } else if rest.starts_with('<') {
            let end = rest.find('>').map_or(rest.len(), |i| i + 1);
            if !rest[..end].ends_with("/>") {
                depth += 1;
            }
            rest = &rest[end..];
        } else {
            let end = rest.find('<').unwrap_or(rest.len());
            out.push_str(&rest[..end]);
            rest = &rest[end..];
        }
    }
    out
}

/// The polar centre our chart drew around: the series group's translate.
fn centre(html: &str) -> (f64, f64) {
    let at = html
        .find("transform=\"translate(")
        .expect("a translated series group");
    let args = &html[at + "transform=\"translate(".len()..];
    let args = &args[..args.find(')').unwrap()];
    let mut it = args.split(',').map(|v| v.trim().parse::<f64>().unwrap());
    (it.next().unwrap(), it.next().unwrap())
}

fn close(what: &str, ours: f64, theirs: f64, tol: f64) {
    assert!(
        (ours - theirs).abs() <= tol,
        "{what}: ours {ours} vs shadcn {theirs}"
    );
}

/// Angles compared modulo a turn.
fn close_deg(what: &str, ours: f64, theirs: f64) {
    let d = (ours - theirs).rem_euclid(360.0);
    assert!(
        d.min(360.0 - d) <= 0.05,
        "{what}: ours {ours} vs shadcn {theirs}"
    );
}

const PX: f64 = 0.02;

// ---------------------------------------------------------------------------
// Card text, data, legend, tooltip
// ---------------------------------------------------------------------------

#[test]
fn card_text_matches() {
    for (name, demo) in demos() {
        let fx = fixture(name);
        let html = render(demo);
        for key in ["title", "description"] {
            let text = fx[key].as_str().unwrap();
            assert!(
                html.contains(&format!(">{text}<")),
                "{name}: {key} {text:?}"
            );
        }
        let footer = fx["footer"].as_array().unwrap();
        for line in footer {
            let line = line.as_str().unwrap();
            assert!(html.contains(line), "{name}: footer line {line:?}");
        }
        if footer.is_empty() {
            assert!(
                !html.contains("Trending up"),
                "{name}: shadcn has no footer"
            );
        }
    }
}

#[test]
fn legend_and_tooltip_presence_match() {
    for (name, demo) in demos() {
        let fx = fixture(name);
        let html = render(demo);
        assert_eq!(
            html.contains("data-slot=\"chart-legend\""),
            fx["legend"].as_bool().unwrap(),
            "{name}: legend"
        );
        assert_eq!(
            html.contains("data-slot=\"chart-tooltip\""),
            !fx["tooltip"].is_null(),
            "{name}: tooltip"
        );
    }
}

/// The category label shadcn shows for a data row: the config label of its
/// first string field's value (`"chrome"` -> `"Chrome"`), else the value.
fn category(fx: &Value, row: &Value) -> String {
    let raw = row
        .as_object()
        .unwrap()
        .iter()
        .filter(|(k, _)| k.as_str() != "fill")
        .find_map(|(_, v)| v.as_str())
        .unwrap();
    fx["config"][raw]["label"]
        .as_str()
        .map_or_else(|| raw.to_string(), str::to_string)
}

/// The data rows shadcn draws: `chartData`, or (`chart-pie-stacked`,
/// `chart-pie-interactive`) `desktopData` merged with `mobileData`.
fn rows(fx: &Value) -> Vec<(String, Vec<f64>)> {
    let keys: Vec<String> = ["pies", "radialBars", "radars"]
        .iter()
        .flat_map(|k| fx[*k].as_array().cloned().unwrap_or_default())
        .map(|el| el["dataKey"].as_str().unwrap().to_string())
        .collect();
    let base = if fx["data"].is_array() {
        &fx["data"]
    } else {
        &fx["desktopData"]
    };
    base.as_array()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let values = keys
                .iter()
                .map(|k| {
                    row[k.as_str()]
                        .as_f64()
                        .or_else(|| fx["mobileData"][i][k.as_str()].as_f64())
                        .unwrap()
                })
                .collect();
            (category(fx, row), values)
        })
        .collect()
}

#[test]
fn data_matches() {
    for (name, demo) in demos() {
        let fx = fixture(name);
        let html = render(demo);
        let table = tags(&html, "data-slot=\"chart-data\"")[0];
        let body = text_after(&html, table);
        let expected = rows(&fx);
        // Each row of the hidden table: its category, then its values.
        let trs: Vec<&str> = html.split("<tr").skip(2).collect();
        assert_eq!(trs.len(), expected.len(), "{name}: row count\n{body}");
        for (tr, (label, values)) in trs.iter().zip(&expected) {
            assert!(
                tr.to_lowercase()
                    .contains(&format!(">{}<", label.to_lowercase())),
                "{name}: row {label}: {tr}"
            );
            let cells: Vec<f64> = tr
                .split("<td")
                .skip(1)
                .filter_map(|c| {
                    let text = &c[c.find('>')? + 1..c.find("</td>")?];
                    text.replace(',', "").trim().parse().ok()
                })
                .collect();
            // Pie/radial tables carry the first series (and a percent);
            // radar tables every series.
            let n = if name.starts_with("chart-radar") {
                values.len()
            } else {
                1
            };
            assert_eq!(&cells[..n], &values[..n], "{name}: row {label}");
        }
    }
}

// ---------------------------------------------------------------------------
// Geometry
// ---------------------------------------------------------------------------

/// Our arcs in draw order: `(inner, outer, start, sweep, is_background)`.
fn our_sectors(html: &str) -> Vec<(f64, f64, f64, f64, bool)> {
    let mut out = Vec::new();
    // Document order matters (Recharts draws backgrounds first, a halo right
    // after its slice), so walk every arc-like tag in one pass.
    for tag in tags(html, "data-slot=\"chart-") {
        let slot = attr(tag, "data-slot").unwrap_or("");
        let background = slot == "chart-radial-background";
        if !(background || slot == "chart-arc" || slot == "chart-arc-halo") {
            continue;
        }
        if slot == "chart-arc-halo" {
            // The halo shares its slice's angles; radii from its own `d`.
            let (_, _, start, sweep, _) = *out.last().expect("halo after its slice");
            let d = attr(tag, "d").unwrap();
            let radii: Vec<f64> = d
                .split('A')
                .skip(1)
                .filter_map(|a| a.split(',').next()?.parse().ok())
                .collect();
            let (lo, hi) = radii
                .iter()
                .fold((f64::MAX, 0.0f64), |(lo, hi), r| (lo.min(*r), hi.max(*r)));
            out.push((lo, hi, start, sweep, false));
            continue;
        }
        let start = num(tag, "data-start-angle");
        let end = num(tag, "data-end-angle");
        out.push((
            num(tag, "data-inner-radius"),
            num(tag, "data-outer-radius"),
            start,
            (end - start).min(360.0),
            background,
        ));
    }
    out
}

#[test]
fn pie_and_radial_sectors_match_rechartss() {
    for (name, demo) in demos() {
        if name.starts_with("chart-radar") {
            continue;
        }
        let fx = fixture(name);
        let rendered = &fx["rendered"];
        let html = render(demo);
        let (cx, cy) = centre(&html);
        close(
            &format!("{name}: cx"),
            cx,
            rendered["cx"].as_f64().unwrap(),
            PX,
        );
        close(
            &format!("{name}: cy"),
            cy,
            rendered["cy"].as_f64().unwrap(),
            PX,
        );

        let ours = our_sectors(&html);
        let theirs = rendered["sectors"].as_array().unwrap();
        assert_eq!(ours.len(), theirs.len(), "{name}: sector count {ours:?}");
        let stacked = name == "chart-radial-stacked";
        for (k, ((inner, outer, start, sweep, bg), t)) in ours.iter().zip(theirs).enumerate() {
            let what = format!("{name}: sector {k}");
            assert_eq!(
                *bg,
                t["background"].as_bool().unwrap(),
                "{what}: background"
            );
            close(
                &format!("{what} inner"),
                *inner,
                t["inner"].as_f64().unwrap(),
                PX,
            );
            close(
                &format!("{what} outer"),
                *outer,
                t["outer"].as_f64().unwrap(),
                PX,
            );
            if stacked {
                continue; // see below
            }
            close_deg(
                &format!("{what} start"),
                *start,
                t["start"].as_f64().unwrap(),
            );
            close(
                &format!("{what} sweep"),
                *sweep,
                t["sweep"].as_f64().unwrap(),
                0.05,
            );
        }
        if stacked {
            // Deliberate difference: shares of the stack total (570 / 1830 of
            // the half turn for mobile), not Recharts' clamped 1260 domain;
            // still mobile first from three o'clock, filling exactly 180.
            close_deg("stacked start", ours[0].2, 0.0);
            close("stacked mobile", ours[0].3, 570.0 / 1830.0 * 180.0, 0.01);
            close("stacked total", ours[0].3 + ours[1].3, 180.0, 0.01);
        }
        // PolarGrid circles (radial grid / text / shape).
        let rings: Vec<f64> = rendered["rings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r.as_f64().unwrap())
            .collect();
        let mut our_rings: Vec<f64> = tags(&html, "data-slot=\"chart-grid-ring\"")
            .iter()
            .map(|t| num(t, "r"))
            .collect();
        for t in tags(&html, "data-slot=\"chart-radial-annulus\"") {
            our_rings.push(num(t, "data-outer-radius"));
            our_rings.push(num(t, "data-inner-radius"));
        }
        assert_eq!(our_rings, rings, "{name}: grid rings");
        assert_eq!(
            tags(&html, "data-slot=\"chart-grid-spoke\"").len() as u64,
            rendered["spokes"].as_u64().unwrap(),
            "{name}: spokes"
        );
    }
}

#[test]
fn pie_and_radial_labels_match_rechartss() {
    for (name, demo) in demos() {
        if name.starts_with("chart-radar") {
            continue;
        }
        let fx = fixture(name);
        let rendered = &fx["rendered"];
        let html = render(demo);
        let theirs: Vec<&Value> = rendered["texts"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|t| {
                matches!(t["kind"].as_str(), Some("label" | "radial-label"))
                    || (t["kind"] == "other"
                        && name.starts_with("chart-pie")
                        && fx["labelList"].as_array().is_some_and(|l| !l.is_empty()))
            })
            .collect();
        let ours = tags(&html, "data-slot=\"chart-arc-label\"");
        assert_eq!(ours.len(), theirs.len(), "{name}: label count");
        for (tag, t) in ours.iter().zip(&theirs) {
            let text = text_after(&html, tag);
            let want = t["text"].as_str().unwrap();
            assert!(
                text.eq_ignore_ascii_case(want),
                "{name}: label {text:?} vs {want:?}"
            );
            if t["kind"] == "radial-label" {
                continue; // a <textPath>: its path is checked by the engine tests
            }
            close(
                &format!("{name}: {want} x"),
                num(tag, "x"),
                t["x"].as_f64().unwrap(),
                PX,
            );
            close(
                &format!("{name}: {want} y"),
                num(tag, "y"),
                t["y"].as_f64().unwrap(),
                PX,
            );
            if let Some(anchor) = t["anchor"].as_str() {
                assert_eq!(
                    attr(tag, "text-anchor"),
                    Some(anchor),
                    "{name}: {want} anchor"
                );
            }
        }
        assert_eq!(
            tags(&html, "data-slot=\"chart-arc-label-line\"").len() as u64,
            rendered["labelLines"].as_u64().unwrap(),
            "{name}: label lines"
        );
        // The center text, as one string (`"1,125Visitors"`).
        let center = rendered["texts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["kind"] == "other" && t["x"] == 0 && t["y"] == 0);
        let ours = tags(&html, "data-slot=\"chart-pie-center-text\"");
        match center {
            Some(t) => {
                let text = text_after(&html, ours.first().expect("center text"));
                assert_eq!(text, t["text"].as_str().unwrap(), "{name}: center text");
            }
            None => assert!(ours.is_empty(), "{name}: no center text in shadcn"),
        }
    }
}

/// Distances of a closed `M x y L x y ... Z` path's vertices from the origin.
fn vertex_radii(d: &str) -> Vec<f64> {
    d.split(['M', 'L'])
        .filter_map(|p| {
            let mut it = p.trim().trim_end_matches('Z').split_whitespace();
            let x: f64 = it.next()?.parse().ok()?;
            let y: f64 = it.next()?.parse().ok()?;
            Some((x * x + y * y).sqrt())
        })
        .collect()
}

#[test]
fn radar_geometry_matches_rechartss() {
    for (name, demo) in demos() {
        if !name.starts_with("chart-radar") {
            continue;
        }
        let fx = fixture(name);
        let rendered = &fx["rendered"];
        let html = render(demo);
        let (cx, cy) = centre(&html);
        close(
            &format!("{name}: cx"),
            cx,
            rendered["cx"].as_f64().unwrap(),
            PX,
        );
        close(
            &format!("{name}: cy"),
            cy,
            rendered["cy"].as_f64().unwrap(),
            PX,
        );

        let rings: Vec<f64> = tags(&html, "data-slot=\"chart-grid-ring\"")
            .iter()
            .map(|t| {
                attr(t, "r")
                    .or(attr(t, "data-radius"))
                    .unwrap()
                    .parse()
                    .unwrap()
            })
            .collect();
        let theirs: Vec<f64> = rendered["rings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r.as_f64().unwrap())
            .collect();
        assert_eq!(rings.len(), theirs.len(), "{name}: rings {rings:?}");
        for (a, b) in rings.iter().zip(&theirs) {
            close(&format!("{name}: ring"), *a, *b, PX);
        }
        assert_eq!(
            tags(&html, "data-slot=\"chart-grid-spoke\"").len() as u64,
            rendered["spokes"].as_u64().unwrap(),
            "{name}: spokes"
        );

        let polygons = tags(&html, "data-slot=\"chart-radar-area\"");
        let theirs = rendered["polygons"].as_array().unwrap();
        assert_eq!(polygons.len(), theirs.len(), "{name}: polygons");
        for (tag, t) in polygons.iter().zip(theirs) {
            close(
                &format!("{name}: fill-opacity"),
                num(tag, "fill-opacity"),
                t["fillOpacity"].as_f64().unwrap(),
                1e-9,
            );
            assert_eq!(
                attr(tag, "data-lines-only").is_some(),
                t["strokeWidth"].as_f64() == Some(2.0),
                "{name}: outline"
            );
            let radii = vertex_radii(attr(tag, "d").unwrap());
            let want: Vec<f64> = t["radii"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| r.as_f64().unwrap())
                .collect();
            assert_eq!(radii.len(), want.len(), "{name}: vertices");
            for (a, b) in radii.iter().zip(&want) {
                close(&format!("{name}: vertex"), *a, *b, 0.01);
            }
        }
        let dots = tags(&html, "data-slot=\"chart-dot\"");
        let theirs = rendered["dots"].as_array().unwrap();
        assert_eq!(dots.len(), theirs.len(), "{name}: dots");
        for (tag, r) in dots.iter().zip(theirs) {
            assert_eq!(num(tag, "r"), r.as_f64().unwrap(), "{name}: dot r");
        }

        // Axis text: the angle-axis labels (or custom ticks) and the
        // radius-axis values, positions relative to the centre.
        for (axis, kinds) in [
            ("angle", ["angle", "other"]),
            ("radius", ["radius", "radius"]),
        ] {
            let ours: Vec<&str> = match tags(&html, &format!("data-axis=\"{axis}\"")).first() {
                Some(group) => {
                    let at = html.find(group).unwrap();
                    let end = html[at..].find("</g>").unwrap() + at;
                    open_tags(&html[at..end], "text")
                }
                None => Vec::new(),
            };
            let theirs: Vec<&Value> = rendered["texts"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|t| kinds.contains(&t["kind"].as_str().unwrap()))
                .collect();
            assert_eq!(ours.len(), theirs.len(), "{name}: {axis} axis labels");
            for t in theirs {
                let x = t["x"].as_f64().unwrap();
                let y = t["y"].as_f64().unwrap();
                let tag = ours
                    .iter()
                    .find(|tag| (num(tag, "x") - x).abs() <= PX && (num(tag, "y") - y).abs() <= PX)
                    .unwrap_or_else(|| panic!("{name}: no {axis} label at ({x}, {y}): {ours:?}"));
                assert_eq!(
                    attr(tag, "text-anchor"),
                    t["anchor"].as_str(),
                    "{name}: anchor"
                );
                let text: String = text_after(&html, tag);
                assert_eq!(
                    text,
                    t["text"].as_str().unwrap(),
                    "{name}: {axis} label text"
                );
            }
        }
    }
}
