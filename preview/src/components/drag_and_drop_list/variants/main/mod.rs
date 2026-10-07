use super::super::component::*;
use crate::components::avatar::{ImageAvatar, AvatarImageSize};
use dioxus::prelude::*;

#[derive(Clone, Copy)]
struct TaskView {
    code: &'static str,
    title: &'static str,
    status: &'static str,
    assignee: &'static str,
    avatar: Asset,
    due: &'static str,
    urgent: bool,
}

// Avatar images are the bundled illustrations in `preview/assets/avatars`
// (CC0, see its README), one per assignee, so the demo needs no network.
// Each task indexes into this list and the initials fall back in if the
// image fails to load.
const AVATARS: &[Asset] = &[
    asset!("/assets/avatars/avery-lin.svg"),
    asset!("/assets/avatars/casey-park.svg"),
    asset!("/assets/avatars/robin-hayes.svg"),
    asset!("/assets/avatars/sarah-chen.svg"),
    asset!("/assets/avatars/marcus-wright.svg"),
    asset!("/assets/avatars/lena-park.svg"),
];

const TASKS: &[TaskView] = &[
    TaskView {
        code: "LNC-128",
        title: "Ship Q2 product roadmap",
        status: "In progress",
        assignee: "AL",
        avatar: AVATARS[0],
        due: "Today",
        urgent: true,
    },
    TaskView {
        code: "LNC-142",
        title: "Redesign onboarding flow",
        status: "In review",
        assignee: "CP",
        avatar: AVATARS[1],
        due: "Apr 24",
        urgent: false,
    },
    TaskView {
        code: "LNC-147",
        title: "Audit payment webhook logs",
        status: "In progress",
        assignee: "RH",
        avatar: AVATARS[2],
        due: "Apr 29",
        urgent: false,
    },
    TaskView {
        code: "LNC-151",
        title: "Draft changelog for v2.4",
        status: "To do",
        assignee: "SC",
        avatar: AVATARS[3],
        due: "May 02",
        urgent: false,
    },
    TaskView {
        code: "LNC-156",
        title: "Migrate analytics to ClickHouse",
        status: "Blocked",
        assignee: "MW",
        avatar: AVATARS[4],
        due: "May 05",
        urgent: false,
    },
    TaskView {
        code: "LNC-160",
        title: "Archive legacy API endpoints",
        status: "Backlog",
        assignee: "LP",
        avatar: AVATARS[5],
        due: "Later",
        urgent: false,
    },
];

#[component]
pub fn Demo() -> Element {
    let items: Vec<Element> = TASKS.iter().map(|t| task_item(*t)).collect();

    rsx! {
        style { r#".dx-tasks-demo {{
  width: 100%;
  max-width: 460px;
  margin: 0 auto;
}}

.dx-tasks-header {{
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  padding: 0 2px 14px;
  gap: 16px;
}}

.dx-tasks-title {{
  margin: 0;
  color: var(--dx-foreground);
  font-size: 14px;
  font-weight: 600;
  line-height: 1.3;
}}

.dx-tasks-subtitle {{
  margin: 4px 0 0;
  color: var(--dx-muted-foreground);
  font-size: 12px;
  line-height: 1.4;
}}

.dx-tasks-count {{
  color: var(--dx-muted-foreground);
  font-size: 12px;
  font-variant-numeric: tabular-nums;
  font-weight: 500;
  white-space: nowrap;
}}

.dx-task-card {{
  display: flex;
  width: 100%;
  min-width: 0;
  align-items: center;
  gap: 12px;
}}

.dx-task-body {{
  display: flex;
  min-width: 0;
  flex: 1;
  flex-direction: column;
  gap: 3px;
}}

.dx-task-title {{
  overflow: hidden;
  color: var(--dx-foreground);
  font-size: 13.5px;
  font-weight: 500;
  line-height: 1.35;
  text-overflow: ellipsis;
  white-space: nowrap;
}}

.dx-task-meta {{
  display: flex;
  align-items: center;
  color: var(--dx-muted-foreground);
  font-size: 11.5px;
  font-variant-numeric: tabular-nums;
  gap: 7px;
  line-height: 1.3;
}}

.dx-task-code {{
  /* axe `color-contrast` (docs/backlog.md row 39, filed 2026-09-03):
   * `--secondary-color-6`'s light value (#d0d0d0) is meant for
   * dark-surface use (its own dark-mode value, #5d5d5d, is the
   * light-surface-appropriate one) -- used here as light-mode text, it
   * measured 1.54:1 on white. `--secondary-color-5` (this theme's actual
   * "muted text on light surfaces" token, used the same way in ~28 other
   * component stylesheets) is the correct token for this role. */
  color: var(--dx-muted-foreground);
  font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, monospace;
  font-size: 10.5px;
  font-weight: 500;
  letter-spacing: 0.03em;
}}

.dx-task-sep {{
  width: 2px;
  height: 2px;
  flex-shrink: 0;
  border-radius: 999px;
  background: var(--dx-muted-foreground);
}}

.dx-task-due[data-urgent="true"] {{
  color: var(--dx-foreground);
  font-weight: 500;
}}

.dx-tasks-demo .dx-drag-and-drop-list-item:hover .dx-task-code {{
  color: var(--dx-foreground);
}}"# }
        div { class: "dx-tasks-demo",
            div { class: "dx-tasks-header",
                div {
                    h2 { class: "dx-tasks-title", "Launch priorities" }
                    p { class: "dx-tasks-subtitle",
                        "Drag to reorder - top is highest priority"
                    }
                }
                span { class: "dx-tasks-count", "6 active" }
            }
            div {
                DragAndDropList { items }
            }
        }
    }
}

fn task_item(t: TaskView) -> Element {
    let urgent_attr = if t.urgent { "true" } else { "false" };
    let avatar_label = format!("Assigned to {}", t.assignee);

    rsx! {
        div { key: "{t.code}", class: "dx-task-card",
            div { class: "dx-task-body",
                div { class: "dx-task-title", "{t.title}" }
                div { class: "dx-task-meta",
                    span { class: "dx-task-code", "{t.code}" }
                    span { class: "dx-task-sep", aria_hidden: "true" }
                    span { "{t.status}" }
                    span { class: "dx-task-sep", aria_hidden: "true" }
                    span {
                        class: "dx-task-due",
                        "data-urgent": "{urgent_attr}",
                        "{t.due}"
                    }
                }
            }
            ImageAvatar {
                size: AvatarImageSize::Small,
                aria_label: "{avatar_label}",
                style: "user-select: none;",
                src: "{t.avatar}",
                alt: "{avatar_label}",
                "{t.assignee}"
            }
        }
    }
}
