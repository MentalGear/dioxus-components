//! Component definitions for the chart primitive -- the Dioxus-facing
//! layer built on [`super::engine`] (see the parent module's doc for the
//! engine/components seam and why it exists).

pub mod container;

pub use container::{ChartContainer, ChartContainerProps};
