    use regex::Regex;
    use std::fmt;
    use std::sync::Arc;
use std::sync::Mutex; 
    /// A pattern-callback pair.
//   #[derive(Clone)]
//     pub struct PatternCallback {
//         pub pattern: Regex,
//         pub callback: Arc<dyn Fn(&str) + Send + Sync>,
//     }
 
//     impl fmt::Debug for PatternCallback {
//         fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//             f.debug_struct("PatternCallback")
//                 .field("pattern", &self.pattern.as_str())
//                 .field("callback", &"Closure")
//                 .finish()
//         }
//     }
//     impl PatternCallback {
//         pub fn new(pattern: &str, callback: Box<dyn Fn(&str) + Send + Sync>) -> Self {
//             PatternCallback {
//                 pattern: Regex::new(pattern).expect("Invalid regex"),
//                 callback: Arc::new(|_s: &str| {})
//             }
//         }
//     }
 
//     /// A simple event dispatcher for output lines.
//   #[derive(Clone, Debug)]
//     pub struct EventDispatcher {
//         pub callbacks: Arc<Mutex<Vec<PatternCallback>>>,
//     }

 
//     impl EventDispatcher {
//         pub fn new() -> Self {
//             EventDispatcher {
//                 callbacks: Arc::new(Mutex::new(Vec::new()))
//             }
//         }
 
//         /// Add a new callback with a regex pattern.
//         pub fn add_callback(&mut self, pattern: &str, callback: Box<dyn Fn(&str) + Send + Sync>) {
//                     let mut callbacks = self.callbacks.lock().unwrap();
//         callbacks.push(PatternCallback::new(pattern, callback));
//         }
 
//         /// Dispatch a line to all callbacks that match.
//         pub fn dispatch(&self, line: &str) {
//                     let callbacks = self.callbacks.lock().unwrap();
//         for cb in callbacks.iter() {
//             if cb.pattern.is_match(line) {
//                 (cb.callback)(line);
//             }
//         }
//         }
//     }



#[derive(Debug)]
pub struct CallbackResponse {
    pub number: usize,
    pub message: Option<String>,
}

#[derive(Clone)]
pub struct PatternCallback {
    pub pattern: Regex,
    // pub callback: Arc<dyn Fn(&str) -> Option<CallbackResponse> + Send + Sync>,
    pub callback: Arc<dyn Fn(&str, Option<regex::Captures>) -> Option<CallbackResponse> + Send + Sync>,
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
    pub fn new(pattern: &str, callback: Box<dyn Fn(&str, Option<regex::Captures>) -> Option<CallbackResponse> + Send + Sync>) -> Self {
        PatternCallback {
            pattern: Regex::new(pattern).expect("Invalid regex"),
            callback: Arc::new(callback),
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
    pub fn add_callback(&mut self, pattern: &str, callback: Box<dyn Fn(&str, Option<regex::Captures>) -> Option<CallbackResponse> + Send + Sync>) {
        let mut callbacks = self.callbacks.lock().unwrap();
        callbacks.push(PatternCallback::new(pattern, callback));
    }

    /// Dispatch a line to all callbacks that match, and collect their responses.
    pub fn dispatch(&self, line: &str) -> Vec<Option<CallbackResponse>> {
        let callbacks = self.callbacks.lock().unwrap();
        let mut responses = Vec::new();
        for cb in callbacks.iter() {
                    // Check if there are any captures for this pattern
        if let Some(captures) = cb.pattern.captures(line) {
            // Pass the captures (or None) to the callback
            let response = (cb.callback)(line, Some(captures));
            responses.push(response);
        } else if cb.pattern.is_match(line) {
            // If there are no captures but there's a match, pass None to the callback
            let response = (cb.callback)(line, None);
            responses.push(response);
        }
        }
        responses
    }
}
