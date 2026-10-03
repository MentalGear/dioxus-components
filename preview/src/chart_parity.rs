//! Parity of the area, line and bar gallery demos with shadcn/ui's own
//! `chart-{area,line,bar}-*` registry demos (`registry/new-york-v4/charts`).
//!
//! `preview/tests/shadcn/{area,line,bar}-<variant>.json` are derived from the
//! registry by `scripts/extract-shadcn-fixtures.mjs` (never hand-typed). For
//! every demo this checks, against its fixture:
//! - the **data**: every row's category and every series' value (the demo's
//!   `chart_data()`), and the per-datum colors (`fill` / `<Cell>`);
//! - the **series**: keys, labels and colors in JSX order (`chart_config()`),
//!   which is Recharts' draw and stacking order;
//! - the **props** that change the drawing, from the demo's source: curve,
//!   stacking and expand, margin, orientation, grid/axes, tick count, tick
//!   margin and min gap, hover cursor, bar radius, fill opacities, gradient,
//!   dots and labels, active bar, legend, and the tooltip's indicator,
//!   label/name and label formatter -- and the card title.
//!
//! Deliberate differences (not checked): the palette (our `--dx-chart-N`
//! tokens stand in for shadcn's `--chart-N`), and our extra `bar_chart`
//! `stacked` variant, which is checked against shadcn's stacked chart minus
//! its legend.
use crate::components::chart::{ChartConfig, ChartDatum};
use dioxus::prelude::*;
use serde_json::Value;

struct Demo {
    fixture: &'static str,
    data: fn() -> Vec<ChartDatum>,
    config: fn() -> ChartConfig,
    source: &'static str,
    /// Our `stacked` bar variant is shadcn's stacked chart without the legend.
    skip_legend: bool,
}

macro_rules! demo {
    ($kind:ident, $variant:ident, $fixture:literal) => {
        demo!($kind, $variant, $fixture, false)
    };
    ($kind:ident, $variant:ident, $fixture:literal, $skip_legend:expr) => {
        Demo {
            fixture: $fixture,
            data: crate::components::$kind::variants::$variant::chart_data,
            config: crate::components::$kind::variants::$variant::chart_config,
            source: include_str!(concat!(
                "components/",
                stringify!($kind),
                "/variants/",
                stringify!($variant),
                "/mod.rs"
            )),
            skip_legend: $skip_legend,
        }
    };
}

fn demos() -> Vec<Demo> {
    vec![
        demo!(area_chart, main, "area-main"),
        demo!(area_chart, linear, "area-linear"),
        demo!(area_chart, step, "area-step"),
        demo!(area_chart, stacked, "area-stacked"),
        demo!(area_chart, stacked_expand, "area-stacked_expand"),
        demo!(area_chart, gradient, "area-gradient"),
        demo!(area_chart, legend, "area-legend"),
        demo!(area_chart, axes, "area-axes"),
        demo!(area_chart, icons, "area-icons"),
        demo!(area_chart, interactive, "area-interactive"),
        demo!(line_chart, main, "line-main"),
        demo!(line_chart, linear, "line-linear"),
        demo!(line_chart, step, "line-step"),
        demo!(line_chart, multiple, "line-multiple"),
        demo!(line_chart, dots, "line-dots"),
        demo!(line_chart, dots_custom, "line-dots_custom"),
        demo!(line_chart, dots_colors, "line-dots_colors"),
        demo!(line_chart, label, "line-label"),
        demo!(line_chart, label_custom, "line-label_custom"),
        demo!(line_chart, interactive, "line-interactive"),
        demo!(bar_chart, main, "bar-main"),
        demo!(bar_chart, horizontal, "bar-horizontal"),
        demo!(bar_chart, multiple, "bar-multiple"),
        demo!(bar_chart, stacked, "bar-stacked_legend", true),
        demo!(bar_chart, stacked_legend, "bar-stacked_legend"),
        demo!(bar_chart, label, "bar-label"),
        demo!(bar_chart, label_custom, "bar-label_custom"),
        demo!(bar_chart, mixed, "bar-mixed"),
        demo!(bar_chart, active, "bar-active"),
        demo!(bar_chart, negative, "bar-negative"),
        demo!(bar_chart, interactive, "bar-interactive"),
    ]
}

fn fixture(name: &str) -> Value {
    let path = format!("{}/tests/shadcn/{name}.json", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// shadcn's `var(--chart-N)` is our `var(--dx-chart-N)`.
fn our_color(shadcn: &str) -> String {
    shadcn.replace("var(--chart-", "var(--dx-chart-")
}

/// `var(--color-chrome)` -> the config's color for `chrome`, as ours.
fn resolve_color(fx: &Value, color: &str) -> String {
    match color
        .strip_prefix("var(--color-")
        .and_then(|k| k.strip_suffix(')'))
    {
        Some(key) => our_color(fx["config"][key]["color"].as_str().unwrap_or_default()),
        None => our_color(color),
    }
}

/// The series keys in JSX order; an interactive chart's single `<Line
/// dataKey={activeChart}>` stands for every colored config entry.
fn series_keys(fx: &Value) -> Vec<String> {
    let series = fx["series"].as_array().unwrap();
    if series.iter().any(|s| s["dataKey"] == "<expr>") {
        fx["config"]
            .as_object()
            .unwrap()
            .iter()
            .filter(|(_, v)| v.get("color").is_some())
            .map(|(k, _)| k.clone())
            .collect()
    } else {
        series
            .iter()
            .map(|s| s["dataKey"].as_str().unwrap().to_string())
            .collect()
    }
}

/// The row's category: its first string field other than `fill`.
fn category(row: &Value) -> String {
    row.as_object()
        .unwrap()
        .iter()
        .find(|(k, v)| *k != "fill" && v.is_string())
        .map(|(_, v)| v.as_str().unwrap().to_string())
        .expect("a category field")
}

fn check_series_and_data(fx: &Value, d: &Demo) {
    let name = fx["name"].as_str().unwrap();
    let keys = series_keys(fx);
    let config = (d.config)();
    let ours: Vec<&str> = config.series.iter().map(|s| s.key.as_str()).collect();
    assert_eq!(ours, keys, "{name}: series keys in JSX (draw/stack) order");
    for s in &config.series {
        let theirs = &fx["config"][&s.key];
        assert_eq!(
            s.label,
            theirs["label"].as_str().unwrap(),
            "{name}: {} label",
            s.key
        );
        // A series without a config color (bars colored per datum) may use
        // any color of ours.
        if let Some(color) = theirs["color"].as_str() {
            assert_eq!(s.color, our_color(color), "{name}: {} color", s.key);
        }
        assert_eq!(
            s.icon.is_some(),
            theirs.get("icon").is_some(),
            "{name}: {} icon",
            s.key
        );
    }

    let rows = fx["data"].as_array().unwrap();
    let data = (d.data)();
    assert_eq!(data.len(), rows.len(), "{name}: row count");
    for (i, (ours, row)) in data.iter().zip(rows).enumerate() {
        let cat = category(row);
        let label_ok =
            ours.label == cat || fx["config"][&cat]["label"].as_str() == Some(ours.label.as_str());
        assert!(label_ok, "{name} row {i}: label {} vs {cat}", ours.label);
        for (s, key) in keys.iter().enumerate() {
            assert_eq!(
                ours.values.get(s).copied().flatten(),
                row[key].as_f64(),
                "{name} row {i}: {key}"
            );
        }
        let want = if let Some(fill) = row["fill"].as_str() {
            Some(resolve_color(fx, fill))
        } else if fx["cells"] == true {
            // chart-bar-negative: `fill={item.visitors > 0 ? chart-1 : chart-2}`.
            let v = row[&keys[0]].as_f64().unwrap();
            Some(our_color(if v > 0.0 {
                "var(--chart-1)"
            } else {
                "var(--chart-2)"
            }))
        } else {
            None
        };
        assert_eq!(ours.color, want, "{name} row {i}: per-datum color");
    }
}

/// The source's `margin: ChartMargin { .. }` sides (`ChartMargin::NONE`'s
/// zeros for the sides it omits), or `None` without a margin prop.
fn source_margin(src: &str) -> Option<[f64; 4]> {
    let at = src.find("margin: ChartMargin")?;
    let rest = &src[at..];
    let text = &rest[..rest.find('\n').unwrap_or(rest.len())];
    if text.contains("ChartMargin::NONE") && !text.contains('{') {
        return Some([0.0; 4]);
    }
    let side = |s: &str| {
        let key = format!("{s}: ");
        text.find(&key)
            .map(|p| {
                let v = &text[p + key.len()..];
                v[..v.find([',', ' ']).unwrap()].parse::<f64>().unwrap()
            })
            .unwrap_or(0.0)
    };
    Some([side("top"), side("right"), side("bottom"), side("left")])
}

fn check_props(fx: &Value, d: &Demo) {
    let name = fx["name"].as_str().unwrap();
    let src = d.source;
    let has = |s: &str| src.contains(s);
    let series = fx["series"].as_array().unwrap();
    let first = &series[0];
    let kind = fx["chart"]["kind"].as_str().unwrap();
    let horizontal = fx["chart"]["layout"] == "vertical";

    // Card title (our extra `stacked` bar variant drops "+ Legend").
    let title = fx["title"].as_str().unwrap();
    let title = if d.skip_legend {
        title.trim_end_matches(" + Legend").to_string()
    } else {
        title.to_string()
    };
    assert!(
        has(&format!("CardTitle {{ \"{title}\" }}")),
        "{name}: title {title}"
    );

    // Curve (Recharts `type`).
    if let Some(curve) = first["type"].as_str() {
        let ours = match curve {
            "natural" => "Curve::Natural",
            "monotone" => "Curve::Monotone",
            "linear" => "Curve::Linear",
            "step" => "Curve::Step",
            other => panic!("{name}: unmapped curve {other}"),
        };
        assert!(has(&format!("curve: {ours}")), "{name}: {ours}");
    }

    // Stacking.
    let stacked = series.iter().any(|s| !s["stackId"].is_null());
    assert_eq!(has("stacked: true"), stacked, "{name}: stacked");
    assert_eq!(
        has("StackMode::Expand"),
        fx["chart"]["stackOffset"] == "expand",
        "{name}: expand"
    );

    // Margin (a Recharts margin replaces the default 5px on every side).
    let margin = fx["chart"]["margin"].as_object().map(|m| {
        let side = |s: &str| m.get(s).and_then(Value::as_f64).unwrap_or(0.0);
        [side("top"), side("right"), side("bottom"), side("left")]
    });
    assert_eq!(source_margin(src), margin, "{name}: margin");

    // Orientation, grid and axes.
    assert_eq!(has("horizontal: true"), horizontal, "{name}: layout");
    assert_eq!(
        has("show_grid: false"),
        fx["grid"].is_null(),
        "{name}: grid"
    );
    let shown = |axis: &str| !fx[axis].is_null() && fx[axis]["hide"] != true;
    assert_eq!(has("show_x_axis: false"), !shown("xAxis"), "{name}: x axis");
    assert_eq!(has("show_y_axis: true"), shown("yAxis"), "{name}: y axis");
    let label_axis = if horizontal { "yAxis" } else { "xAxis" };
    let value_axis = if horizontal { "xAxis" } else { "yAxis" };
    if let Some(count) = fx[value_axis]["tickCount"].as_u64() {
        assert!(has(&format!("y_tick_count: {count}")), "{name}: tickCount");
    }
    match fx[label_axis]["tickMargin"].as_f64() {
        Some(m) if shown(label_axis) && m != 8.0 => {
            assert!(
                has(&format!("tick_margin: {m:.1}")),
                "{name}: tickMargin {m}"
            )
        }
        _ => assert!(!has("tick_margin:"), "{name}: default tickMargin"),
    }
    match fx[label_axis]["minTickGap"].as_f64() {
        Some(g) => assert!(has(&format!("min_tick_gap: {g:.1}")), "{name}: minTickGap"),
        None => assert!(!has("min_tick_gap:"), "{name}: default minTickGap"),
    }

    // Hover cursor, legend, gradient.
    assert_eq!(
        has("cursor: false"),
        fx["tooltip"]["cursor"] == false,
        "{name}: cursor"
    );
    if !d.skip_legend {
        assert_eq!(
            has("ChartLegend {}"),
            fx["legend"] == true,
            "{name}: legend"
        );
    }
    assert_eq!(
        has("gradient: true"),
        fx["gradient"] == true,
        "{name}: gradient"
    );

    // Tooltip content.
    let tip = &fx["tooltip"];
    let indicator = match tip["indicator"].as_str().unwrap() {
        "line" => Some("TooltipIndicator::Line"),
        "dashed" => Some("TooltipIndicator::Dashed"),
        _ => None,
    };
    assert_eq!(
        has("indicator: TooltipIndicator::"),
        indicator.is_some(),
        "{name}: indicator"
    );
    if let Some(ind) = indicator {
        assert!(has(&format!("indicator: {ind}")), "{name}: {ind}");
    }
    assert_eq!(
        has("hide_label: true"),
        tip["hideLabel"] == true,
        "{name}: hideLabel"
    );
    assert_eq!(
        has("hide_indicator: true"),
        tip["hideIndicator"] == true,
        "{name}: hideIndicator"
    );
    assert_eq!(
        has("name_key:"),
        !tip["nameKey"].is_null(),
        "{name}: nameKey"
    );
    assert_eq!(
        has("label_format:"),
        tip["labelFormatter"] == true,
        "{name}: labelFormatter"
    );

    match kind {
        "area" => {
            // Recharts' `<Area>` default fill opacity is 0.6; our default 0.4.
            let opacities: Vec<f64> = series
                .iter()
                .map(|s| s["fillOpacity"].as_f64().unwrap_or(0.6))
                .collect();
            if opacities.iter().all(|o| *o == opacities[0]) {
                if opacities[0] == 0.4 {
                    assert!(!has("fill_opacity"), "{name}: default fill opacity");
                } else {
                    assert!(
                        has(&format!("fill_opacity: {}", opacities[0])),
                        "{name}: fill opacity {}",
                        opacities[0]
                    );
                }
            } else {
                let list: Vec<String> = opacities.iter().map(|o| format!("{o}")).collect();
                assert!(
                    has(&format!("series_fill_opacity: vec![{}]", list.join(", "))),
                    "{name}: per-series fill opacity {list:?}"
                );
            }
        }
        "line" => {
            let dots = !first["dot"].is_null() && first["dot"] != false;
            assert_eq!(has("dots: true") || has("dot: Some("), dots, "{name}: dots");
            let labels = fx["labelLists"].as_array().unwrap();
            match labels.first() {
                None => assert!(!has("labels:"), "{name}: no labels"),
                Some(l) if l["dataKey"].is_null() => {
                    assert!(has("labels: LineLabels::Value"), "{name}: value labels")
                }
                Some(_) => assert!(has("labels: LineLabels::Custom"), "{name}: custom labels"),
            }
        }
        _ => {
            // Bar radius, per series when they differ.
            let radii: Vec<&Value> = series.iter().map(|s| &s["radius"]).collect();
            if radii.iter().any(|r| r.is_array()) {
                for r in &radii {
                    let c: Vec<String> = r
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| format!("{:.1}", v.as_f64().unwrap()))
                        .collect();
                    assert!(
                        has(&format!("BarRadius::corners({})", c.join(", "))),
                        "{name}: radius {c:?}"
                    );
                }
            } else if let Some(r) = radii[0].as_f64() {
                assert!(
                    has(&format!("radius: BarRadius::all({r:.1})")),
                    "{name}: radius {r}"
                );
            } else {
                assert!(!has("radius:"), "{name}: square bars");
            }
            let labels = fx["labelLists"].as_array().unwrap();
            let position = |p: &str| labels.iter().any(|l| l["position"] == p);
            assert_eq!(
                has("inside_labels: true"),
                position("insideLeft"),
                "{name}: inside"
            );
            let top_category = labels
                .iter()
                .any(|l| l["position"] == "top" && !l["dataKey"].is_null());
            assert_eq!(
                has("category_labels: true"),
                top_category,
                "{name}: category"
            );
            let top_value = labels
                .iter()
                .any(|l| l["position"] == "top" && l["dataKey"].is_null());
            assert_eq!(has("value_labels: true"), top_value, "{name}: values");
            match fx["activeIndex"].as_u64() {
                Some(i) => assert!(has(&format!("active_index: Some({i})")), "{name}: active"),
                None => assert!(!has("active_index"), "{name}: no active bar"),
            }
        }
    }
}

#[test]
fn every_demo_matches_its_shadcn_source() {
    // A config with icons builds `Callback`s, which need a Dioxus scope.
    let mut dom = VirtualDom::new(|| rsx! {});
    dom.rebuild_in_place();
    dom.in_scope(ScopeId::ROOT, || {
        for d in demos() {
            let fx = fixture(d.fixture);
            check_series_and_data(&fx, &d);
            check_props(&fx, &d);
        }
    });
}

#[test]
fn every_shadcn_area_line_bar_chart_has_a_demo() {
    let dir = format!("{}/tests/shadcn", env!("CARGO_MANIFEST_DIR"));
    let mut fixtures: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|f| f.starts_with("area-") || f.starts_with("line-") || f.starts_with("bar-"))
        .map(|f| f.trim_end_matches(".json").to_string())
        .collect();
    fixtures.sort();
    let mut ours: Vec<String> = demos().iter().map(|d| d.fixture.to_string()).collect();
    ours.sort();
    ours.dedup();
    assert_eq!(ours, fixtures);
}

#[test]
fn the_interactive_charts_share_shadcns_dataset_and_totals() {
    // 91 days, desktop 24,828 / mobile 25,010 (shadcn's header totals).
    for (data, desktop, mobile) in [
        (
            crate::components::line_chart::variants::interactive::chart_data(),
            0,
            1,
        ),
        (
            crate::components::bar_chart::variants::interactive::chart_data(),
            0,
            1,
        ),
        (
            crate::components::area_chart::variants::interactive::chart_data(),
            1,
            0,
        ),
    ] {
        let total = |s: usize| -> f64 { data.iter().filter_map(|d| d.values[s]).sum() };
        assert_eq!(data.len(), 91);
        assert_eq!((total(desktop), total(mobile)), (24_828.0, 25_010.0));
    }
}

#[test]
fn the_area_range_select_keeps_shadcns_91_31_8_days() {
    use crate::components::area_chart::variants::interactive::{visible_data, TimeRange};
    for (range, rows, first) in [
        (TimeRange::Days90, 91, "2024-04-01"),
        (TimeRange::Days30, 31, "2024-05-31"),
        (TimeRange::Days7, 8, "2024-06-23"),
    ] {
        let data = visible_data(range);
        assert_eq!(data.len(), rows, "{range}");
        assert_eq!(data[0].label, first, "{range}");
        assert_eq!(data.last().unwrap().label, "2024-06-30", "{range}");
    }
}

// -- Geometry: what our demos draw vs what shadcn's pages render -----------
//
// `preview/tests/shadcn/geometry.json` holds the SVG geometry of shadcn's own
// charts at its 369px card (DOM probes of ui.shadcn.com, see
// `scripts/extract-shadcn-fixtures.mjs`). Our server render IS that size --
// the initial width before measuring is shadcn's 369px card, and one svg
// unit is one CSS px -- so every gridline, curve control point, bar and dot
// must land on shadcn's coordinates. The three charts with a legend are in
// too: Recharts draws its legend inside the chart box and shrinks the plot by
// the legend's height (28px, one row); ours takes that room out of the same
// box -- the svg is the box minus the legend, the legend right below it -- so
// the plot coordinates are shadcn's and only the svg's own height differs.

/// A gallery demo's component.
type DemoFn = fn() -> Element;

/// The legend's room inside the chart box (`DEFAULT_LEGEND_SIZE`, one row).
const LEGEND: f64 = 28.0;

/// The legend's share of the box for a probed chart that has one.
fn legend_reserve(name: &str) -> f64 {
    match name {
        "chart-area-legend" | "chart-area-icons" | "chart-bar-stacked" => LEGEND,
        _ => 0.0,
    }
}

/// (shadcn chart, our demo) for every probed 369px chart.
fn geometry_demos() -> Vec<(&'static str, DemoFn)> {
    use crate::components::{area_chart as a, bar_chart as b, line_chart as l};
    vec![
        ("chart-area-default", a::variants::main::Demo),
        ("chart-area-linear", a::variants::linear::Demo),
        ("chart-area-step", a::variants::step::Demo),
        ("chart-area-stacked", a::variants::stacked::Demo),
        (
            "chart-area-stacked-expand",
            a::variants::stacked_expand::Demo,
        ),
        ("chart-area-legend", a::variants::legend::Demo),
        ("chart-area-icons", a::variants::icons::Demo),
        ("chart-area-gradient", a::variants::gradient::Demo),
        ("chart-area-axes", a::variants::axes::Demo),
        ("chart-line-default", l::variants::main::Demo),
        ("chart-line-linear", l::variants::linear::Demo),
        ("chart-line-step", l::variants::step::Demo),
        ("chart-line-multiple", l::variants::multiple::Demo),
        ("chart-line-dots", l::variants::dots::Demo),
        ("chart-line-dots-custom", l::variants::dots_custom::Demo),
        ("chart-line-dots-colors", l::variants::dots_colors::Demo),
        ("chart-line-label", l::variants::label::Demo),
        ("chart-line-label-custom", l::variants::label_custom::Demo),
        ("chart-bar-default", b::variants::main::Demo),
        ("chart-bar-horizontal", b::variants::horizontal::Demo),
        ("chart-bar-multiple", b::variants::multiple::Demo),
        ("chart-bar-stacked", b::variants::stacked_legend::Demo),
        ("chart-bar-label", b::variants::label::Demo),
        ("chart-bar-label-custom", b::variants::label_custom::Demo),
        ("chart-bar-mixed", b::variants::mixed::Demo),
        ("chart-bar-active", b::variants::active::Demo),
        ("chart-bar-negative", b::variants::negative::Demo),
    ]
}

/// The opening tag of every element carrying `data-slot="<slot>"`.
fn tags<'a>(html: &'a str, slot: &str) -> Vec<&'a str> {
    let key = format!(r#"data-slot="{slot}""#);
    html.match_indices(&key)
        .map(|(at, _)| {
            let start = html[..at].rfind('<').unwrap();
            let end = at + html[at..].find('>').unwrap();
            &html[start..end]
        })
        .collect()
}

fn attr(tag: &str, name: &str) -> Option<String> {
    let key = format!(r#" {name}=""#);
    let at = tag.find(&key)? + key.len();
    Some(tag[at..at + tag[at..].find('"')?].to_string())
}

fn num(tag: &str, name: &str) -> f64 {
    attr(tag, name)
        .unwrap_or_else(|| panic!("no {name} in {tag}"))
        .parse()
        .unwrap()
}

/// Every number in an SVG path (or any) string.
fn numbers(d: &str) -> Vec<f64> {
    d.split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
        .filter(|t| !t.is_empty() && *t != "-")
        .map(|t| t.parse().unwrap())
        .collect()
}

fn close(ours: &[f64], theirs: &[f64], tol: f64) -> bool {
    ours.len() == theirs.len() && ours.iter().zip(theirs).all(|(a, b)| (a - b).abs() <= tol)
}

#[test]
fn the_demos_draw_shadcns_geometry() {
    // Ours round to 3 decimals, shadcn's DOM to 2-4.
    const TOL: f64 = 0.006;
    let geometry = fixture("geometry");
    for (name, demo) in geometry_demos() {
        let g = &geometry[name];
        let mut dom = VirtualDom::new(demo);
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);

        let svg = tags(&html, "chart-svg")[0];
        let size = numbers(&attr(svg, "viewBox").unwrap());
        let want = [
            g["svg"][0].as_f64().unwrap(),
            g["svg"][1].as_f64().unwrap() - legend_reserve(name),
        ];
        assert!(
            close(&size[2..], &want, TOL),
            "{name}: svg {size:?} vs {want:?}"
        );

        // Gridlines, as sets.
        let grid_group = &html[html.find(r#"data-slot="chart-grid""#).map_or(0, |i| i)..];
        let mut ours: Vec<Vec<f64>> = if g["grid"].as_array().unwrap().is_empty() {
            Vec::new()
        } else {
            grid_group[..grid_group.find("</g>").unwrap()]
                .split("<line")
                .skip(1)
                .map(|t| ["x1", "y1", "x2", "y2"].map(|a| num(t, a)).to_vec())
                .collect()
        };
        let mut theirs: Vec<Vec<f64>> = g["grid"]
            .as_array()
            .unwrap()
            .iter()
            .map(|l| {
                l.as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_f64().unwrap())
                    .collect()
            })
            .collect();
        let key = |v: &Vec<f64>| (v[0] * 1000.0) as i64 * 1_000_000 + (v[1] * 1000.0) as i64;
        ours.sort_by_key(key);
        theirs.sort_by_key(key);
        assert_eq!(ours.len(), theirs.len(), "{name}: gridline count");
        for (o, t) in ours.iter().zip(&theirs) {
            assert!(close(o, t, TOL), "{name}: gridline {o:?} vs {t:?}");
        }

        // Line and area top-edge curves, control points included.
        let ours: Vec<String> = tags(&html, "chart-line")
            .iter()
            .map(|t| attr(t, "d").unwrap())
            .collect();
        let theirs: Vec<&str> = g["curves"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| !c["cls"].as_str().unwrap().contains("area-area"))
            .map(|c| c["d"].as_str().unwrap())
            .collect();
        assert_eq!(ours.len(), theirs.len(), "{name}: curve count");
        for (o, t) in ours.iter().zip(&theirs) {
            assert!(
                close(&numbers(o), &numbers(t), TOL),
                "{name}:\n ours {o}\n shadcn {t}"
            );
        }

        // Bars: the normalized rect (a path's own numbers for per-corner radii).
        let ours = tags(&html, "chart-bar");
        let theirs = g["bars"].as_array().unwrap();
        assert_eq!(ours.len(), theirs.len(), "{name}: bar count");
        for (o, t) in ours.iter().zip(theirs) {
            let (x, y, w, h) = (
                t["x"].as_f64().unwrap(),
                t["y"].as_f64().unwrap(),
                t["w"].as_f64().unwrap(),
                t["h"].as_f64().unwrap(),
            );
            let want = [x.min(x + w), y.min(y + h), w.abs(), h.abs()];
            if o.starts_with("<rect") {
                let got = ["x", "y", "width", "height"].map(|a| num(o, a));
                assert!(close(&got, &want, TOL), "{name}: bar {got:?} vs {want:?}");
            } else {
                let d = attr(o, "d").unwrap();
                let theirs_d = t["d"].as_str().unwrap();
                assert!(
                    close(&numbers(&d), &numbers(theirs_d), TOL),
                    "{name}:\n ours {d}\n shadcn {theirs_d}"
                );
            }
        }

        // Dots (the default circles). `dots-custom`'s probe "dots" are the
        // circle inside each lucide icon (its own 24x24 viewBox): count those.
        if name == "chart-line-dots-custom" {
            let icons = tags(&html, "chart-custom-dot").len();
            assert_eq!(
                icons,
                g["dots"].as_array().unwrap().len(),
                "{name}: icon dots"
            );
            continue;
        }
        let ours: Vec<[f64; 3]> = tags(&html, "chart-dot")
            .iter()
            .map(|t| ["cx", "cy", "r"].map(|a| num(t, a)))
            .collect();
        let theirs: Vec<[f64; 3]> = g["dots"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| [0, 1, 2].map(|i| d[i].as_f64().unwrap()))
            .collect();
        assert_eq!(ours.len(), theirs.len(), "{name}: dot count");
        for (o, t) in ours.iter().zip(&theirs) {
            assert!(close(o, t, 0.01), "{name}: dot {o:?} vs {t:?}");
        }
    }
}
