//! Structural pretty-printer for [`Value`].
//!
//! See `specs/pretty/api.md` for the canonical specification.
//!
//! Gated behind the `format-pretty` Cargo feature.
//!
//! ```
//! # #[cfg(feature = "format-pretty")] {
//! use mcp_tools::parse_value;
//! use mcp_tools::pretty::{pretty_print, PrettyOpts};
//!
//! let v = parse_value("(define foo bar :doc \"hi\")").unwrap();
//! let opts = PrettyOpts { max_line_width: 20, ..PrettyOpts::default() };
//! let s = pretty_print(&v, &opts);
//! assert!(s.starts_with("(define foo"));
//! # }
//! ```

use crate::Value;

/// Layout options for [`pretty_print`].
///
/// All fields are public so callers can construct via record-update syntax:
///
/// ```
/// # #[cfg(feature = "format-pretty")] {
/// use mcp_tools::pretty::PrettyOpts;
/// let opts = PrettyOpts { max_line_width: 100, ..PrettyOpts::default() };
/// # let _ = opts;
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrettyOpts {
    /// Maximum target line width before a list is wrapped onto multiple lines.
    /// Default: 80. A value of 0 forces every list to wrap.
    pub max_line_width: usize,
    /// Number of spaces per indent level. Default: 2.
    pub indent: usize,
    /// When wrapping a list whose tail is `:k v :k v ...`, align values vertically.
    /// Default: true.
    pub align_keyword_args: bool,
    /// When using [`pretty_print_top_forms`], separate consecutive forms with a blank
    /// line instead of a single newline. Default: true.
    pub blank_line_between_top_forms: bool,
}

impl Default for PrettyOpts {
    fn default() -> Self {
        PrettyOpts {
            max_line_width: 80,
            indent: 2,
            align_keyword_args: true,
            blank_line_between_top_forms: true,
        }
    }
}

/// Pretty-print a single [`Value`] with the given options.
pub fn pretty_print(value: &Value, opts: &PrettyOpts) -> String {
    let mut out = String::new();
    write_value(&mut out, value, 0, opts);
    out
}

/// Pretty-print a single [`Value`] with [`PrettyOpts::default`].
pub fn pretty_print_default(value: &Value) -> String {
    pretty_print(value, &PrettyOpts::default())
}

/// Pretty-print a slice of top-level forms, separated by a blank line (or single
/// newline if `opts.blank_line_between_top_forms` is `false`).
///
/// Returns the empty string for an empty slice. Output never starts or ends with
/// a newline.
pub fn pretty_print_top_forms(values: &[Value], opts: &PrettyOpts) -> String {
    let sep = if opts.blank_line_between_top_forms { "\n\n" } else { "\n" };
    values
        .iter()
        .map(|v| pretty_print(v, opts))
        .collect::<Vec<_>>()
        .join(sep)
}

fn write_value(out: &mut String, value: &Value, current_col: usize, opts: &PrettyOpts) {
    let single = format!("{}", value);
    if current_col + display_width(&single) <= opts.max_line_width {
        out.push_str(&single);
        return;
    }

    match value {
        Value::List(items) if !items.is_empty() => {
            write_wrapped_list(out, items, current_col, opts);
        }
        Value::Pair(pair) => {
            write_wrapped_pair(out, &pair.0, &pair.1, current_col, opts);
        }
        // Atoms and `()` cannot be subdivided — emit as-is even if over budget.
        _ => out.push_str(&single),
    }
}

fn write_wrapped_list(
    out: &mut String,
    items: &[Value],
    current_col: usize,
    opts: &PrettyOpts,
) {
    out.push('(');
    let head = &items[0];
    let head_col = current_col + 1;
    write_value(out, head, head_col, opts);

    let rest = &items[1..];
    if rest.is_empty() {
        out.push(')');
        return;
    }

    let inner_col = current_col + opts.indent.max(1);

    // Try to put first positional on the same line as head, but only when:
    //   (1) head is a (single-line) atom — keeping line column predictable;
    //   (2) the first rest item is a non-keyword atom — same-line for kwargs
    //       defeats the keyword-block alignment;
    //   (3) it actually fits within max_line_width.
    let mut first_idx = 0;
    if is_simple_atom(head) && !rest.is_empty() && is_simple_atom(&rest[0]) && !rest[0].is_keyword()
    {
        let after_head = head_col + display_width(&format!("{}", head));
        let space_col = after_head + 1;
        let first_str = format!("{}", &rest[0]);
        if space_col + display_width(&first_str) <= opts.max_line_width {
            out.push(' ');
            out.push_str(&first_str);
            first_idx = 1;
        }
    }

    let tail = &rest[first_idx..];
    let kw_split = split_keyword_tail(tail);

    match kw_split {
        Some((kw_start, widths)) => {
            // Positional prefix tail[..kw_start] each on own line.
            for item in &tail[..kw_start] {
                out.push('\n');
                push_indent(out, inner_col);
                write_value(out, item, inner_col, opts);
            }
            // Keyword block: each `:kw value` pair on its own line. With
            // `align_keyword_args`, pad each `:kw` to `max_kw`. Without it,
            // emit `:kw value` with a single separating space.
            let max_kw = widths.iter().copied().max().unwrap_or(0);
            let kw_part = &tail[kw_start..];
            let mut i = 0;
            while i + 1 < kw_part.len() {
                let kw_name = kw_part[i].as_keyword().expect("kw_split guarantees keyword");
                let val = &kw_part[i + 1];
                out.push('\n');
                push_indent(out, inner_col);
                let kw_str = format!(":{}", kw_name);
                out.push_str(&kw_str);
                let val_col = if opts.align_keyword_args {
                    let pad = max_kw.saturating_sub(kw_str.len());
                    for _ in 0..pad {
                        out.push(' ');
                    }
                    out.push(' ');
                    inner_col + max_kw + 1
                } else {
                    out.push(' ');
                    inner_col + kw_str.chars().count() + 1
                };
                write_value(out, val, val_col, opts);
                i += 2;
            }
        }
        None => {
            for item in tail {
                out.push('\n');
                push_indent(out, inner_col);
                write_value(out, item, inner_col, opts);
            }
        }
    }

    out.push(')');
}

fn write_wrapped_pair(
    out: &mut String,
    a: &Value,
    b: &Value,
    current_col: usize,
    opts: &PrettyOpts,
) {
    out.push('(');
    write_value(out, a, current_col + 1, opts);
    out.push('\n');
    let inner_col = current_col + opts.indent.max(1);
    push_indent(out, inner_col);
    out.push_str(". ");
    write_value(out, b, inner_col + 2, opts);
    out.push(')');
}

fn push_indent(out: &mut String, col: usize) {
    for _ in 0..col {
        out.push(' ');
    }
}

/// True for atoms whose `Display` form does not contain a newline.
fn is_simple_atom(v: &Value) -> bool {
    matches!(
        v,
        Value::Nil
            | Value::Bool(_)
            | Value::Integer(_)
            | Value::Float(_)
            | Value::Symbol(_)
            | Value::Keyword(_)
            | Value::String(_)
    )
}

/// Visible-width approximation: counts Unicode scalar values, treating each as one
/// column. The pretty printer never inserts non-space whitespace into atoms, so this
/// matches the column geometry the parser will see on round-trip.
fn display_width(s: &str) -> usize {
    s.chars().count()
}

/// If the slice is `[..., :k1, v1, :k2, v2, ...]` with a keyword/value alternation
/// from the first keyword onward, returns `(start_index, kw_widths)`. Each width is
/// the rendered length of the keyword token (`:` + name).
fn split_keyword_tail(tail: &[Value]) -> Option<(usize, Vec<usize>)> {
    let kw_start = tail.iter().position(|v| v.is_keyword())?;
    let kw_part = &tail[kw_start..];
    if kw_part.is_empty() || kw_part.len() % 2 != 0 {
        return None;
    }
    let mut widths = Vec::with_capacity(kw_part.len() / 2);
    for chunk in kw_part.chunks(2) {
        let kw = chunk[0].as_keyword()?;
        widths.push(1 + kw.chars().count());
    }
    Some((kw_start, widths))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parse_value, SpecItem};

    macro_rules! covers {
        ([$($item:expr),* $(,)?]) => {
            { $( let _ = $item; )* }
        };
    }

    fn opts(width: usize) -> PrettyOpts {
        PrettyOpts { max_line_width: width, ..PrettyOpts::default() }
    }

    #[test]
    fn options_default_values() {
        covers!([SpecItem::McpToolsPrettyOptions]);
        let d = PrettyOpts::default();
        assert_eq!(d.max_line_width, 80);
        assert_eq!(d.indent, 2);
        assert!(d.align_keyword_args);
        assert!(d.blank_line_between_top_forms);
    }

    #[test]
    fn options_record_update_syntax() {
        covers!([SpecItem::McpToolsPrettyOptions]);
        let o = PrettyOpts { max_line_width: 40, ..PrettyOpts::default() };
        assert_eq!(o.max_line_width, 40);
        assert_eq!(o.indent, 2);
    }

    #[test]
    fn options_zero_width_does_not_panic() {
        covers!([SpecItem::McpToolsPrettyOptions]);
        let v = parse_value("(a b)").unwrap();
        let _ = pretty_print(&v, &opts(0));
    }

    #[test]
    fn pretty_print_default_uses_default_opts() {
        covers!([SpecItem::McpToolsPrettyPrettyPrint]);
        let v = parse_value("(a b)").unwrap();
        assert_eq!(pretty_print_default(&v), pretty_print(&v, &PrettyOpts::default()));
    }

    #[test]
    fn atoms_emit_display_form() {
        covers!([SpecItem::McpToolsPrettyLayoutRules]);
        let cases: &[(&str, &str)] = &[
            ("42", "42"),
            ("-7", "-7"),
            ("#t", "#t"),
            ("#f", "#f"),
            ("()", "()"),
            ("foo", "foo"),
            (":kw", ":kw"),
            (r#""hello""#, r#""hello""#),
        ];
        for (input, expected) in cases {
            let v = parse_value(input).unwrap();
            assert_eq!(pretty_print_default(&v), *expected, "input: {}", input);
        }
    }

    #[test]
    fn short_list_stays_on_one_line() {
        covers!([SpecItem::McpToolsPrettyLayoutRules]);
        let v = parse_value("(a b c)").unwrap();
        assert_eq!(pretty_print_default(&v), "(a b c)");
    }

    #[test]
    fn long_list_wraps() {
        covers!([SpecItem::McpToolsPrettyLayoutRules]);
        // Width 5: forces wrap.
        let v = parse_value("(define x 42)").unwrap();
        let s = pretty_print(&v, &opts(5));
        // First line gets head + first arg if both atoms (and fit). With width=5,
        // `(define x` is 9 chars, doesn't fit on first attempt — but the same-line
        // optimization will be skipped because `(define` itself exceeds 5.
        // Whatever the exact layout, output must contain newlines and be parseable.
        assert!(s.contains('\n'), "expected wrap, got: {:?}", s);
        let reparsed = parse_value(&s).unwrap();
        assert_eq!(reparsed, v);
    }

    #[test]
    fn empty_list_never_wraps() {
        covers!([SpecItem::McpToolsPrettyLayoutRules]);
        let v = Value::Nil;
        assert_eq!(pretty_print(&v, &opts(0)), "()");
    }

    #[test]
    fn pair_single_line_when_fits() {
        covers!([SpecItem::McpToolsPrettyLayoutRules]);
        let v = parse_value("(a . b)").unwrap();
        assert_eq!(pretty_print_default(&v), "(a . b)");
    }

    #[test]
    fn pair_wraps_when_too_wide() {
        covers!([SpecItem::McpToolsPrettyLayoutRules]);
        let v = parse_value("(aaa . bbb)").unwrap();
        let s = pretty_print(&v, &opts(5));
        assert!(s.contains('\n'));
        assert!(s.contains(". "));
    }

    #[test]
    fn head_and_first_atom_share_line() {
        covers!([SpecItem::McpToolsPrettyLayoutRules]);
        // (define x <body>) — width forces wrap; expect `(define x` on first line.
        let v = parse_value("(define x (lambda () 1))").unwrap();
        let s = pretty_print(&v, &opts(15));
        let first_line = s.lines().next().unwrap();
        assert_eq!(first_line, "(define x");
    }

    #[test]
    fn head_alone_when_first_is_keyword() {
        covers!([SpecItem::McpToolsPrettyKeywordAlignment]);
        // First rest element is a keyword — must NOT share line with head.
        let v = parse_value(r#"(config :a 1 :b 2)"#).unwrap();
        let s = pretty_print(&v, &opts(10));
        let first_line = s.lines().next().unwrap();
        assert_eq!(first_line, "(config");
    }

    #[test]
    fn keyword_block_aligns_values() {
        covers!([SpecItem::McpToolsPrettyKeywordAlignment]);
        let v = parse_value(r#"(define-tool foo :pred (use "p") :body (use "b") :max 3)"#).unwrap();
        let s = pretty_print(&v, &opts(40));
        // Expected layout (indent=2):
        // (define-tool foo
        //   :pred (use "p")
        //   :body (use "b")
        //   :max  3)
        let lines: Vec<&str> = s.lines().collect();
        assert_eq!(lines[0], "(define-tool foo", "got: {:?}", s);
        assert_eq!(lines[1], r#"  :pred (use "p")"#);
        assert_eq!(lines[2], r#"  :body (use "b")"#);
        assert_eq!(lines[3], "  :max  3)");
    }

    #[test]
    fn keyword_alignment_disabled_emits_single_space() {
        covers!([SpecItem::McpToolsPrettyKeywordAlignment]);
        let v = parse_value(r#"(f :a 1 :bb 2)"#).unwrap();
        let s = pretty_print(
            &v,
            &PrettyOpts { max_line_width: 5, align_keyword_args: false, ..PrettyOpts::default() },
        );
        // Expect each :kw value separated by one space, no padding.
        assert!(s.contains("  :a 1"));
        assert!(s.contains("  :bb 2"));
        // No double-space padding.
        assert!(!s.contains(":a  1"));
        assert!(!s.contains(":bb  2"));
    }

    #[test]
    fn pretty_print_top_forms_blank_line_default() {
        covers!([SpecItem::McpToolsPrettyBlankLineBetweenTopForms]);
        let a = parse_value("(define x 1)").unwrap();
        let b = parse_value("(define y 2)").unwrap();
        let s = pretty_print_top_forms(&[a, b], &PrettyOpts::default());
        assert_eq!(s, "(define x 1)\n\n(define y 2)");
    }

    #[test]
    fn pretty_print_top_forms_single_newline_when_disabled() {
        covers!([SpecItem::McpToolsPrettyBlankLineBetweenTopForms]);
        let a = parse_value("(define x 1)").unwrap();
        let b = parse_value("(define y 2)").unwrap();
        let s = pretty_print_top_forms(
            &[a, b],
            &PrettyOpts { blank_line_between_top_forms: false, ..PrettyOpts::default() },
        );
        assert_eq!(s, "(define x 1)\n(define y 2)");
    }

    #[test]
    fn pretty_print_top_forms_empty_returns_empty() {
        covers!([SpecItem::McpToolsPrettyBlankLineBetweenTopForms]);
        assert_eq!(pretty_print_top_forms(&[], &PrettyOpts::default()), "");
    }

    #[test]
    fn pretty_print_top_forms_single_value_no_separator() {
        covers!([SpecItem::McpToolsPrettyBlankLineBetweenTopForms]);
        let v = parse_value("(define x 1)").unwrap();
        let s = pretty_print_top_forms(&[v], &PrettyOpts::default());
        assert_eq!(s, "(define x 1)");
    }

    #[test]
    fn deterministic_output() {
        covers!([SpecItem::McpToolsPrettyDeterminism]);
        let v = parse_value(r#"(tool :a 1 :b 2 :c (nested 1 2 3) :d "long string here")"#).unwrap();
        let opts = opts(30);
        let s1 = pretty_print(&v, &opts);
        let s2 = pretty_print(&v, &opts);
        assert_eq!(s1, s2);
        // Repeat several times to expose any randomized iteration order.
        for _ in 0..50 {
            assert_eq!(pretty_print(&v, &opts), s1);
        }
    }

    #[test]
    fn round_trip_simple_atoms() {
        covers!([SpecItem::McpToolsPrettyRoundTrip]);
        for input in &["42", "#t", "#f", r#""hello""#, "foo", ":kw", "()"] {
            let v = parse_value(input).unwrap();
            let s = pretty_print_default(&v);
            let r = parse_value(&s).unwrap();
            assert_eq!(r, v, "round-trip failed for {}", input);
        }
    }

    #[test]
    fn round_trip_nested_lists() {
        covers!([SpecItem::McpToolsPrettyRoundTrip]);
        let inputs = [
            r#"(define foo (lambda (x) (+ x 1)))"#,
            r#"(tool :a 1 :b (nested :c 2 :d 3))"#,
            r#"((a b) (c d) (e f))"#,
        ];
        for input in &inputs {
            let v = parse_value(input).unwrap();
            for w in [10usize, 20, 40, 80, 200] {
                let s = pretty_print(&v, &opts(w));
                let r = parse_value(&s)
                    .unwrap_or_else(|e| panic!("re-parse failed: {} for input={} width={}\noutput:\n{}", e, input, w, s));
                assert_eq!(r, v, "round-trip failed for {} at width {}", input, w);
            }
        }
    }

    #[test]
    fn round_trip_dotted_pair() {
        covers!([SpecItem::McpToolsPrettyRoundTrip]);
        let v = parse_value("(a . b)").unwrap();
        for w in [3usize, 5, 80] {
            let s = pretty_print(&v, &opts(w));
            let r = parse_value(&s).unwrap();
            assert_eq!(r, v);
        }
    }

    #[test]
    fn round_trip_strings_with_escapes() {
        covers!([SpecItem::McpToolsPrettyRoundTrip]);
        let v = Value::String(r#"a "b" \c"#.to_string());
        let s = pretty_print_default(&v);
        let r = parse_value(&s).unwrap();
        assert_eq!(r, v);
    }

    #[test]
    fn feature_gate_exposes_public_api() {
        covers!([SpecItem::McpToolsPrettyFeatureGate]);
        // Compiles only when `format-pretty` is enabled.
        let _: fn(&Value, &PrettyOpts) -> String = pretty_print;
        let _: fn(&Value) -> String = pretty_print_default;
        let _: fn(&[Value], &PrettyOpts) -> String = pretty_print_top_forms;
    }

    #[test]
    fn no_trailing_newline() {
        covers!([SpecItem::McpToolsPrettyPrettyPrint]);
        let v = parse_value("(a (b c) (d e))").unwrap();
        for w in [5usize, 80] {
            let s = pretty_print(&v, &opts(w));
            assert!(!s.ends_with('\n'), "trailing newline in: {:?}", s);
            assert!(!s.starts_with('\n'), "leading newline in: {:?}", s);
        }
    }
}
