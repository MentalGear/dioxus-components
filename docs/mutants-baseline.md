# cargo-mutants baseline — algorithmic modules, 2026-08-31

One-off baseline per [`backlog.md`](./backlog.md) row 17, scoped to the 9 unit-tested algorithmic files in `primitives/src` (per `conformance-harness.md`'s list, `lib.rs` excluded). 765 mutants in ~110 min (`cargo mutants -p dioxus-primitives -f <files> --timeout-multiplier 3 -j 2`, three passes).

## Score: 37% (233 caught / 637 viable) — 397 survivors

| File | Total | Caught | Missed | Unviable | Timeout |
|---|--:|--:|--:|--:|--:|
| calendar.rs | 253 | 45 | 145 | 57 | 6 |
| select/text_search.rs | 143 | 92 | 50 | 1 | 0 |
| slider.rs | 110 | 34 | 50 | 26 | 0 |
| virtual/virtualizer.rs | 79 | 44 | 25 | 9 | 1 |
| date_picker.rs | 71 | **0** | 52 | 19 | 0 |
| color_picker.rs | 45 | **0** | 36 | 9 | 0 |
| move_interaction.rs | 26 | 9 | 12 | 5 | 0 |
| selection.rs | 24 | 7 | 15 | 2 | 0 |
| pointer.rs | 14 | 2 | 12 | 0 | 0 |

`date_picker.rs` and `color_picker.rs` caught **zero** viable mutants — their tests are SSR-render smoke checks that never touch the algorithmic core.

## Classification: ~350 of 397 survivors are genuine, unit-killable gaps

Headline find — the exact failure mode this baseline exists to catch: **`calendar.rs` `calendar_grid_weeks` (the shipped month-grid builder) survived all 9 of its mutants because `test_calendar_grid_weeks` tests a hand-copied reimplementation of the function instead of calling it.**

The eight concrete, cheap fixes (kill the large majority of survivors):

1. `calendar.rs:1613-1644` — point `test_calendar_grid_weeks` at the real function.
2. `text_search.rs:329-364` — one parametrized test over all 36 `code_to_char` arms (30/36 deletable today).
3. `slider.rs:770-810` — `with_runtime` test constructing `SliderContext` directly (~24 survivors; the free functions are tested, the signal-wiring methods are not).
4. `pointer.rs` — same `with_runtime` pattern for the public `GlobalSignal` API (12/12 missed today).
5. `color_picker.rs:20-83` — direct tests for the pure color math (`color_hex`, `area_value_from_hsv`, clamps).
6. `selection.rs:60-97` — direct tests for `selected_text`, `option_text_value`, and `RcPartialEqValue`'s `PartialEq` (never exercised by any test).
7. `date_picker.rs:678-947` — extract segment key-handling from `rsx!` closures into standalone functions (precedent: `move_interaction.rs`'s `MoveEvent::from_keyboard`), then unit-test (~40 of 52 killable).
8. `calendar.rs` pure predicates (`DateRange::contains`, `nth_month_*`, `AvailableRanges::*`, `is_between/start/end`, context accessors, ~70 survivors) — direct tests; calendar has essentially **no** browser-only residue.

Browser-only (expected, Playwright's layer): ~30–40 (drag-effect math in slider/color_picker/move_interaction). Trivial/noise: ~10–15. Timeouts (7): mutations inducing unbounded loops — neutralized, not silent.

## Recommendation (adopted)

**Targeted test additions now; a `workflow_dispatch` (manual-trigger) job as the standing artifact — not nightly.** The gap is test *content*, not monitoring cadence; these files change infrequently, and a nightly re-report of a known backlog is ~2h/day of CI for no new information. Re-run the exact scoped command before releases or after touching these files.

Full per-mutant detail was archived off-repo by the run; `mutants.out*/` is gitignored.
