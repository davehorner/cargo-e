    use regex::Regex;
    use std::fmt;
    use std::sync::Arc;
use std::sync::Mutex; 
    /// A pattern-callback pair.
  #[derive(Clone)]
    pub struct PatternCallback {
        pub pattern: Regex,
        pub callback: Arc<dyn Fn(&str) + Send + Sync>,
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
        pub fn new(pattern: &str, callback: Box<dyn Fn(&str) + Send + Sync>) -> Self {
            PatternCallback {
                pattern: Regex::new(pattern).expect("Invalid regex"),
                callback: Arc::new(|_s: &str| {})
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
                callbacks: Arc::new(Mutex::new(Vec::new()))
            }
        }
 
        /// Add a new callback with a regex pattern.
        pub fn add_callback(&mut self, pattern: &str, callback: Box<dyn Fn(&str) + Send + Sync>) {
                    let mut callbacks = self.callbacks.lock().unwrap();
        callbacks.push(PatternCallback::new(pattern, callback));
        }
 
        /// Dispatch a line to all callbacks that match.
        pub fn dispatch(&self, line: &str) {
                    let callbacks = self.callbacks.lock().unwrap();
        for cb in callbacks.iter() {
            if cb.pattern.is_match(line) {
                (cb.callback)(line);
            }
        }
        }
    }