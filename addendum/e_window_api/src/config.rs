/// Configuration for launching an e_window instance
#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
    pub auto_center: bool,  // New field to enable auto-centering
    pub input_file: Option<String>,
    pub follow_hwnd: Option<usize>,
    pub parent_pid: Option<u32>,
    pub pool_count: Option<usize>,
    pub pool_index: Option<usize>,
    pub pool_rate: Option<u64>,
    pub decode_debug: bool,
    pub additional_args: Vec<String>,
    pub initial_input: Option<String>,  // Initial input data to prevent default card usage
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "E Window".to_string(),
            width: 0,  // Let egui determine size based on content
            height: 0, // Let egui determine size based on content
            x: 0,      // Will be auto-centered unless specified
            y: 0,      // Will be auto-centered unless specified
            auto_center: true,  // Enable auto-centering by default
            input_file: None,
            follow_hwnd: None,
            parent_pid: None,
            pool_count: None,
            pool_index: None,
            pool_rate: None,
            decode_debug: false,
            additional_args: Vec::new(),
            initial_input: None,  // No initial input by default
        }
    }
}

impl WindowConfig {
    /// Create a new window configuration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the window title
    pub fn title<S: Into<String>>(mut self, title: S) -> Self {
        self.title = title.into();
        self
    }

    /// Set the window size
    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Set the window position
    pub fn position(mut self, x: i32, y: i32) -> Self {
        self.x = x;
        self.y = y;
        self.auto_center = false;  // Disable auto-centering when position is explicitly set
        self
    }

    /// Set the input file
    pub fn input_file<S: Into<String>>(mut self, file: S) -> Self {
        self.input_file = Some(file.into());
        self
    }

    /// Set the window to follow another window by HWND
    pub fn follow_hwnd(mut self, hwnd: usize) -> Self {
        self.follow_hwnd = Some(hwnd);
        self
    }

    /// Set the parent process ID
    pub fn parent_pid(mut self, pid: u32) -> Self {
        self.parent_pid = Some(pid);
        self
    }

    /// Set pool configuration
    pub fn pool_config(mut self, count: usize, index: usize, rate: u64) -> Self {
        self.pool_count = Some(count);
        self.pool_index = Some(index);
        self.pool_rate = Some(rate);
        self
    }

    /// Enable debug decoding
    pub fn decode_debug(mut self, enabled: bool) -> Self {
        self.decode_debug = enabled;
        self
    }

    /// Add additional command line arguments
    pub fn add_arg<S: Into<String>>(mut self, arg: S) -> Self {
        self.additional_args.push(arg.into());
        self
    }

    /// Convert to command line arguments for e_window
    pub fn to_args(&self) -> Vec<String> {
        let mut args = Vec::new();

        // Only add arguments if auto_center is disabled or values are explicitly set
        if !self.auto_center {
            // When auto_center is disabled, send all arguments
            args.push("--title".to_string());
            args.push(self.title.clone());

            args.push("--width".to_string());
            args.push(self.width.to_string());

            args.push("--height".to_string());
            args.push(self.height.to_string());

            args.push("--x".to_string());
            args.push(self.x.to_string());

            args.push("--y".to_string());
            args.push(self.y.to_string());
        } else {
            // When auto_center is enabled, send title if different from default OR if we have initial input
            if self.title != "E Window" || self.initial_input.is_some() {
                args.push("--title".to_string());
                args.push(self.title.clone());
            }
        }

        if let Some(ref file) = self.input_file {
            args.push("--input-file".to_string());
            args.push(file.clone());
        }

        if let Some(hwnd) = self.follow_hwnd {
            args.push("--follow-hwnd".to_string());
            args.push(hwnd.to_string());
        }

        if let Some(pid) = self.parent_pid {
            args.push("--parent-pid".to_string());
            args.push(pid.to_string());
        }

        if let Some(count) = self.pool_count {
            args.push("--w-pool-cnt".to_string());
            args.push(count.to_string());
        }

        if let Some(index) = self.pool_index {
            args.push("--w-pool-ndx".to_string());
            args.push(index.to_string());
        }

        if let Some(rate) = self.pool_rate {
            args.push("--w-pool-rate".to_string());
            args.push(rate.to_string());
        }

        if self.decode_debug {
            args.push("--decode-debug".to_string());
        }

        args.extend(self.additional_args.clone());

        eprintln!("[DEBUG] WindowConfig::to_args() generated args: {:?}", args);
        args
    }
}