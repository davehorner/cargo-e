use regex::Regex;
use std::cell::RefCell;
use std::fmt;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::Mutex;

use crate::e_cargocommand_ext::CargoStats;
use crate::e_command_builder::TerminalError;

// Consolidated thread-local storage for context and prior response.
thread_local! {
    pub static THREAD_CONTEXT: RefCell<ThreadLocalContext> = RefCell::new(ThreadLocalContext {
        target_name: String::new(),
        manifest_path: String::new(),
    });

    static PRIOR_RESPONSE: RefCell<Option<CallbackResponse>> = RefCell::new(None);
}

/// Context struct for thread-local storage.
#[derive(Debug, Clone)]
pub struct ThreadLocalContext {
    pub target_name: String,
    pub manifest_path: String,
}

impl ThreadLocalContext {
    /// Set the thread-local context.
    pub fn set_context(target_name: &str, manifest_path: &str) {
        log::trace!(
            "Setting thread-local context: target_name={}, manifest_path={}",
            target_name,
            manifest_path
        );
        THREAD_CONTEXT.with(|ctx| {
            let mut context = ctx.borrow_mut();
            context.target_name = target_name.to_string();
            context.manifest_path = manifest_path.to_string();
        });
    }

    /// Get the thread-local context.
    pub fn get_context() -> ThreadLocalContext {
        THREAD_CONTEXT.with(|ctx| ctx.borrow().clone())
    }
}

/// Our internal diagnostic level for cargo.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub enum CargoDiagnosticLevel {
    Error,
    Warning,
    Help,
    Note,
}

// /// A line of source code associated with a diagnostic.
// #[derive(Debug, Clone)]
// pub struct CargoDiagnosticSpanLine {
//     pub text: String,
//     pub highlight_start: usize,
//     pub highlight_end: usize,
// }

// A span (i.e. file location) associated with a diagnostic.
// #[derive(Debug, Clone)]
// pub struct CargoDiagnosticSpan {
//     pub file_name: String,
//     pub line_start: usize,
//     pub line_end: usize,
//     pub column_start: usize,
//     pub column_end: usize,
//     pub is_primary: bool,
//     pub text: Vec<CargoDiagnosticSpanLine>,
//     pub label: Option<String>,
//     pub suggested_replacement: Option<String>,
// }

// /// Our internal diagnostic message.
// #[derive(Debug, Clone)]
// pub struct CargoDiagnostic {
//     pub message: String,
//     pub code: Option<String>,
//     pub level: CargoDiagnosticLevel,
//     pub spans: Vec<CargoDiagnosticSpan>,
//     pub children: Vec<CargoDiagnostic>,
// }

/// Our callback type enum.
#[derive(Debug, Clone)]
pub enum CallbackType {
    LevelMessage,
    Warning,
    Error,
    Help,
    Note,
    Location,
    OpenedUrl,
    Unspecified,
    Suggestion,
}

/// The callback response produced by our event dispatcher.
#[derive(Debug, Clone)]
pub struct CallbackResponse {
    pub callback_type: CallbackType,
    pub message: Option<String>,
    pub file: Option<String>,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub suggestion: Option<String>,
    pub terminal_status: Option<TerminalError>,
}

#[derive(Clone)]
pub struct PatternCallback {
    pub pattern: Regex,
    // pub callback: Arc<dyn Fn(&str) -> Option<CallbackResponse> + Send + Sync>,
    pub callback: Arc<
        dyn Fn(
                &str,
                Option<regex::Captures>,
                Arc<AtomicBool>,
                Arc<Mutex<CargoStats>>,
                Option<CallbackResponse>,
            ) -> Option<CallbackResponse>
            + Send
            + Sync,
    >,
    pub is_reading_multiline: Arc<AtomicBool>,
}

impl fmt::Debug for PatternCallback {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PatternCallback")
            .field("pattern", &self.pattern.as_str())
            .field("callback", &"Closure")
            .finish()
    }
}

impl PatternCallback {
    pub fn new(
        pattern: &str,
        callback: Box<
            dyn Fn(
                    &str,
                    Option<regex::Captures>,
                    Arc<AtomicBool>,
                    Arc<Mutex<CargoStats>>,
                    Option<CallbackResponse>,
                ) -> Option<CallbackResponse>
                + Send
                + Sync,
        >,
    ) -> Self {
        PatternCallback {
            pattern: Regex::new(pattern).expect("Invalid regex"),
            callback: Arc::new(callback),
            is_reading_multiline: Arc::new(AtomicBool::new(false)),
        }
    }
}

/// A simple event dispatcher for output lines.
#[derive(Clone, Debug)]
pub struct EventDispatcher {
    pub callbacks: Arc<Mutex<Vec<PatternCallback>>>,
}

impl EventDispatcher {
    pub fn new() -> Self {
        EventDispatcher {
            callbacks: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Add a new callback with a regex pattern.
    pub fn add_callback(
        &mut self,
        pattern: &str,
        callback: Box<
            dyn Fn(
                    &str,
                    Option<regex::Captures>,
                    Arc<AtomicBool>,
                    Arc<Mutex<CargoStats>>,
                    Option<CallbackResponse>,
                ) -> Option<CallbackResponse>
                + Send
                + Sync,
        >,
    ) {
        if let Ok(mut callbacks) = self.callbacks.lock() {
            callbacks.push(PatternCallback::new(pattern, callback));
        } else {
            eprintln!("Failed to acquire lock on callbacks in add_callback");
        }
    }

    /// Dispatch a line to all callbacks that match, and collect their responses.
    pub fn dispatch(
        &self,
        line: &str,
        stats: Arc<Mutex<CargoStats>>,
    ) -> Vec<Option<CallbackResponse>> {
        let mut responses = Vec::new();
        if let Ok(callbacks) = self.callbacks.lock() {
            for cb in callbacks.iter() {
                let is_reading_multiline = Arc::clone(&cb.is_reading_multiline);
                let prior = PRIOR_RESPONSE.with(|p| p.borrow().clone());
                let response = if is_reading_multiline.load(Ordering::Relaxed) {
                    // Multiline mode: always call with prior_response
                    (cb.callback)(
                        line,
                        None,
                        Arc::clone(&is_reading_multiline),
                        stats.clone(),
                        prior,
                    )
                } else if let Some(captures) = cb.pattern.captures(line) {
                    (cb.callback)(
                        line,
                        Some(captures),
                        Arc::clone(&is_reading_multiline),
                        stats.clone(),
                        None,
                    )
                } else if cb.pattern.is_match(line) {
                    (cb.callback)(
                        line,
                        None,
                        Arc::clone(&is_reading_multiline),
                        stats.clone(),
                        None,
                    )
                } else {
                    None
                };
                if is_reading_multiline.load(Ordering::Relaxed) {
                    PRIOR_RESPONSE.with(|p| *p.borrow_mut() = response.clone());
                }
                responses.push(response);
            }
        } else {
            eprintln!("Failed to acquire lock on callbacks in dispatch");
        }
        responses
    }

    /// Process all lines from a BufRead, dispatching to callbacks.
    pub fn process_stream<R: std::io::BufRead>(
        &self,
        reader: R,
        stats: Arc<Mutex<CargoStats>>,
    ) -> Vec<CallbackResponse> {
        let mut responses = Vec::new();
        for line in reader.lines() {
            if let Ok(line) = line {
                let res = self.dispatch(&line, Arc::clone(&stats));
                for r in res {
                    if let Some(cb) = r {
                        responses.push(cb);
                    }
                }
            }
        }
        responses
    }
}
