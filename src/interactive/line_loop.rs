use anyhow::{Context, Result};
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::future::Future;
use std::path::{Path, PathBuf};

/// Control flow for the line loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopControl {
    /// Continue the loop
    Continue,
    /// Break out of the loop
    Break,
}

/// Type of history file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryKind {
    /// REPL history
    Repl,
    /// Log viewer history
    LogViewer,
}

/// Get the default history file path for a given history kind.
///
/// Returns generic history file names. Applications should customize these
/// paths for their specific use case.
pub fn default_history_path(kind: HistoryKind) -> PathBuf {
    match kind {
        HistoryKind::Repl => PathBuf::from(".mcp-repl.history"),
        HistoryKind::LogViewer => PathBuf::from(".mcp-log-viewer.history"),
    }
}

/// Configuration for the interactive line loop.
pub struct LineLoopConfig<'a> {
    /// Function to generate the prompt string
    pub prompt: Box<dyn FnMut() -> String + 'a>,
    /// Whether to add lines to history
    pub add_history: bool,
    /// Optional history file path
    pub history_file: Option<PathBuf>,
    /// Handler for Ctrl-C interrupt
    pub on_interrupt: Box<dyn FnMut() -> LoopControl + 'a>,
    /// Handler for EOF
    pub on_eof: Box<dyn FnMut() -> LoopControl + 'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReadlineErrorKind {
    Interrupted,
    Eof,
    Other,
}

fn classify_readline_error(e: &ReadlineError) -> ReadlineErrorKind {
    match e {
        ReadlineError::Interrupted => ReadlineErrorKind::Interrupted,
        ReadlineError::Eof => ReadlineErrorKind::Eof,
        _ => ReadlineErrorKind::Other,
    }
}

impl<'a> LineLoopConfig<'a> {
    /// Create a new line loop configuration.
    pub fn new(
        prompt: impl FnMut() -> String + 'a,
        add_history: bool,
        on_interrupt: impl FnMut() -> LoopControl + 'a,
        on_eof: impl FnMut() -> LoopControl + 'a,
    ) -> Self {
        Self {
            prompt: Box::new(prompt),
            add_history,
            history_file: None,
            on_interrupt: Box::new(on_interrupt),
            on_eof: Box::new(on_eof),
        }
    }

    /// Set the history file path.
    pub fn with_history_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.history_file = Some(path.into());
        self
    }
}

fn try_load_history(editor: &mut DefaultEditor, path: &Path) {
    let _ = editor.load_history(path);
}

fn try_save_history(editor: &mut DefaultEditor, path: &Path) {
    let _ = editor.save_history(path);
}

fn read_next_nonempty_line<'a>(
    editor: &mut DefaultEditor,
    cfg: &mut LineLoopConfig<'a>,
) -> Result<Option<String>> {
    loop {
        let prompt = (cfg.prompt)();
        let line = match editor.readline(&prompt) {
            Ok(l) => l,
            Err(e) => match classify_readline_error(&e) {
                ReadlineErrorKind::Interrupted => {
                    if matches!((cfg.on_interrupt)(), LoopControl::Break) {
                        return Ok(None);
                    }
                    continue;
                }
                ReadlineErrorKind::Eof => {
                    if matches!((cfg.on_eof)(), LoopControl::Break) {
                        return Ok(None);
                    }
                    continue;
                }
                ReadlineErrorKind::Other => return Err(e).context("Readline error"),
            },
        };

        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }

        if cfg.add_history {
            let _ = editor.add_history_entry(&line);
            if let Some(path) = cfg.history_file.as_deref() {
                try_save_history(editor, path);
            }
        }

        return Ok(Some(line));
    }
}

/// Run a synchronous interactive line loop.
pub fn run_line_loop<'a, F>(mut cfg: LineLoopConfig<'a>, mut on_line: F) -> Result<()>
where
    F: FnMut(&str) -> Result<LoopControl> + 'a,
{
    let mut editor = DefaultEditor::new().context("Failed to initialize line editor")?;

    if cfg.add_history {
        if let Some(path) = cfg.history_file.as_deref() {
            try_load_history(&mut editor, path);
        }
    }

    while let Some(line) = read_next_nonempty_line(&mut editor, &mut cfg)? {
        match on_line(&line)? {
            LoopControl::Continue => {}
            LoopControl::Break => break,
        }
    }

    Ok(())
}

/// Run an asynchronous interactive line loop.
pub async fn run_line_loop_async<'a, F, Fut>(
    mut cfg: LineLoopConfig<'a>,
    mut on_line: F,
) -> Result<()>
where
    F: FnMut(String) -> Fut + 'a,
    Fut: Future<Output = Result<LoopControl>> + 'a,
{
    let mut editor = DefaultEditor::new().context("Failed to initialize line editor")?;

    if cfg.add_history {
        if let Some(path) = cfg.history_file.as_deref() {
            try_load_history(&mut editor, path);
        }
    }

    while let Some(line) = read_next_nonempty_line(&mut editor, &mut cfg)? {
        match on_line(line).await? {
            LoopControl::Continue => {}
            LoopControl::Break => break,
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustyline::history::History;
    use tempfile::tempdir;

    #[test]
    fn test_classify_readline_error_interrupted() {
        assert_eq!(
            classify_readline_error(&ReadlineError::Interrupted),
            ReadlineErrorKind::Interrupted
        );
    }

    #[test]
    fn test_classify_readline_error_eof() {
        assert_eq!(
            classify_readline_error(&ReadlineError::Eof),
            ReadlineErrorKind::Eof
        );
    }

    #[test]
    fn test_classify_readline_error_other() {
        let e = ReadlineError::Io(std::io::Error::new(std::io::ErrorKind::Other, "x"));
        assert_eq!(classify_readline_error(&e), ReadlineErrorKind::Other);
    }

    #[test]
    fn test_history_persistence() {
        let dir = tempdir().unwrap();
        let history_file = dir.path().join("history.txt");

        let mut editor = DefaultEditor::new().unwrap();
        try_load_history(&mut editor, &history_file);
        let _ = editor.add_history_entry("line1");
        let _ = editor.add_history_entry("line2");
        try_save_history(&mut editor, &history_file);

        let mut editor2 = DefaultEditor::new().unwrap();
        try_load_history(&mut editor2, &history_file);
        assert_eq!(editor2.history().len(), 2);
    }
}
