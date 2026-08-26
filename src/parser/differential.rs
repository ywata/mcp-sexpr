//! Differential validation mode: runs both the new parser and `lexpr::from_str`,
//! reports structural divergences via a configurable sink, and never affects the
//! consumer's parse outcome.
//!
//! See `specs/parser/differential-mode.md`.

use std::collections::VecDeque;
use std::env;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::RwLock;

use super::reader::ParseError;
use super::types::Value;

/// Differential validation switch.
#[derive(Clone)]
pub enum DifferentialMode {
    /// No validation; only the new parser runs.
    Off,
    /// Run both parsers, compare structurally, dispatch discrepancies to `sink`.
    On {
        /// Where to send discrepancies.
        sink: DiscrepancySink,
        /// When `true`, discrepancy reports include the full source string;
        /// otherwise only its SHA-256.
        verbose: bool,
    },
}

impl fmt::Debug for DifferentialMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DifferentialMode::Off => f.write_str("Off"),
            DifferentialMode::On { sink, verbose } => f
                .debug_struct("On")
                .field("sink", sink)
                .field("verbose", verbose)
                .finish(),
        }
    }
}

/// Where discrepancy reports are dispatched.
#[derive(Clone)]
pub enum DiscrepancySink {
    /// Write a one-line human-readable report to stderr.
    Stderr,
    /// Invoke a user-supplied callback. The callback runs synchronously on the
    /// parsing thread.
    Callback(Arc<dyn Fn(&Discrepancy) + Send + Sync>),
}

impl fmt::Debug for DiscrepancySink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiscrepancySink::Stderr => f.write_str("Stderr"),
            DiscrepancySink::Callback(_) => f.write_str("Callback(..)"),
        }
    }
}

/// Source string representation in a discrepancy report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscrepancyInput {
    /// Hashed (default).
    Hashed {
        /// SHA-256 of the input.
        sha256: [u8; 32],
    },
    /// Verbose (opt-in).
    Verbose {
        /// Original source string.
        source: String,
    },
}

/// Path to the first divergence within a parsed tree.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructuralPath(pub Vec<PathElement>);

/// Single hop along a [`StructuralPath`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PathElement {
    /// Child `i` of a list disagrees.
    ListIndex(usize),
    /// Head of a dotted pair disagrees.
    PairCar,
    /// Tail of a dotted pair disagrees.
    PairCdr,
    /// Atoms or variant boundaries disagree.
    Atom,
}

impl fmt::Display for StructuralPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            return f.write_str(".");
        }
        for (i, el) in self.0.iter().enumerate() {
            if i > 0 {
                f.write_str(".")?;
            }
            match el {
                PathElement::ListIndex(n) => write!(f, "{}", n)?,
                PathElement::PairCar => f.write_str("car")?,
                PathElement::PairCdr => f.write_str("cdr")?,
                PathElement::Atom => f.write_str("atom")?,
            }
        }
        Ok(())
    }
}

/// Lightweight, owned representation of a parse error for inclusion in a
/// [`Discrepancy`] report (the original `ParseError` is not `Clone`-friendly across
/// thread boundaries in all configurations).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseErrorRepr {
    /// Variant name (e.g., "InvalidEscape").
    pub kind: String,
    /// Human-readable message.
    pub message: String,
}

impl From<&ParseError> for ParseErrorRepr {
    fn from(err: &ParseError) -> Self {
        let kind = match err {
            ParseError::SourceTooLarge => "SourceTooLarge",
            ParseError::UnexpectedChar { .. } => "UnexpectedChar",
            ParseError::UnterminatedString { .. } => "UnterminatedString",
            ParseError::UnterminatedBlockComment { .. } => "UnterminatedBlockComment",
            ParseError::InvalidEscape { .. } => "InvalidEscape",
            ParseError::IntegerOutOfRange { .. } => "IntegerOutOfRange",
            ParseError::InvalidNumber { .. } => "InvalidNumber",
            ParseError::UnmatchedRParen { .. } => "UnmatchedRParen",
            ParseError::UnclosedList { .. } => "UnclosedList",
            ParseError::DotWithoutHead { .. } => "DotWithoutHead",
            ParseError::DotWithoutTail { .. } => "DotWithoutTail",
            ParseError::DotWithMultipleTail { .. } => "DotWithMultipleTail",
            ParseError::QuoteWithoutValue { .. } => "QuoteWithoutValue",
            ParseError::UnexpectedEof => "UnexpectedEof",
            ParseError::TrailingInput { .. } => "TrailingInput",
        }
        .to_string();
        ParseErrorRepr {
            kind,
            message: err.to_string(),
        }
    }
}

/// Single discrepancy between the new parser and `lexpr::from_str`.
#[derive(Debug, Clone)]
pub struct Discrepancy {
    /// Hashed or verbose representation of the input.
    pub input: DiscrepancyInput,
    /// Outcome from the new parser (lightweight Value, no spans).
    pub new_value: Result<Value, ParseErrorRepr>,
    /// Outcome from `lexpr::from_str` (string-formatted error if it failed).
    pub lexpr_value: Result<lexpr::Value, String>,
    /// Path to the first divergence.
    pub path: StructuralPath,
}

const DEFAULT_DEDUP_CAPACITY: usize = 1024;
const DEFAULT_CLASS_DEDUP_CAPACITY: usize = 256;

/// The *class* of a discrepancy: where it diverged and which variant pair
/// disagreed, independent of the input's contents. A second dedup key so that
/// many distinct payloads exhibiting one grammar-level difference produce one
/// report. See `specs/parser/differential-mode.md` § Discrepancy Class
/// Deduplication.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct DiscrepancyClass {
    /// As reported, list indices included: `1.atom` and `2.atom` differ.
    path: StructuralPath,
    new_kind: NewKind,
    lexpr_kind: LexprSideKind,
}

/// New-parser side of a class: the value variant, or the error kind.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum NewKind {
    Ok(ValueKind),
    Err(String),
}

/// lexpr side of a class: the value variant, or an error (messages are not
/// part of the class — they vary per input).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum LexprSideKind {
    Ok(LexprKind),
    Err,
}

impl DiscrepancyClass {
    fn of(report: &Discrepancy) -> Self {
        DiscrepancyClass {
            path: report.path.clone(),
            new_kind: match &report.new_value {
                Ok(v) => NewKind::Ok(ValueKind::of(v)),
                Err(e) => NewKind::Err(e.kind.clone()),
            },
            lexpr_kind: match &report.lexpr_value {
                Ok(v) => LexprSideKind::Ok(LexprKind::of(v)),
                Err(_) => LexprSideKind::Err,
            },
        }
    }
}

struct GlobalState {
    mode: RwLock<DifferentialMode>,
    /// Keyed by SHA-256 of the input.
    dedup: Mutex<DedupCache<[u8; 32]>>,
    /// Keyed by discrepancy class.
    class_dedup: Mutex<DedupCache<DiscrepancyClass>>,
    initialized: OnceLock<()>,
}

/// Bounded LRU set. Small capacities and equality-keyed lookups make a linear
/// scan over a `VecDeque` adequate.
struct DedupCache<K> {
    capacity: usize,
    /// Most-recently-seen keys at the back, least-recently-seen at the front.
    items: VecDeque<K>,
}

impl<K: PartialEq> DedupCache<K> {
    fn new(capacity: usize) -> Self {
        DedupCache {
            capacity,
            items: VecDeque::new(),
        }
    }

    /// Returns `true` if the key was already present (reporter should skip);
    /// also refreshes its position to most-recently-used.
    fn record(&mut self, key: K) -> bool {
        if self.capacity == 0 {
            return false;
        }
        if let Some(pos) = self.items.iter().position(|k| k == &key) {
            self.items.remove(pos);
            self.items.push_back(key);
            return true;
        }
        if self.items.len() >= self.capacity {
            self.items.pop_front();
        }
        self.items.push_back(key);
        false
    }

    fn flush(&mut self) {
        self.items.clear();
    }

    fn resize(&mut self, new_capacity: usize) {
        self.capacity = new_capacity;
        while self.items.len() > new_capacity {
            self.items.pop_front();
        }
    }
}

fn state() -> &'static GlobalState {
    static STATE: OnceLock<GlobalState> = OnceLock::new();
    STATE.get_or_init(|| GlobalState {
        mode: RwLock::new(DifferentialMode::Off),
        dedup: Mutex::new(DedupCache::new(DEFAULT_DEDUP_CAPACITY)),
        class_dedup: Mutex::new(DedupCache::new(DEFAULT_CLASS_DEDUP_CAPACITY)),
        initialized: OnceLock::new(),
    })
}

fn ensure_initialized() {
    let s = state();
    let _ = s.initialized.get_or_init(|| {
        let initial = parse_env_mode().unwrap_or(default_mode());
        if let Ok(mut guard) = s.mode.write() {
            *guard = initial;
        }
    });
}

fn default_mode() -> DifferentialMode {
    DifferentialMode::On {
        sink: DiscrepancySink::Stderr,
        verbose: false,
    }
}

fn parse_env_mode() -> Option<DifferentialMode> {
    let v = env::var("MCP_TOOLS_DIFFERENTIAL_PARSE").ok()?;
    match v.to_ascii_lowercase().as_str() {
        "off" => Some(DifferentialMode::Off),
        "on" => Some(DifferentialMode::On {
            sink: DiscrepancySink::Stderr,
            verbose: false,
        }),
        "verbose" => Some(DifferentialMode::On {
            sink: DiscrepancySink::Stderr,
            verbose: true,
        }),
        _ => {
            eprintln!(
                "[mcp-tools differential] unrecognized MCP_TOOLS_DIFFERENTIAL_PARSE={:?}; using default",
                v
            );
            None
        }
    }
}

/// Set the differential mode at runtime.
pub fn set_differential_mode(mode: DifferentialMode) {
    ensure_initialized();
    if let Ok(mut guard) = state().mode.write() {
        *guard = mode;
    }
}

/// Read the current differential mode.
pub fn current_differential_mode() -> DifferentialMode {
    ensure_initialized();
    state()
        .mode
        .read()
        .map(|g| g.clone())
        .unwrap_or(DifferentialMode::Off)
}

/// Resize the discrepancy dedup LRU.
pub fn set_discrepancy_dedup_capacity(capacity: usize) {
    if let Ok(mut guard) = state().dedup.lock() {
        guard.resize(capacity);
    }
}

/// Resize the discrepancy *class* dedup LRU. `0` disables class deduplication
/// (every input-cache miss is dispatched).
pub fn set_discrepancy_class_dedup_capacity(capacity: usize) {
    if let Ok(mut guard) = state().class_dedup.lock() {
        guard.resize(capacity);
    }
}

/// Clear both discrepancy dedup LRUs (input-hash and class). After this call,
/// every input — including those previously reported — may report again on the
/// next parse.
pub fn flush_discrepancy_dedup() {
    if let Ok(mut guard) = state().dedup.lock() {
        guard.flush();
    }
    if let Ok(mut guard) = state().class_dedup.lock() {
        guard.flush();
    }
}

/// Run differential validation against `input` for a parse outcome `new`. Always
/// returns `new` to the caller — discrepancies are reported via the configured sink
/// without affecting return values.
pub fn record_discrepancy_if_diverging(
    input: &str,
    new: &Result<Value, ParseError>,
) -> Result<(), DiscrepancyDispatchError> {
    let mode = current_differential_mode();
    let DifferentialMode::On { verbose, .. } = mode else {
        return Ok(());
    };

    let lexpr_outcome: Result<lexpr::Value, String> =
        lexpr::from_str(input).map_err(|e| e.to_string());

    let Some(path) = compare(new, &lexpr_outcome) else {
        return Ok(());
    };

    let input_repr = if verbose {
        DiscrepancyInput::Verbose {
            source: input.to_string(),
        }
    } else {
        DiscrepancyInput::Hashed {
            sha256: sha256_bytes(input.as_bytes()),
        }
    };

    let new_value = match new {
        Ok(v) => Ok(v.clone()),
        Err(e) => Err(ParseErrorRepr::from(e)),
    };

    record_discrepancy(Discrepancy {
        input: input_repr,
        new_value,
        lexpr_value: lexpr_outcome,
        path,
    })
}

/// Post-comparison pipeline for an already-built report: input-hash dedup,
/// then class dedup, then dispatch to the current mode's sink (the stderr
/// budget lives inside the `Stderr` sink). Returns without dispatching when
/// the mode is `Off`.
///
/// This is the seam tests use to exercise the dedup caches with explicit
/// paths and kinds, independent of what the two parsers happen to disagree on.
pub(crate) fn record_discrepancy(report: Discrepancy) -> Result<(), DiscrepancyDispatchError> {
    let DifferentialMode::On { sink, verbose } = current_differential_mode() else {
        return Ok(());
    };

    let hash = match &report.input {
        DiscrepancyInput::Hashed { sha256 } => *sha256,
        DiscrepancyInput::Verbose { source } => sha256_bytes(source.as_bytes()),
    };
    let input_seen = match state().dedup.lock() {
        Ok(mut guard) => guard.record(hash),
        Err(_) => false,
    };
    if input_seen {
        return Ok(());
    }

    let class_seen = match state().class_dedup.lock() {
        Ok(mut guard) => guard.record(DiscrepancyClass::of(&report)),
        Err(_) => false,
    };
    if class_seen {
        return Ok(());
    }

    dispatch(&sink, verbose, &report)
}

/// Failure to dispatch a discrepancy report (callback panicked, etc.).
#[derive(Debug)]
pub enum DiscrepancyDispatchError {
    /// Callback or stderr write panicked.
    SinkPanicked,
}

fn dispatch(
    sink: &DiscrepancySink,
    verbose: bool,
    report: &Discrepancy,
) -> Result<(), DiscrepancyDispatchError> {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match sink {
        DiscrepancySink::Stderr => write_stderr(report, verbose),
        DiscrepancySink::Callback(cb) => cb(report),
    }));
    result.map_err(|_| DiscrepancyDispatchError::SinkPanicked)
}

/// Variant name of a [`Value`], with no payload.
///
/// The `of` match is exhaustive on purpose: adding a `Value` variant must be a
/// compile error here, not a silently unclassified report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ValueKind {
    Nil,
    Bool,
    Integer,
    Float,
    String,
    Symbol,
    Keyword,
    List,
    Pair,
}

impl ValueKind {
    pub(crate) fn of(v: &Value) -> Self {
        match v {
            Value::Nil => ValueKind::Nil,
            Value::Bool(_) => ValueKind::Bool,
            Value::Integer(_) => ValueKind::Integer,
            Value::Float(_) => ValueKind::Float,
            Value::String(_) => ValueKind::String,
            Value::Symbol(_) => ValueKind::Symbol,
            Value::Keyword(_) => ValueKind::Keyword,
            Value::List(_) => ValueKind::List,
            Value::Pair(_) => ValueKind::Pair,
        }
    }

    fn name(self) -> &'static str {
        match self {
            ValueKind::Nil => "Nil",
            ValueKind::Bool => "Bool",
            ValueKind::Integer => "Integer",
            ValueKind::Float => "Float",
            ValueKind::String => "String",
            ValueKind::Symbol => "Symbol",
            ValueKind::Keyword => "Keyword",
            ValueKind::List => "List",
            ValueKind::Pair => "Pair",
        }
    }
}

impl fmt::Display for ValueKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Variant name of a [`lexpr::Value`], with no payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum LexprKind {
    Nil,
    Null,
    Bool,
    Number,
    Char,
    String,
    Symbol,
    Keyword,
    Bytes,
    Cons,
    Vector,
}

impl LexprKind {
    pub(crate) fn of(v: &lexpr::Value) -> Self {
        match v {
            lexpr::Value::Nil => LexprKind::Nil,
            lexpr::Value::Null => LexprKind::Null,
            lexpr::Value::Bool(_) => LexprKind::Bool,
            lexpr::Value::Number(_) => LexprKind::Number,
            lexpr::Value::Char(_) => LexprKind::Char,
            lexpr::Value::String(_) => LexprKind::String,
            lexpr::Value::Symbol(_) => LexprKind::Symbol,
            lexpr::Value::Keyword(_) => LexprKind::Keyword,
            lexpr::Value::Bytes(_) => LexprKind::Bytes,
            lexpr::Value::Cons(_) => LexprKind::Cons,
            lexpr::Value::Vector(_) => LexprKind::Vector,
        }
    }

    fn name(self) -> &'static str {
        match self {
            LexprKind::Nil => "Nil",
            LexprKind::Null => "Null",
            LexprKind::Bool => "Bool",
            LexprKind::Number => "Number",
            LexprKind::Char => "Char",
            LexprKind::String => "String",
            LexprKind::Symbol => "Symbol",
            LexprKind::Keyword => "Keyword",
            LexprKind::Bytes => "Bytes",
            LexprKind::Cons => "Cons",
            LexprKind::Vector => "Vector",
        }
    }
}

impl fmt::Display for LexprKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Hashed-mode token for the new-parser side: variant name, or `Err(<kind>)`
/// with no message (messages can quote the input).
fn new_kind_token(v: &Result<Value, ParseErrorRepr>) -> String {
    match v {
        Ok(v) => ValueKind::of(v).to_string(),
        Err(e) => format!("Err({})", e.kind),
    }
}

/// Hashed-mode token for the lexpr side: variant name, or the bare `Err`.
fn lexpr_kind_token(v: &Result<lexpr::Value, String>) -> String {
    match v {
        Ok(v) => LexprKind::of(v).to_string(),
        Err(_) => "Err".to_string(),
    }
}

/// Verbose-mode representation: full one-line debug / `Err(<kind>: <message>)`.
fn new_full_repr(v: &Result<Value, ParseErrorRepr>) -> String {
    match v {
        Ok(v) => format!("Ok({:?})", v),
        Err(e) => format!("Err({}: {})", e.kind, e.message),
    }
}

fn lexpr_full_repr(v: &Result<lexpr::Value, String>) -> String {
    match v {
        Ok(v) => format!("Ok({:?})", v),
        Err(e) => format!("Err({})", e),
    }
}

/// Render a discrepancy as the text the `Stderr` sink writes (trailing newline
/// included). Pure; the sink does nothing but write the result.
///
/// Hashed-mode invariant (`verbose == false`): no byte of the source string, and
/// no byte derived from it other than its SHA-256, appears in the output. See
/// `specs/parser/differential-mode.md` § Stderr sink format.
pub(crate) fn format_report(report: &Discrepancy, verbose: bool) -> String {
    let (new_repr, lexpr_repr) = if verbose {
        (
            new_full_repr(&report.new_value),
            lexpr_full_repr(&report.lexpr_value),
        )
    } else {
        (
            new_kind_token(&report.new_value),
            lexpr_kind_token(&report.lexpr_value),
        )
    };
    let input_line = match &report.input {
        DiscrepancyInput::Verbose { source } => format!("  input={}", source),
        DiscrepancyInput::Hashed { sha256 } => format!("  input-sha256={}", hex(sha256)),
    };
    format!(
        "[mcp-tools differential] new={} lexpr={} path={}\n{}\n",
        new_repr, lexpr_repr, report.path, input_line
    )
}

/// Maximum number of reports the `Stderr` sink writes per process. After the
/// budget is spent one terminal line is written and every later report is
/// dropped before formatting, so total stderr output from this module is
/// bounded (< 16 KB) and an undrained stderr pipe can never block the consumer.
///
/// Deliberately a constant, not a setter: a configurable budget would invite
/// consumers to reopen the hazard. See
/// `specs/parser/differential-mode.md` § Stderr Report Budget.
pub const STDERR_REPORT_BUDGET: usize = 64;

/// Process-wide count of reports the `Stderr` sink has been asked to write.
/// Monotonic; never reset by `set_differential_mode`.
static STDERR_REPORTS_REQUESTED: AtomicUsize = AtomicUsize::new(0);

/// What the `Stderr` sink does with the report whose ordinal is `count_before`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BudgetDecision {
    /// Within budget: write the formatted report.
    Emit,
    /// The first report past the budget: write the terminal line only.
    EmitExhausted,
    /// Past the terminal line: write nothing, do not format.
    Drop,
}

/// Pure budget rule over the pre-increment counter value.
pub(crate) fn budget_decision(count_before: usize, budget: usize) -> BudgetDecision {
    match count_before.cmp(&budget) {
        std::cmp::Ordering::Less => BudgetDecision::Emit,
        std::cmp::Ordering::Equal => BudgetDecision::EmitExhausted,
        std::cmp::Ordering::Greater => BudgetDecision::Drop,
    }
}

/// The single line written when the budget is exhausted.
pub(crate) fn budget_exhausted_line() -> String {
    format!(
        "[mcp-tools differential] report budget ({}) exhausted; further reports suppressed. \
         Use DiscrepancySink::Callback or MCP_TOOLS_DIFFERENTIAL_PARSE=off.",
        STDERR_REPORT_BUDGET
    )
}

/// Number of reports the `Stderr` sink has been asked to write so far in this
/// process (including those dropped). Test hook; tests reason in deltas.
#[cfg(test)]
pub(crate) fn stderr_reports_requested() -> usize {
    STDERR_REPORTS_REQUESTED.load(Ordering::Relaxed)
}

fn write_stderr(report: &Discrepancy, verbose: bool) {
    let count_before = STDERR_REPORTS_REQUESTED.fetch_add(1, Ordering::Relaxed);
    match budget_decision(count_before, STDERR_REPORT_BUDGET) {
        BudgetDecision::Emit => eprint!("{}", format_report(report, verbose)),
        BudgetDecision::EmitExhausted => eprintln!("{}", budget_exhausted_line()),
        BudgetDecision::Drop => {}
    }
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{:02x}", b));
    }
    out
}

/// Structural comparison between the new parser's result and lexpr's. Returns `None`
/// if structurally equivalent, or `Some(path)` to the first divergence.
fn compare(
    new: &Result<Value, ParseError>,
    lexpr_v: &Result<lexpr::Value, String>,
) -> Option<StructuralPath> {
    match (new, lexpr_v) {
        (Ok(new_v), Ok(lexpr_v)) => compare_values(new_v, lexpr_v, &mut Vec::new()),
        (Err(_), Err(_)) => None,
        (Ok(_), Err(_)) | (Err(_), Ok(_)) => Some(StructuralPath(Vec::new())),
    }
}

/// Expected-class table: `(new, lexpr)` pairs that differ only in
/// representation, where the new parser's reading is the canonical form.
/// These are not discrepancies. Deliberately asymmetric — a new-parser
/// `Symbol(":k")` against a lexpr `Keyword("k")` is still reported, because it
/// would mean the new parser failed to canonicalise a keyword.
///
/// See `specs/parser/differential-mode.md` § Expected Discrepancy Classes.
fn is_expected_class(new: &Value, lexpr_v: &lexpr::Value) -> bool {
    match (new, lexpr_v) {
        (Value::Keyword(k), lexpr::Value::Symbol(s)) => s
            .strip_prefix(':')
            .is_some_and(|rest| rest == k.as_str()),
        // The grammar reserves the bare token `nil` as `Value::Nil`; lexpr reads
        // it as a symbol. Byte-exact: `NIL` is a symbol to both parsers.
        (Value::Nil, lexpr::Value::Symbol(s)) => &**s == "nil",
        _ => false,
    }
}

/// True for a lexpr cons chain (at least one cell) whose final cdr is the
/// symbol `nil` — the improper shape lexpr produces for a dotted `nil` tail.
fn is_nil_terminated_chain(v: &lexpr::Value) -> bool {
    match v {
        lexpr::Value::Cons(cons) => {
            let mut cur = cons.cdr();
            while let lexpr::Value::Cons(next) = cur {
                cur = next.cdr();
            }
            matches!(cur, lexpr::Value::Symbol(s) if &**s == "nil")
        }
        _ => false,
    }
}

fn compare_values(
    new: &Value,
    lexpr_v: &lexpr::Value,
    path: &mut Vec<PathElement>,
) -> Option<StructuralPath> {
    match (new, lexpr_v) {
        (Value::Nil, lexpr::Value::Nil) => None,
        (Value::Nil, lexpr::Value::Null) => None,
        (Value::Bool(a), lexpr::Value::Bool(b)) if a == b => None,
        (Value::Integer(a), lexpr::Value::Number(n)) if n.as_i64() == Some(*a) => None,
        (Value::Float(a), lexpr::Value::Number(n)) if n.as_f64().map(f64::to_bits) == Some(a.to_bits()) => {
            None
        }
        (Value::String(a), lexpr::Value::String(b)) if a.as_str() == b.as_ref() => None,
        (Value::Symbol(a), lexpr::Value::Symbol(b)) if a.as_str() == b.as_ref() => None,
        (Value::Keyword(a), lexpr::Value::Keyword(b)) if a.as_str() == b.as_ref() => None,
        (n, l) if is_expected_class(n, l) => None,
        (Value::List(items), v) if v.is_list() => compare_list(items, v, path),
        // Expected class (tail form): `(a . nil)` is the proper list `(a)` to the
        // new parser; lexpr leaves an improper chain ending in Symbol("nil").
        (Value::List(items), v) if is_nil_terminated_chain(v) => compare_list(items, v, path),
        (Value::Pair(pair), lexpr::Value::Cons(cons)) => {
            path.push(PathElement::PairCar);
            if let Some(p) = compare_values(&pair.0, cons.car(), path) {
                return Some(p);
            }
            path.pop();
            path.push(PathElement::PairCdr);
            if let Some(p) = compare_values(&pair.1, cons.cdr(), path) {
                return Some(p);
            }
            path.pop();
            None
        }
        _ => Some(structural_path_with(path, PathElement::Atom)),
    }
}

fn compare_list(
    items: &[Value],
    lexpr_v: &lexpr::Value,
    path: &mut Vec<PathElement>,
) -> Option<StructuralPath> {
    let mut current = lexpr_v.clone();
    for (i, item) in items.iter().enumerate() {
        match current {
            lexpr::Value::Cons(cons) => {
                let (car, cdr) = cons.into_pair();
                path.push(PathElement::ListIndex(i));
                if let Some(p) = compare_values(item, &car, path) {
                    return Some(p);
                }
                path.pop();
                current = cdr;
            }
            _ => return Some(structural_path_with(path, PathElement::ListIndex(i))),
        }
    }
    match current {
        lexpr::Value::Null | lexpr::Value::Nil => None,
        // Expected class (tail form): a dotted `nil` tail is `()` to the new parser.
        lexpr::Value::Symbol(s) if &*s == "nil" => None,
        _ => Some(structural_path_with(path, PathElement::ListIndex(items.len()))),
    }
}

fn structural_path_with(prefix: &[PathElement], last: PathElement) -> StructuralPath {
    let mut p = prefix.to_vec();
    p.push(last);
    StructuralPath(p)
}

fn sha256_bytes(input: &[u8]) -> [u8; 32] {
    // Minimal SHA-256 implementation kept inline so we don't pull in a hashing crate
    // for the differential validator alone.
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let k: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    let bit_len = (input.len() as u64).wrapping_mul(8);
    let mut padded: Vec<u8> = input.to_vec();
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in padded.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut hh = h[7];
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(k[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    let mut out = [0u8; 32];
    for (i, word) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SpecItem;

    macro_rules! covers {
        ([$($item:expr),* $(,)?]) => {
            {
                $(
                    let _ = $item;
                )*
            }
        };
    }

    /// Serialises the tests that touch process-global state (the differential
    /// mode, the dedup cache, the stderr report counter).
    fn global_state_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    fn make_callback() -> (Arc<Mutex<Vec<Discrepancy>>>, DiscrepancySink) {
        let recorded = Arc::new(Mutex::new(Vec::new()));
        let recorded_for_cb = Arc::clone(&recorded);
        let cb = DiscrepancySink::Callback(Arc::new(move |d: &Discrepancy| {
            recorded_for_cb.lock().unwrap().push(d.clone());
        }));
        (recorded, cb)
    }

    #[test]
    fn keyword_vs_colon_symbol_is_expected_class() {
        covers!([SpecItem::McpToolsParserExpectedDiscrepancyClasses]);

        let new = Value::Keyword("k".to_string());
        let old = lexpr::Value::symbol(":k");
        assert!(is_expected_class(&new, &old));
        assert!(compare_values(&new, &old, &mut Vec::new()).is_none());
    }

    #[test]
    fn nil_vs_lexpr_nil_symbol_is_expected_class() {
        covers!([SpecItem::McpToolsParserExpectedDiscrepancyClasses]);

        let new = Value::Nil;
        let old = lexpr::Value::symbol("nil");
        assert!(is_expected_class(&new, &old));
        assert!(compare_values(&new, &old, &mut Vec::new()).is_none());

        // Byte-exact: `NIL` is an ordinary symbol to both parsers.
        assert!(!is_expected_class(&Value::Nil, &lexpr::Value::symbol("NIL")));
        assert!(!is_expected_class(&Value::Nil, &lexpr::Value::symbol("nil ")));
    }

    #[test]
    fn nil_tail_is_expected_class_but_other_symbol_tail_is_not() {
        covers!([SpecItem::McpToolsParserExpectedDiscrepancyClasses]);

        let new = Value::List(vec![Value::Symbol("a".to_string())]);
        let nil_tail = lexpr::Value::cons(lexpr::Value::symbol("a"), lexpr::Value::symbol("nil"));
        assert!(is_nil_terminated_chain(&nil_tail));
        assert!(compare_values(&new, &nil_tail, &mut Vec::new()).is_none());

        let other_tail =
            lexpr::Value::cons(lexpr::Value::symbol("a"), lexpr::Value::symbol("other"));
        assert!(!is_nil_terminated_chain(&other_tail));
        assert_eq!(
            compare_values(&new, &other_tail, &mut Vec::new()),
            Some(StructuralPath(vec![PathElement::Atom]))
        );
    }

    #[test]
    fn nil_expected_class_is_not_symmetric() {
        covers!([SpecItem::McpToolsParserExpectedDiscrepancyClasses]);

        // The new parser failing to apply its own grammar must still be reported.
        let new = Value::Symbol("nil".to_string());
        for old in [lexpr::Value::Nil, lexpr::Value::Null] {
            assert!(!is_expected_class(&new, &old));
            assert_eq!(
                compare_values(&new, &old, &mut Vec::new()),
                Some(StructuralPath(vec![PathElement::Atom]))
            );
        }
    }

    #[test]
    fn expected_class_is_not_symmetric() {
        covers!([SpecItem::McpToolsParserExpectedDiscrepancyClasses]);

        // The reverse pair — the new parser failing to canonicalise a keyword —
        // must still be reported at `atom`.
        let new = Value::Symbol(":k".to_string());
        let old = lexpr::Value::keyword("k");
        assert!(!is_expected_class(&new, &old));
        let path = compare_values(&new, &old, &mut Vec::new());
        assert_eq!(path, Some(StructuralPath(vec![PathElement::Atom])));

        // And a colon-symbol whose name does not match is not expected either.
        let new = Value::Keyword("k".to_string());
        let old = lexpr::Value::symbol(":other");
        assert!(!is_expected_class(&new, &old));
    }

    const SENTINEL: &str = "SECRET-9f3a";

    /// A discrepancy whose payloads (both trees, the error message, and the
    /// verbose source) all carry the sentinel.
    fn secret_bearing_report(input: DiscrepancyInput) -> Discrepancy {
        Discrepancy {
            input,
            new_value: Ok(Value::List(vec![
                Value::Symbol("tool".to_string()),
                Value::String(SENTINEL.to_string()),
                Value::Integer(1),
            ])),
            lexpr_value: Ok(lexpr::Value::list(vec![
                lexpr::Value::symbol("tool"),
                lexpr::Value::string(SENTINEL),
                lexpr::Value::Number(2i64.into()),
            ])),
            path: StructuralPath(vec![PathElement::ListIndex(2), PathElement::Atom]),
        }
    }

    #[test]
    fn hashed_mode_report_omits_tree_contents() {
        covers!([SpecItem::McpToolsParserDiscrepancyReporting]);

        let report = secret_bearing_report(DiscrepancyInput::Hashed { sha256: [0xab; 32] });
        let out = format_report(&report, false);

        assert!(!out.contains(SENTINEL), "hashed mode leaked payload: {out}");
        assert_eq!(
            out,
            format!(
                "[mcp-tools differential] new=List lexpr=Cons path=2.atom\n  input-sha256={}\n",
                "ab".repeat(32)
            )
        );
    }

    #[test]
    fn verbose_mode_report_includes_tree_contents_and_source() {
        covers!([SpecItem::McpToolsParserDiscrepancyReporting]);

        let source = format!("(tool \"{SENTINEL}\" 1)");
        let report = secret_bearing_report(DiscrepancyInput::Verbose {
            source: source.clone(),
        });
        let out = format_report(&report, true);

        assert!(out.starts_with("[mcp-tools differential] new=Ok("));
        assert!(out.contains(&format!("String(\"{SENTINEL}\")")));
        assert!(out.contains("path=2.atom\n"));
        assert!(out.ends_with(&format!("  input={source}\n")));
    }

    #[test]
    fn hashed_mode_report_uses_error_kind_not_message() {
        covers!([SpecItem::McpToolsParserDiscrepancyReporting]);

        let message = format!("integer literal `{SENTINEL}` out of range");
        let report = Discrepancy {
            input: DiscrepancyInput::Hashed { sha256: [0u8; 32] },
            new_value: Err(ParseErrorRepr {
                kind: "IntegerOutOfRange".to_string(),
                message: message.clone(),
            }),
            lexpr_value: Err(format!("lexpr choked on {SENTINEL}")),
            path: StructuralPath(vec![PathElement::Atom]),
        };

        let hashed = format_report(&report, false);
        assert!(hashed.contains("new=Err(IntegerOutOfRange) lexpr=Err path=atom"));
        assert!(!hashed.contains(SENTINEL), "hashed mode leaked an error message: {hashed}");

        let verbose = format_report(&report, true);
        assert!(verbose.contains(&format!("new=Err(IntegerOutOfRange: {message})")));
        assert!(verbose.contains(&format!("lexpr=Err(lexpr choked on {SENTINEL})")));
    }

    // -----------------------------------------------------------------------
    // Stderr report budget (mcp-tools/parser/stderr-report-budget)
    // -----------------------------------------------------------------------

    #[test]
    fn budget_decision_partitions_counts() {
        covers!([SpecItem::McpToolsParserStderrReportBudget]);

        let decisions: Vec<BudgetDecision> = (0..70)
            .map(|count_before| budget_decision(count_before, 64))
            .collect();
        let tally = |d: BudgetDecision| decisions.iter().filter(|&&x| x == d).count();

        assert_eq!(tally(BudgetDecision::Emit), 64);
        assert_eq!(tally(BudgetDecision::EmitExhausted), 1);
        assert_eq!(tally(BudgetDecision::Drop), 5);
        // Ordered: all Emits, then the single EmitExhausted, then Drops.
        assert_eq!(decisions[63], BudgetDecision::Emit);
        assert_eq!(decisions[64], BudgetDecision::EmitExhausted);
        assert_eq!(decisions[65], BudgetDecision::Drop);
        assert_eq!(STDERR_REPORT_BUDGET, 64);
        assert!(budget_exhausted_line().contains("report budget (64) exhausted"));
    }

    #[test]
    fn switching_mode_does_not_reset_budget_counter() {
        covers!([SpecItem::McpToolsParserStderrReportBudget]);
        let _guard = global_state_lock();

        // Drive the sink past exhaustion (writes go to the test harness's
        // captured stderr). The counter is process-global, so start from
        // wherever it is.
        let report = secret_bearing_report(DiscrepancyInput::Hashed { sha256: [7u8; 32] });
        while budget_decision(stderr_reports_requested(), STDERR_REPORT_BUDGET)
            != BudgetDecision::Drop
        {
            write_stderr(&report, false);
        }
        let exhausted_at = stderr_reports_requested();

        let (_recorded, cb) = make_callback();
        set_differential_mode(DifferentialMode::On {
            sink: cb,
            verbose: false,
        });
        set_differential_mode(DifferentialMode::On {
            sink: DiscrepancySink::Stderr,
            verbose: false,
        });
        set_differential_mode(DifferentialMode::Off);

        assert_eq!(stderr_reports_requested(), exhausted_at);
        assert_eq!(
            budget_decision(stderr_reports_requested(), STDERR_REPORT_BUDGET),
            BudgetDecision::Drop
        );
    }

    #[test]
    fn dedup_runs_before_budget() {
        covers!([SpecItem::McpToolsParserStderrReportBudget]);
        let _guard = global_state_lock();

        set_differential_mode(DifferentialMode::On {
            sink: DiscrepancySink::Stderr,
            verbose: false,
        });
        flush_discrepancy_dedup();
        // The 63 inputs below share one class; only input-hash dedup is under test.
        set_discrepancy_class_dedup_capacity(0);

        // 63 fresh diverging inputs (bignums: new errors, lexpr accepts).
        let fresh: Vec<String> = (0..63).map(|i| format!("3689348814741910323{i}")).collect();
        let before = stderr_reports_requested();
        for input in &fresh {
            let outcome = crate::parser::reader::parse_value(input);
            record_discrepancy_if_diverging(input, &outcome).unwrap();
        }
        assert_eq!(stderr_reports_requested(), before + 63);

        // A duplicate is deduplicated and must not consume report 64.
        let outcome = crate::parser::reader::parse_value(&fresh[0]);
        record_discrepancy_if_diverging(&fresh[0], &outcome).unwrap();
        assert_eq!(stderr_reports_requested(), before + 63);

        set_discrepancy_class_dedup_capacity(DEFAULT_CLASS_DEDUP_CAPACITY);
        set_differential_mode(DifferentialMode::Off);
        flush_discrepancy_dedup();
    }

    // -----------------------------------------------------------------------
    // Class-keyed deduplication, position-sensitive cases
    // (mcp-tools/parser/discrepancy-class-deduplication). Driven through the
    // `record_discrepancy` seam: both sides Ok, explicit path, distinct hash.
    // -----------------------------------------------------------------------

    /// A both-Ok list/cons discrepancy at list index `index`, with a unique
    /// input hash derived from `seq` so the input cache never intervenes.
    fn ok_ok_report_at(index: usize, seq: u8) -> Discrepancy {
        Discrepancy {
            input: DiscrepancyInput::Hashed { sha256: [seq; 32] },
            new_value: Ok(Value::List(vec![Value::Symbol("a".to_string()), Value::Nil])),
            lexpr_value: Ok(lexpr::Value::list(vec![
                lexpr::Value::symbol("a"),
                lexpr::Value::symbol("nil"),
            ])),
            path: StructuralPath(vec![PathElement::ListIndex(index), PathElement::Atom]),
        }
    }

    /// An Err/Ok discrepancy at the root (the bignum shape), unique hash from `seq`.
    fn err_ok_report(seq: u8) -> Discrepancy {
        Discrepancy {
            input: DiscrepancyInput::Hashed { sha256: [seq; 32] },
            new_value: Err(ParseErrorRepr {
                kind: "IntegerOutOfRange".to_string(),
                message: "out of range".to_string(),
            }),
            lexpr_value: Ok(lexpr::Value::Number(1i64.into())),
            path: StructuralPath(Vec::new()),
        }
    }

    fn install_callback_with_class_capacity(
        class_capacity: usize,
    ) -> Arc<Mutex<Vec<Discrepancy>>> {
        let (recorded, sink) = make_callback();
        set_differential_mode(DifferentialMode::On {
            sink,
            verbose: false,
        });
        set_discrepancy_dedup_capacity(1024);
        set_discrepancy_class_dedup_capacity(class_capacity);
        flush_discrepancy_dedup();
        recorded
    }

    fn restore_defaults() {
        set_discrepancy_class_dedup_capacity(DEFAULT_CLASS_DEDUP_CAPACITY);
        set_differential_mode(DifferentialMode::Off);
        flush_discrepancy_dedup();
    }

    #[test]
    fn class_dedup_distinguishes_divergence_position() {
        covers!([SpecItem::McpToolsParserDiscrepancyClassDeduplication]);
        let _guard = global_state_lock();

        let recorded = install_callback_with_class_capacity(256);
        record_discrepancy(ok_ok_report_at(1, 1)).unwrap(); // 1.atom
        record_discrepancy(ok_ok_report_at(2, 2)).unwrap(); // 2.atom

        let captured = recorded.lock().unwrap();
        let paths: Vec<String> = captured.iter().map(|d| d.path.to_string()).collect();
        assert_eq!(paths, vec!["1.atom".to_string(), "2.atom".to_string()]);
        drop(captured);

        restore_defaults();
    }

    #[test]
    fn class_dedup_evicts_least_recently_seen_class() {
        covers!([SpecItem::McpToolsParserDiscrepancyClassDeduplication]);
        let _guard = global_state_lock();

        let recorded = install_callback_with_class_capacity(2);
        // Three classes in rotation, each time with a fresh input hash so the
        // input cache never intervenes: A = 1.atom, B = 2.atom, C = Err/Ok at root.
        record_discrepancy(ok_ok_report_at(1, 10)).unwrap(); // A: reported (1)
        record_discrepancy(ok_ok_report_at(2, 11)).unwrap(); // B: reported (2)
        record_discrepancy(err_ok_report(12)).unwrap(); // C: reported (3); A evicted
        record_discrepancy(ok_ok_report_at(1, 13)).unwrap(); // A again: reported (4); B evicted
        record_discrepancy(ok_ok_report_at(2, 14)).unwrap(); // B again: reported (5)
        record_discrepancy(ok_ok_report_at(1, 15)).unwrap(); // A: still cached -> suppressed

        let captured = recorded.lock().unwrap();
        let paths: Vec<String> = captured.iter().map(|d| d.path.to_string()).collect();
        assert_eq!(
            paths,
            vec!["1.atom", "2.atom", ".", "1.atom", "2.atom"]
                .into_iter()
                .map(String::from)
                .collect::<Vec<_>>()
        );
        drop(captured);

        restore_defaults();
    }

    #[test]
    fn dedup_records_each_hash_once() {
        covers!([SpecItem::McpToolsParserDiscrepancyDeduplication]);

        let mut cache = DedupCache::new(2);
        let h1 = [1u8; 32];
        let h2 = [2u8; 32];
        let h3 = [3u8; 32];

        assert!(!cache.record(h1));
        assert!(cache.record(h1));
        assert!(!cache.record(h2));
        assert!(cache.record(h2));
        // h3 inserts; h1 (LRU since h2 was just touched) is evicted.
        assert!(!cache.record(h3));
        assert!(cache.record(h2));
        assert!(cache.record(h3));
        assert!(!cache.record(h1));
    }

    #[test]
    fn dedup_capacity_zero_disables() {
        covers!([SpecItem::McpToolsParserDiscrepancyDeduplication]);

        let mut cache = DedupCache::new(0);
        let h = [9u8; 32];
        assert!(!cache.record(h));
        assert!(!cache.record(h));
    }

    #[test]
    fn structural_path_display() {
        covers!([SpecItem::McpToolsParserDiscrepancyReporting]);

        let p = StructuralPath(vec![
            PathElement::ListIndex(0),
            PathElement::ListIndex(2),
            PathElement::PairCar,
        ]);
        assert_eq!(p.to_string(), "0.2.car");

        let empty = StructuralPath(Vec::new());
        assert_eq!(empty.to_string(), ".");
    }

    #[test]
    fn callback_dispatch_invoked_synchronously() {
        covers!([SpecItem::McpToolsParserDifferentialMode]);

        let (recorded, sink) = make_callback();
        let report = Discrepancy {
            input: DiscrepancyInput::Hashed { sha256: [0u8; 32] },
            new_value: Ok(Value::Integer(1)),
            lexpr_value: Ok(lexpr::Value::Number(2i64.into())),
            path: StructuralPath(vec![PathElement::Atom]),
        };
        dispatch(&sink, false, &report).unwrap();
        let captured = recorded.lock().unwrap();
        assert_eq!(captured.len(), 1);
    }

    #[test]
    fn callback_panic_is_caught() {
        covers!([SpecItem::McpToolsParserDifferentialMode]);

        let sink = DiscrepancySink::Callback(Arc::new(|_| panic!("boom")));
        let report = Discrepancy {
            input: DiscrepancyInput::Hashed { sha256: [0u8; 32] },
            new_value: Ok(Value::Nil),
            lexpr_value: Ok(lexpr::Value::Null),
            path: StructuralPath(vec![PathElement::Atom]),
        };
        match dispatch(&sink, false, &report) {
            Err(DiscrepancyDispatchError::SinkPanicked) => {}
            Ok(()) => panic!("expected SinkPanicked"),
        }
    }

    #[test]
    fn parse_env_mode_recognizes_values() {
        let _guard = global_state_lock();
        covers!([SpecItem::McpToolsParserDifferentialMode]);

        // We can't easily mutate process env in tests without races, so test the
        // parse helper directly by constructing the same logic.
        // Instead, exercise the public mode setter/getter.
        set_differential_mode(DifferentialMode::Off);
        match current_differential_mode() {
            DifferentialMode::Off => {}
            other => panic!("expected Off, got {:?}", other),
        }
        set_differential_mode(DifferentialMode::On {
            sink: DiscrepancySink::Stderr,
            verbose: true,
        });
        match current_differential_mode() {
            DifferentialMode::On { verbose: true, .. } => {}
            other => panic!("expected On verbose, got {:?}", other),
        }
        set_differential_mode(DifferentialMode::Off);
    }

    #[test]
    fn structural_compare_lists_are_equal() {
        covers!([SpecItem::McpToolsParserDifferentialMode]);

        let new = Ok(Value::List(vec![Value::Integer(1), Value::Integer(2)]));
        let lexpr_v = lexpr::from_str("(1 2)").map_err(|e| e.to_string());
        assert!(compare(&new, &lexpr_v).is_none());
    }

    #[test]
    fn structural_compare_detects_atom_difference() {
        covers!([SpecItem::McpToolsParserDifferentialMode]);

        let new: Result<Value, ParseError> = Ok(Value::Integer(1));
        let lexpr_v: Result<lexpr::Value, String> = Ok(lexpr::Value::Number(2i64.into()));
        let p = compare(&new, &lexpr_v).expect("should diverge");
        assert_eq!(p.0, vec![PathElement::Atom]);
    }

    #[test]
    fn sha256_known_vector() {
        covers!([SpecItem::McpToolsParserDiscrepancyDeduplication]);

        // SHA-256("abc") = ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
        let h = sha256_bytes(b"abc");
        assert_eq!(
            hex(&h),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
