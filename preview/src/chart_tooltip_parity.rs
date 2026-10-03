//! Parity of the `chart_tooltip` gallery's nine demos with shadcn/ui's `chart-tooltip-*` registry
//! sources (`registry/new-york-v4/charts/chart-tooltip-*.tsx`).
//!
//! `preview/tests/shadcn/chart-tooltip-*.json` are derived from the registry by
//! `scripts/extract-shadcn-tooltip-fixtures.mjs` (never hand-typed). Each test server-renders one
//! of our demos and compares what it shows with its fixture: card title and description, the
//! dataset and series (via the chart's hidden data table), colors, which axes and grid are drawn,
//! the x tick labels, the tooltip's initial index, and the tooltip's own props (indicator, label,
//! label formatter, `formatter` rows, icons, total row, width), the per-series bar radius
//! (`[0, 0, 4, 4]` / `[4, 4, 0, 0]`) and `cursor={false}`.
//!
//! The `labelKey` demo differs by construction: shadcn's `labelKey="activities"` looks up a config entry, ours takes
//! the resulting text ("Activities") directly (`ChartTooltipProps::label_key`).
use crate::components::chart_tooltip::variants::*;
use dioxus::prelude::*;
use serde_json::Value;

type Demo = fn() -> Element;

/// (fixture name, our demo) for each of shadcn's nine tooltip charts.
fn demos() -> Vec<(&'static str, Demo)> {
    vec![
        ("chart-tooltip-default", main::Demo),
        ("chart-tooltip-indicator-line", indicator_line::Demo),
        ("chart-tooltip-indicator-none", indicator_none::Demo),
        ("chart-tooltip-label-none", label_none::Demo),
        ("chart-tooltip-label-custom", label_custom::Demo),
        ("chart-tooltip-label-formatter", label_formatter::Demo),
        ("chart-tooltip-formatter", formatter::Demo),
        ("chart-tooltip-icons", icons::Demo),
        ("chart-tooltip-advanced", advanced::Demo),
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

/// `html` with every tag removed.
fn strip_tags(html: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

/// The text of the first element carrying `data-slot="{slot}"`, nested elements included.
fn slot_text(html: &str, slot: &str) -> Option<String> {
    let marker = format!(r#"data-slot="{slot}""#);
    let after = &html[html.find(&marker)? + marker.len()..];
    let after = &after[after.find('>')? + 1..];
    // Walk to this element's own closing tag, counting nested openers of the same tag name.
    let tag_end = html[..html.find(&marker)?].rfind('<')? + 1;
    let tag: String = html[tag_end..]
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric())
        .collect();
    let (open, close) = (format!("<{tag}"), format!("</{tag}>"));
    let mut depth = 1;
    let mut at = 0;
    while depth > 0 {
        let next_close = after[at..].find(&close)? + at;
        match after[at..].find(&open).map(|i| i + at) {
            Some(next_open) if next_open < next_close => {
                depth += 1;
                at = next_open + open.len();
            }
            _ => {
                depth -= 1;
                at = next_close + close.len();
                if depth == 0 {
                    return Some(strip_tags(&after[..next_close]));
                }
            }
        }
    }
    None
}

fn count(html: &str, needle: &str) -> usize {
    html.matches(needle).count()
}

fn num(value: &Value) -> f64 {
    value.as_f64().expect("a number")
}

/// How our tooltip prints a JSON number (`380`, `305.5`).
fn shown(value: f64) -> String {
    format!("{value}")
}

fn series_keys(fixture: &Value) -> Vec<String> {
    fixture["config"]
        .as_object()
        .unwrap()
        .iter()
        .filter(|(_, v)| v.get("color").is_some())
        .map(|(k, _)| k.clone())
        .collect()
}

/// shadcn's `new Date(value).toLocaleDateString("en-US", { weekday: "short" })`.
fn weekday(iso: &str) -> String {
    let mut parts = iso.split('-').map(|p| p.parse::<u32>().unwrap());
    let (y, m, d) = (
        parts.next().unwrap(),
        parts.next().unwrap(),
        parts.next().unwrap(),
    );
    let date =
        time::Date::from_calendar_date(y as i32, time::Month::try_from(m as u8).unwrap(), d as u8)
            .unwrap();
    date.weekday().to_string().chars().take(3).collect()
}

/// "July 16, 2024": `toLocaleDateString("en-US", { day: "numeric", month: "long", year: "numeric" })`.
fn long_date(iso: &str) -> String {
    let mut parts = iso.split('-').map(|p| p.parse::<u32>().unwrap());
    let (y, m, d) = (
        parts.next().unwrap(),
        parts.next().unwrap(),
        parts.next().unwrap(),
    );
    let month = time::Month::try_from(m as u8).unwrap();
    format!("{month} {d}, {y}")
}

#[test]
fn every_registry_tooltip_chart_has_a_demo_and_vice_versa() {
    let dir = format!("{}/tests/shadcn", env!("CARGO_MANIFEST_DIR"));
    let mut on_disk: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|f| f.starts_with("chart-tooltip-") && f.ends_with(".json"))
        .map(|f| f.trim_end_matches(".json").to_string())
        .collect();
    on_disk.sort();
    let mut ours: Vec<String> = demos().into_iter().map(|(n, _)| n.to_string()).collect();
    ours.sort();
    assert_eq!(on_disk, ours);
}

#[test]
fn card_text_and_dataset_match_the_registry() {
    for (name, demo) in demos() {
        let fx = fixture(name);
        let html = render(demo);
        assert_eq!(
            slot_text(&html, "card-title").as_deref(),
            fx["title"].as_str(),
            "{name}: title"
        );
        assert_eq!(
            slot_text(&html, "card-description").as_deref(),
            fx["description"].as_str(),
            "{name}: description"
        );

        // The hidden data table carries every row of the dataset and the series' labels.
        let keys = series_keys(&fx);
        let table = &html[html.find("<table").expect("data table")..];
        let table = &table[..table.find("</table>").unwrap()];
        let (head, body) = table.split_once("<tbody>").unwrap();
        let head = head.split_once("</caption>").map_or(head, |(_, rest)| rest);
        let headers: Vec<String> = head
            .split("</th>")
            .filter(|h| h.contains("<th"))
            .map(strip_tags)
            .collect();
        let mut expected_headers = vec!["Date".to_string()];
        expected_headers.extend(
            keys.iter()
                .map(|k| fx["config"][k]["label"].as_str().unwrap().to_string()),
        );
        assert_eq!(headers, expected_headers, "{name}: table headers in {head}");

        let rows: Vec<&str> = body.split("<tr>").skip(1).collect();
        let data = fx["data"].as_array().unwrap();
        assert_eq!(rows.len(), data.len(), "{name}: row count");
        for (row, datum) in rows.iter().zip(data) {
            let date = datum[fx["xAxis"]["dataKey"].as_str().unwrap()]
                .as_str()
                .unwrap();
            assert!(
                row.contains(&format!(">{date}</th>")),
                "{name}: {date} in {row}"
            );
            for key in &keys {
                let value = shown(num(&datum[key]));
                assert!(
                    row.contains(&format!("<td>{value}</td>")),
                    "{name}: {date} {key} = {value} in {row}"
                );
            }
        }
    }
}

#[test]
fn series_colors_come_from_the_registry_config() {
    for (name, demo) in demos() {
        let fx = fixture(name);
        let html = render(demo);
        for key in series_keys(&fx) {
            // shadcn's `var(--chart-N)` is this site's `var(--dx-chart-N)`.
            let color = fx["config"][&key]["color"]
                .as_str()
                .unwrap()
                .replace("--chart-", "--dx-chart-");
            assert!(
                html.contains(&format!("--color-{key}:{color}")),
                "{name}: {key} {color}"
            );
        }
        // One stacked bar series per `<Bar>`, in the registry's order.
        let bars = fx["bars"].as_array().unwrap();
        let series: Vec<&str> = html
            .split(r#"data-slot="chart-series" data-series=""#)
            .skip(1)
            .map(|s| &s[..s.find('"').unwrap()])
            .collect();
        let expected: Vec<&str> = bars
            .iter()
            .map(|b| b["dataKey"].as_str().unwrap())
            .collect();
        assert_eq!(series, expected, "{name}: series");
        assert!(
            bars.iter().all(|b| b["stackId"] == bars[0]["stackId"]),
            "{name}: one stack"
        );
        assert_eq!(
            count(&html, r#"data-slot="chart-bar""#),
            bars.len() * fx["data"].as_array().unwrap().len(),
            "{name}: bars"
        );
    }
}

#[test]
fn axes_and_grid_match_the_registry() {
    for (name, demo) in demos() {
        let fx = fixture(name);
        let html = render(demo);
        // The registry draws no `CartesianGrid` and no `YAxis`.
        assert_eq!(fx["hasGrid"], false, "{name}: fixture");
        assert_eq!(fx["hasYAxis"], false, "{name}: fixture");
        assert_eq!(
            count(&html, r#"data-slot="chart-grid""#),
            0,
            "{name}: no grid"
        );
        assert_eq!(count(&html, r#"data-axis="y""#), 0, "{name}: no y axis");

        // The x axis shows each date's short weekday.
        assert_eq!(fx["xAxis"]["weekdayTicks"], true, "{name}: fixture");
        let axis = &html[html.find(r#"data-axis="x""#).expect("x axis")..];
        let axis = &axis[axis.find('>').unwrap()..];
        let axis = &axis[..axis.find("</g>").unwrap()];
        let ticks: Vec<String> = axis
            .split("</text>")
            .filter(|t| t.contains("<text"))
            .map(strip_tags)
            .collect();
        let expected: Vec<String> = fx["data"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| weekday(d["date"].as_str().unwrap()))
            .collect();
        assert_eq!(ticks, expected, "{name}: x ticks");
    }
}

#[test]
fn tooltip_opens_on_the_registry_default_index_with_its_props() {
    for (name, demo) in demos() {
        let fx = fixture(name);
        let html = render(demo);
        let index = fx["defaultIndex"].as_u64().expect("defaultIndex") as usize;
        assert_eq!(index, 1, "{name}: fixture default index");

        let tip = &html[html
            .rfind(r#"data-slot="chart-tooltip" data-state="open""#)
            .expect("open tooltip")..];
        let datum = &fx["data"][index];
        let keys = series_keys(&fx);
        let t = &fx["tooltip"];
        let unit = t["unit"].as_str().unwrap_or("");

        // The label: hidden, a fixed heading (`labelKey`), a formatted date, or the raw date.
        let date = datum["date"].as_str().unwrap();
        let label = slot_text(tip, "chart-tooltip-label");
        if t["hideLabel"] == true {
            assert_eq!(label, None, "{name}: label hidden");
        } else if let Some(key) = t["labelKey"].as_str() {
            assert_eq!(
                label.as_deref(),
                fx["config"][key]["label"].as_str(),
                "{name}: labelKey"
            );
        } else if t["labelFormatter"] == true {
            assert_eq!(label, Some(long_date(date)), "{name}: labelFormatter");
        } else {
            assert_eq!(label.as_deref(), Some(date), "{name}: raw date label");
        }

        // Indicator shape on the root.
        let expected_indicator = t["indicator"].as_str().unwrap();
        assert!(
            html.contains(&format!(
                r#"class="dx-chart-tooltip" data-indicator="{expected_indicator}""#
            )),
            "{name}: indicator {expected_indicator}"
        );

        // One row per series, with the series' name and the hovered datum's value.
        let rows: Vec<&str> = tip
            .split(r#"data-slot="chart-tooltip-item""#)
            .skip(1)
            .collect();
        assert_eq!(rows.len(), keys.len(), "{name}: rows");
        for (row, key) in rows.iter().zip(&keys) {
            let label = fx["config"][key]["label"].as_str().unwrap();
            assert_eq!(
                slot_text(row, "chart-tooltip-name").as_deref(),
                Some(label),
                "{name}: {key} name"
            );
            assert_eq!(
                slot_text(row, "chart-tooltip-value"),
                Some(format!("{}{unit}", shown(num(&datum[key])))),
                "{name}: {key} value"
            );

            // Swatch or icon: hidden, shadcn's own colored square (`formatter` that draws one),
            // the config's icon, or the default dot/line.
            let has_icon = fx["config"][key].get("icon").is_some();
            let formatter_without_swatch = t["formatter"] == true && t["customSwatch"] != true;
            let wants_marker = t["hideIndicator"] != true && !formatter_without_swatch;
            assert_eq!(
                row.contains(r#"data-slot="chart-icon""#),
                wants_marker && has_icon,
                "{name}: {key} icon"
            );
            assert_eq!(
                row.contains(r#"data-slot="chart-swatch""#),
                wants_marker && !has_icon,
                "{name}: {key} swatch"
            );
        }

        // The advanced demo's extra "Total" row, and the tooltip's own width.
        let total = slot_text(tip, "chart-tooltip-total");
        if t["totalRow"] == true {
            let sum: f64 = keys.iter().map(|k| num(&datum[k])).sum();
            assert_eq!(
                total,
                Some(format!("Total{}{unit}", shown(sum))),
                "{name}: total"
            );
        } else {
            assert_eq!(total, None, "{name}: no total");
        }
        match t["className"].as_str() {
            Some("w-[180px]") => assert!(tip.contains("width: 180px"), "{name}: width"),
            Some(other) => panic!("{name}: unhandled className {other}"),
            None => assert!(!tip.contains("width: 180px"), "{name}: no width"),
        }
    }
}

/// The path commands a rounded rectangle with corner radii `[tl, tr, br, bl]` is drawn with:
/// a move, then a line per side with an arc before it for every rounded corner.
fn rounded_rect_commands(radius: &[f64]) -> String {
    let arc = |r: f64| if r > 0.0 { "A" } else { "" };
    format!(
        "M{}L{}L{}L{}Z",
        arc(radius[0]),
        arc(radius[1]),
        arc(radius[2]),
        arc(radius[3])
    )
}

#[test]
fn stacked_bars_are_rounded_like_the_registry() {
    for (name, demo) in demos() {
        let fx = fixture(name);
        let html = render(demo);
        for bar in fx["bars"].as_array().unwrap() {
            let key = bar["dataKey"].as_str().unwrap();
            let radius: Vec<f64> = bar["radius"].as_array().unwrap().iter().map(num).collect();
            let series = html
                .split(&format!(r#"data-slot="chart-series" data-series="{key}""#))
                .nth(1)
                .unwrap_or_else(|| panic!("{name}: series {key}"));
            let series = &series[..series.find("</g>").unwrap()];
            let paths: Vec<&str> = series
                .split(" d=\"")
                .skip(1)
                .map(|d| &d[..d.find('"').unwrap()])
                .collect();
            assert_eq!(
                paths.len(),
                fx["data"].as_array().unwrap().len(),
                "{name}: {key} bars are paths"
            );
            let expected = rounded_rect_commands(&radius);
            for d in paths {
                let commands: String = d.chars().filter(|c| c.is_ascii_alphabetic()).collect();
                assert_eq!(commands, expected, "{name}: {key} corners in {d}");
                // Every arc has the registry's radius (4).
                for arc in d.split(" A").skip(1) {
                    assert!(arc.starts_with("4 4 "), "{name}: {key} arc radius in {d}");
                }
            }
        }
    }
}

#[test]
fn hover_cursor_is_off_like_the_registry() {
    for (name, demo) in demos() {
        let fx = fixture(name);
        assert_eq!(fx["cursor"], false, "{name}: fixture");
        // Open on the default index, yet no cursor band is drawn behind it.
        let html = render(demo);
        assert_eq!(
            count(&html, r#"data-slot="chart-cursor-rect""#),
            0,
            "{name}: cursor"
        );
        assert_eq!(
            count(&html, r#"data-slot="chart-cursor-line""#),
            0,
            "{name}: cursor"
        );
    }
}
