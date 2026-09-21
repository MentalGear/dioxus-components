//! Analyzer behind `scripts/check-attr-spread-collision.sh` -- see that script's header
//! for the defect this detects, the baseline/ratchet design, and known limitations.
//!
//! This binary does the parsing/analysis only and is deliberately policy-free: it prints
//! every finding it can find, as tab-separated records, to stdout, and a few progress/
//! summary lines to stderr. It does not know about the checked-in baseline and does not
//! decide pass/fail -- that ratchet logic lives entirely in the `.sh` wrapper, which is
//! the thing that actually needs to change if the ratchet policy ever changes.
//!
//! Output (stdout), one finding per line, fields separated by a single tab:
//!   <kind>\t<file>\t<line>\t<enclosing-fn>\t<name>\t<reason>
//!
//! `kind` is `spread` (the primary defect this gate ratchets against) or
//! `component-attributes-forward` (the secondary, experimental check for the
//! `5fc1439`-shaped variant -- see that finding kind's own doc below). Consumers that
//! only care about the primary, ratcheted count should filter to `kind == "spread"`.
//!
//! Approach: real parsing, not text/indentation heuristics (this replaces
//! `dev-docs/issues/duplicate-attribute-guard-prototype.py`, which was line/indentation
//! based and therefore blind to single-line element bodies -- see that file's own header
//! for the full disclosed-limitations list this rewrite closes).
//!   1. `syn::parse_file` every `.rs` file under the scan roots into a real AST.
//!   2. Walk every file (via `syn::visit::Visit`) to build a GLOBAL map of
//!      component-fn-name -> its typed field/parameter names, exactly as
//!      the prototype's two authoring styles did (see `StructFieldCollector` and
//!      `FnFieldCollector` below).
//!   3. Walk every file again, tracking the lexically-nearest enclosing `fn`, and at each
//!      `rsx! { .. }` macro invocation, hand its token stream to `dioxus_rsx::CallBody`
//!      (the real grammar this repo's own `rsx!` invocations are written against -- the
//!      same crate `dioxus-core-macro` itself uses to expand them) to get a structured
//!      node tree instead of guessing at brace nesting from source indentation.

use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use dioxus_rsx::{
    AttributeName, AttributeValue, BodyNode, CallBody, Component, Element, TemplateBody,
};
use syn::visit::{self, Visit};
use syn::{FnArg, Pat, Type};

/// Bare-ident CSS-shorthand `style` properties (`padding`, `flex_direction`, ...): these
/// fold into a single `style="prop:val;...;"` string on SSR via `dioxus-ssr`'s
/// `accumulated_dynamic_styles` handling, a different, non-erroring code path from the
/// WHATWG duplicate-HTML-attribute class this tool looks for -- see
/// `dev-docs/issues/duplicate-attribute-root-cause.md`'s "what the guard excludes"
/// section. Mechanically regenerated (not hand-copied) from this lockfile's own
/// `dioxus-html/src/attribute_groups.rs` by
/// `scripts/attr-spread-collision-checker/regenerate-attribute-idents.py`; do not hand-edit.
const DEFAULT_STYLE_IDENTS_PATH: &str =
    "dev-docs/issues/duplicate-attribute-style-shorthand-idents.txt";

/// The real `GlobalAttributes` idents (`id`, `class`, `role`, every `aria_*`, ...) --
/// the only names a `#[props(extends = GlobalAttributes)]` struct's builder generates an
/// ad-hoc setter for. Used only by the secondary, `component-attributes-forward` check
/// (see that finding kind's doc). Mechanically extracted from the same source and by the
/// same script as the style-shorthand list above; do not hand-edit.
const DEFAULT_GLOBAL_IDENTS_PATH: &str =
    "dev-docs/issues/duplicate-attribute-global-attribute-idents.txt";

const DEFAULT_ROOTS: &[&str] = &["primitives/src", "preview/src/components"];

fn main() -> ExitCode {
    let mut roots: Vec<String> = Vec::new();
    let mut style_idents_path = DEFAULT_STYLE_IDENTS_PATH.to_string();
    let mut global_idents_path = DEFAULT_GLOBAL_IDENTS_PATH.to_string();

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--style-idents" => {
                style_idents_path = args.next().expect("--style-idents needs a path");
            }
            "--global-idents" => {
                global_idents_path = args.next().expect("--global-idents needs a path");
            }
            other => roots.push(other.to_string()),
        }
    }
    if roots.is_empty() {
        roots = DEFAULT_ROOTS.iter().map(|s| s.to_string()).collect();
    }

    let style_idents = load_ident_set(&style_idents_path);
    let global_idents = load_ident_set(&global_idents_path);

    let mut files: Vec<PathBuf> = Vec::new();
    for root in &roots {
        collect_rs_files(Path::new(root), &mut files);
    }
    files.sort();

    let mut parsed: Vec<(PathBuf, syn::File)> = Vec::new();
    for path in &files {
        let src = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("# SKIP {}: {e}", path.display());
                continue;
            }
        };
        match syn::parse_file(&src) {
            Ok(file) => parsed.push((path.clone(), file)),
            Err(e) => {
                eprintln!("# PARSE ERROR {}: {e}", path.display());
            }
        }
    }

    // Phase 1: struct field names, keyed by struct name, GLOBALLY across every scanned
    // file (a Props struct and the component fn using it are always in the same file in
    // this codebase, but the prototype resolved this globally too, and a stray
    // resolution across files is strictly more permissive, never a new false positive --
    // it can only turn an "unresolved" into a resolved one).
    let mut struct_fields: HashMap<String, HashSet<String>> = HashMap::new();
    for (_, file) in &parsed {
        let mut collector = StructFieldCollector {
            out: &mut struct_fields,
        };
        collector.visit_file(file);
    }

    // Phase 2: fn/component name -> typed field set, resolved via phase 1 for the
    // `props: XProps` manual style, or the literal parameter names for `#[component]`
    // sugar. `None` means "we saw this name called as a function but could not resolve
    // any typed fields for it" (kept distinct from "never seen" for the
    // `Reason::UnresolvedFn` case below -- see `FnFieldCollector::record`).
    let mut fn_typed_fields: HashMap<String, Option<HashSet<String>>> = HashMap::new();
    for (_, file) in &parsed {
        let mut collector = FnFieldCollector {
            struct_fields: &struct_fields,
            out: &mut fn_typed_fields,
        };
        collector.visit_file(file);
    }

    // Phase 3: walk every file's `rsx! { .. }` invocations and flag collisions.
    let mut findings: Vec<Finding> = Vec::new();
    for (path, file) in &parsed {
        let display_path = display_path(path);
        let mut finder = RsxFinder {
            file: &display_path,
            current_fn: None,
            fn_typed_fields: &fn_typed_fields,
            style_idents: &style_idents,
            global_idents: &global_idents,
            findings: &mut findings,
        };
        finder.visit_file(file);
    }

    findings.sort_by(|a, b| {
        (a.file.as_str(), a.line, a.name.as_str(), a.kind.as_str()).cmp(&(
            b.file.as_str(),
            b.line,
            b.name.as_str(),
            b.kind.as_str(),
        ))
    });

    let spread_count = findings
        .iter()
        .filter(|f| f.kind == FindingKind::Spread)
        .count();
    let component_count = findings
        .iter()
        .filter(|f| f.kind == FindingKind::ComponentAttributesForward)
        .count();
    let files_with_spread: HashSet<&str> = findings
        .iter()
        .filter(|f| f.kind == FindingKind::Spread)
        .map(|f| f.file.as_str())
        .collect();

    for f in &findings {
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}",
            f.kind.as_str(),
            f.file,
            f.line,
            f.enclosing_fn.as_deref().unwrap_or("<module-level>"),
            f.name,
            f.reason.as_str(),
        );
    }

    eprintln!("# files parsed: {}", parsed.len());
    eprintln!("# struct defs indexed: {}", struct_fields.len());
    eprintln!("# fn/component defs indexed: {}", fn_typed_fields.len());
    eprintln!("# style-shorthand idents loaded: {}", style_idents.len());
    eprintln!("# global-attribute idents loaded: {}", global_idents.len());
    eprintln!(
        "# TOTAL spread findings: {spread_count} across {} files",
        files_with_spread.len()
    );
    eprintln!("# TOTAL component-attributes-forward findings: {component_count}");

    ExitCode::SUCCESS
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// An attribute-like name (an rsx `AttributeName::BuiltIn`, a struct field, or a
/// `#[component]`-sugar parameter) whose Rust spelling is a keyword -- `type`, `for`,
/// ... -- round-trips through `syn::Ident`/`dioxus_rsx` as a raw identifier (`r#type`),
/// since that's the only legal Rust spelling for a field/parameter of that name. Strip
/// the `r#` prefix so findings read the way a caller actually writes the attribute in
/// rsx (`type: "button"`, never `r#type: "button"`), and, just as importantly, so a
/// typed field's name and an attribute's name normalize to the SAME string on both
/// sides of every `HashSet::contains` check in this file -- both go through this same
/// function, so the comparison stays correct either way, but un-prefixed reads right in
/// output and in the checked-in baseline.
fn ident_name(ident: &syn::Ident) -> String {
    let s = ident.to_string();
    s.strip_prefix("r#").map(str::to_string).unwrap_or(s)
}

fn load_ident_set(path: &str) -> HashSet<String> {
    match fs::read_to_string(path) {
        Ok(text) => text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(String::from)
            .collect(),
        Err(_) => HashSet::new(),
    }
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = entries.filter_map(Result::ok).collect();
    entries.sort_by_key(|e| e.path());
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum FindingKind {
    /// The primary defect: a literal attribute rendered beside a `..spread` on the same
    /// element, where the enclosing component's props have no typed field of that name.
    /// This is the count ratcheted against the checked-in baseline.
    Spread,
    /// The `5fc1439`-shaped variant the prototype disclosed it could not detect: a
    /// component invocation that both (a) sets an ad-hoc `#[props(extends =
    /// GlobalAttributes)]`-routed key directly (e.g. `class: "dx-label"`) and (b)
    /// separately forwards a whole `attributes: expr` field to the SAME child -- both
    /// funnel into the child's one `attributes: Vec<Attribute>` field, colliding there
    /// exactly as a same-element spread collides, just one call-site hop removed. See
    /// this tool's header and the `.sh` wrapper for why this is reported separately
    /// (zero-baseline, not ratcheted against the 623/624 prototype-comparable count) and
    /// for its disclosed conservatism (only fires when the OTHER field's name is a known
    /// `GlobalAttributes` ident, and only when the target component resolves).
    ComponentAttributesForward,
}

impl FindingKind {
    fn as_str(self) -> &'static str {
        match self {
            FindingKind::Spread => "spread",
            FindingKind::ComponentAttributesForward => "component-attributes-forward",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Reason {
    /// `AttributeName::Custom("kebab-name")` -- a string-literal attribute name can never
    /// be a Rust struct field name (field names cannot contain `-`), so this is always a
    /// real instance regardless of what the enclosing component's props look like.
    CustomNameAlways,
    /// `AttributeName::BuiltIn(ident)`, the enclosing fn's typed-field set is known, and
    /// `ident` is not in it.
    NoTypedField,
    /// `AttributeName::BuiltIn(ident)`, but this tool could not resolve a typed-field set
    /// for the enclosing fn at all (see `FnFieldCollector`'s doc) -- reported rather than
    /// silently dropped, matching the prototype's own `unresolved enclosing fn` bucket;
    /// expected to be rare-to-never on real code (it was 0/624 in the prototype's final,
    /// bug-fixed run).
    UnresolvedFn,
    /// The `component-attributes-forward` kind's only reason.
    AdHocGlobalAttrPlusForwardedAttributes,
}

impl Reason {
    fn as_str(self) -> &'static str {
        match self {
            Reason::CustomNameAlways => "always (string-literal name, cannot be a typed field)",
            Reason::NoTypedField => "no typed field/param of this name",
            Reason::UnresolvedFn => "unresolved enclosing fn",
            Reason::AdHocGlobalAttrPlusForwardedAttributes => {
                "ad-hoc GlobalAttributes-extends key beside a separately forwarded `attributes:` field on the same child component"
            }
        }
    }
}

struct Finding {
    kind: FindingKind,
    file: String,
    line: usize,
    enclosing_fn: Option<String>,
    name: String,
    reason: Reason,
}

/// Phase 1 visitor: `struct Name { field: T, .. }` -> `{"field", ..}`, for every
/// `ItemStruct` reachable anywhere in the file (any module/fn nesting depth -- the
/// default `syn::visit::Visit` traversal this impl falls through to already walks into
/// `mod { .. }` blocks and fn bodies for us). Deliberately does not require `pub`: the
/// prototype's own iteration history found and fixed exactly this as a false-positive
/// source (`SliderImplProps` is not `pub`).
struct StructFieldCollector<'a> {
    out: &'a mut HashMap<String, HashSet<String>>,
}

impl<'ast> Visit<'ast> for StructFieldCollector<'_> {
    fn visit_item_struct(&mut self, node: &'ast syn::ItemStruct) {
        if let syn::Fields::Named(named) = &node.fields {
            let fields: HashSet<String> = named
                .named
                .iter()
                .filter_map(|f| f.ident.as_ref().map(ident_name))
                .collect();
            self.out.insert(node.ident.to_string(), fields);
        }
        visit::visit_item_struct(self, node);
    }
}

/// Phase 2 visitor: every `fn`/method name (bare ident, matching the prototype's own
/// global-by-bare-name keying -- validated in the root-cause investigation as sound for
/// this codebase's two authoring styles; see this tool's module doc) -> its typed field
/// set, `None` if unresolvable.
struct FnFieldCollector<'a> {
    struct_fields: &'a HashMap<String, HashSet<String>>,
    out: &'a mut HashMap<String, Option<HashSet<String>>>,
}

impl<'a> FnFieldCollector<'a> {
    fn record(
        &mut self,
        name: &syn::Ident,
        inputs: &syn::punctuated::Punctuated<FnArg, syn::token::Comma>,
    ) {
        let typed_params: Vec<&syn::PatType> = inputs
            .iter()
            .filter_map(|a| match a {
                FnArg::Typed(pt) => Some(pt),
                FnArg::Receiver(_) => None,
            })
            .collect();

        // Manual style: exactly one non-receiver parameter, shaped `props: XProps` /
        // `props: &XProps`, `XProps` possibly itself generic (`props: FooProps<T>`) --
        // the parameter's own name must be exactly `props`, and its type must be a
        // single-segment path (optionally behind exactly one non-`mut` `&`).
        //
        // The prototype's own `OLD_STYLE_PROPS_RE` was anchored with a trailing `$` right
        // after the bare identifier, so it did NOT match a generic `props: FooProps<T>`
        // and silently fell through to the `#[component]`-sugar branch below, resolving
        // to the single literal param name `props` instead of `FooProps<T>`'s real
        // fields. Caught empirically while validating this port (spot-checking
        // `component-attributes-forward` findings surfaced a false positive on
        // `ComboboxOption`'s real, typed `id` field -- `ComboboxOptionProps<T>`), and
        // confirmed as a class, not a one-off: ~16 components across `primitives/src`
        // and `preview/src/components` take a generic Props type this same way
        // (`CommandItemProps<T>`, `SelectOptionProps<T>`, `TagOptionProps<T>`,
        // `DropdownMenuItemProps<T>`, `DatePickerCalendarProps<CalendarProps>`, ...) --
        // every one of them was silently mis-resolved by the prototype's regex (and by
        // this tool's own first draft, before this fix). This is a deliberate,
        // by-construction improvement over the prototype, not a faithful port of its
        // bug: `syn` already hands us the real parsed type, so there is no reason to
        // reproduce a regex anchoring accident. See this tool's header comment and the
        // `.sh` wrapper's for how this shows up in the final count vs. the prototype's.
        if typed_params.len() == 1 {
            if let Pat::Ident(pat_ident) = &*typed_params[0].pat {
                if pat_ident.ident == "props" {
                    if let Some(struct_name) = generic_reference_type_ident(&typed_params[0].ty) {
                        let resolved = self.struct_fields.get(&struct_name).cloned();
                        self.out.insert(name.to_string(), resolved);
                        return;
                    }
                }
            }
        }

        // `#[component]` sugar: the literal parameter names are the typed fields.
        let mut names = HashSet::new();
        for pt in &typed_params {
            if let Pat::Ident(pat_ident) = &*pt.pat {
                names.insert(ident_name(&pat_ident.ident));
            }
        }
        let resolved = if names.is_empty() && !typed_params.is_empty() {
            None
        } else {
            Some(names)
        };
        self.out.insert(name.to_string(), resolved);
    }
}

/// `&Ident<..>` or bare `Ident<..>` (single path segment, any/no generic arguments) ->
/// the segment's own ident, i.e. the struct name with its generics stripped; anything
/// else (`&mut`, a multi-segment path, a qualified `<T as Trait>::X` path, ...) ->
/// `None`. Generic arguments are intentionally ignored, not required-absent -- see the
/// call site's doc for why (a real, empirically-found false-positive class this fixes).
fn generic_reference_type_ident(ty: &Type) -> Option<String> {
    let inner = match ty {
        Type::Reference(r) if r.mutability.is_none() => &*r.elem,
        other => other,
    };
    match inner {
        Type::Path(tp) if tp.qself.is_none() && tp.path.segments.len() == 1 => {
            Some(tp.path.segments[0].ident.to_string())
        }
        _ => None,
    }
}

impl<'ast> Visit<'ast> for FnFieldCollector<'_> {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        self.record(&node.sig.ident, &node.sig.inputs);
        visit::visit_item_fn(self, node);
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        self.record(&node.sig.ident, &node.sig.inputs);
        visit::visit_impl_item_fn(self, node);
    }
}

/// Phase 3 visitor: tracks the lexically-nearest enclosing `fn`/method name while
/// walking the whole file, and at every `rsx! { .. }` macro invocation, parses its token
/// stream with `dioxus_rsx::CallBody` and walks the resulting node tree.
struct RsxFinder<'a> {
    file: &'a str,
    current_fn: Option<String>,
    fn_typed_fields: &'a HashMap<String, Option<HashSet<String>>>,
    style_idents: &'a HashSet<String>,
    global_idents: &'a HashSet<String>,
    findings: &'a mut Vec<Finding>,
}

impl<'ast> Visit<'ast> for RsxFinder<'_> {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        let prev = self.current_fn.replace(node.sig.ident.to_string());
        visit::visit_item_fn(self, node);
        self.current_fn = prev;
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        let prev = self.current_fn.replace(node.sig.ident.to_string());
        visit::visit_impl_item_fn(self, node);
        self.current_fn = prev;
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        let is_rsx = node
            .path
            .segments
            .last()
            .is_some_and(|seg| seg.ident == "rsx");
        if is_rsx {
            // Known, disclosed gap (see the `.sh` wrapper's header): an `rsx! { .. }`
            // invocation written INSIDE another `rsx! { .. }`'s own token stream (as
            // opposed to a plain nested element/component, which `dioxus_rsx` already
            // models structurally and this tool already walks via `walk_template_body`)
            // would not be found here, because a macro's `.tokens` are opaque to `syn`
            // until something explicitly reparses them -- exactly what this branch does
            // for the OUTER call, but nothing does for a hypothetical doubly-nested one.
            // Not chased further: idiomatic `rsx!` nests via plain child elements, not a
            // second macro invocation, so this is believed to be at most a narrow,
            // real-world-rare gap, not a systematic blind spot like the prototype's
            // single-line one this tool was written to close.
            if let Ok(call_body) = syn::parse2::<CallBody>(node.tokens.clone()) {
                self.walk_template_body(&call_body.body);
            }
        } else {
            visit::visit_macro(self, node);
        }
    }
}

impl RsxFinder<'_> {
    fn walk_template_body(&mut self, body: &TemplateBody) {
        for node in &body.roots {
            self.walk_body_node(node);
        }
    }

    fn walk_body_node(&mut self, node: &BodyNode) {
        match node {
            BodyNode::Element(el) => {
                self.check_element(el);
                for child in &el.children {
                    self.walk_body_node(child);
                }
            }
            BodyNode::Component(comp) => {
                self.check_component_attributes_forward(comp);
                self.walk_template_body(&comp.children);
            }
            BodyNode::ForLoop(floop) => self.walk_template_body(&floop.body),
            BodyNode::IfChain(chain) => {
                let mut branches = Vec::new();
                chain.for_each_branch(&mut |b: &TemplateBody| branches.push(b.clone()));
                for branch in &branches {
                    self.walk_template_body(branch);
                }
            }
            BodyNode::Text(_) | BodyNode::RawExpr(_) => {}
        }
    }

    fn check_element(&mut self, el: &Element) {
        if el.spreads.is_empty() {
            return;
        }
        for attr in &el.raw_attributes {
            if attr.name.is_likely_event() || attr.name.is_likely_key() {
                continue;
            }
            let (name, is_custom) = match &attr.name {
                AttributeName::BuiltIn(ident) => (ident_name(ident), false),
                AttributeName::Custom(lit) => (lit.value(), true),
                AttributeName::Spread(_) => continue,
            };
            if !is_custom && self.style_idents.contains(&name) {
                continue;
            }
            let line = attr.name.span().start().line;
            let (reason, enclosing_fn) = if is_custom {
                (Reason::CustomNameAlways, self.current_fn.clone())
            } else {
                match self
                    .current_fn
                    .as_deref()
                    .and_then(|f| self.fn_typed_fields.get(f))
                {
                    None => (Reason::UnresolvedFn, self.current_fn.clone()),
                    Some(None) => (Reason::UnresolvedFn, self.current_fn.clone()),
                    Some(Some(fields)) if fields.contains(&name) => continue,
                    Some(Some(_)) => (Reason::NoTypedField, self.current_fn.clone()),
                }
            };
            self.findings.push(Finding {
                kind: FindingKind::Spread,
                file: self.file.to_string(),
                line,
                enclosing_fn,
                name,
                reason,
            });
        }
    }

    /// The `5fc1439`-shaped check -- see `FindingKind::ComponentAttributesForward`'s doc.
    fn check_component_attributes_forward(&mut self, comp: &Component) {
        let has_forwarded_attributes = comp.fields.iter().any(|f| {
            matches!(&f.name, AttributeName::BuiltIn(ident) if ident == "attributes")
                && matches!(
                    f.value,
                    AttributeValue::AttrExpr(_) | AttributeValue::Shorthand(_)
                )
        });
        if !has_forwarded_attributes {
            return;
        }

        let Some(target) = comp.name.segments.last().map(|s| s.ident.to_string()) else {
            return;
        };
        // Conservative by design (see this tool's module doc): only act when we can
        // actually resolve the callee's own typed fields, so we never flag a name that
        // turns out to be a genuine typed prop on the target (the same discriminator the
        // primary check uses, applied one call-site hop over).
        let Some(Some(target_fields)) = self.fn_typed_fields.get(&target) else {
            return;
        };

        for field in &comp.fields {
            let AttributeName::BuiltIn(ident) = &field.name else {
                continue;
            };
            let name = ident_name(ident);
            if name == "attributes" {
                continue;
            }
            if !self.global_idents.contains(&name) {
                continue;
            }
            if target_fields.contains(&name) {
                continue;
            }
            let line = field.name.span().start().line;
            self.findings.push(Finding {
                kind: FindingKind::ComponentAttributesForward,
                file: self.file.to_string(),
                line,
                enclosing_fn: self.current_fn.clone(),
                name,
                reason: Reason::AdHocGlobalAttrPlusForwardedAttributes,
            });
        }
    }
}
