use crate::{Card, ControlCommand, EWindowError, Result, WindowConfig};
use std::io::{BufWriter, Write};
use std::process::{Child, Command, Stdio};
use tokio::time::{sleep, Duration};

/// Get the primary screen dimensions
/// Returns (width, height) of the primary screen
pub fn get_screen_size() -> (u32, u32) {
    #[cfg(target_os = "windows")]
    {
        use winapi::um::winuser::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

        let width = unsafe { GetSystemMetrics(SM_CXSCREEN) } as u32;
        let height = unsafe { GetSystemMetrics(SM_CYSCREEN) } as u32;
        (width, height)
    }

    #[cfg(target_os = "macos")]
    {
        unsafe {
            use core_graphics::display::{CGDisplayBounds, CGMainDisplayID};

            let display_id = CGMainDisplayID();
            let bounds = unsafe { CGDisplayBounds(display_id) };
            let width = bounds.size.width as u32;
            let height = bounds.size.height as u32;
            (width, height)
        }
    }

    #[cfg(target_os = "linux")]
    {
        // Linux screen detection using X11
        use std::ptr;
        use x11::xlib::*;

        unsafe {
            let display = XOpenDisplay(ptr::null());
            if !display.is_null() {
                let screen = XDefaultScreen(display);
                let width = XDisplayWidth(display, screen) as u32;
                let height = XDisplayHeight(display, screen) as u32;
                XCloseDisplay(display);
                (width, height)
            } else {
                // Fallback if X11 display fails
                (1920, 1080)
            }
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        // Fallback for unsupported platforms
        (1920, 1080)
    }
}

/// A handle to a running e_window instance
pub struct EWindow {
    process: Child,
    stdin: BufWriter<std::process::ChildStdin>,
    /// Current window size (width, height) as reported by e_window
    pub current_size: Option<(u32, u32)>,
    /// Current window position (x, y) as reported by e_window
    pub current_position: Option<(i32, i32)>,
    /// Channel to receive window info updates
    window_info_rx: tokio::sync::mpsc::UnboundedReceiver<(u32, u32, i32, i32)>,
    /// Channel to receive result messages from e_window
    result_rx: tokio::sync::mpsc::UnboundedReceiver<String>,
}

impl EWindow {
    /// Launch a new e_window instance with the given configuration
    pub async fn launch(config: WindowConfig) -> Result<Self> {
        // Use the which crate to locate the e_window executable
        let e_window_path =
            which::which("e_window").map_err(|_| EWindowError::ExecutableNotFound)?;

        let mut cmd = Command::new(e_window_path);

        // Add configuration arguments
        let config_args = config.to_args();
        cmd.args(&config_args);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped()); // Capture stdout to read results
        cmd.stderr(Stdio::piped()); // Capture stderr to listen for ready signal

        eprintln!(
            "[DEBUG] Spawning e_window command: {:?} with args: {:?}",
            cmd.get_program(),
            cmd.get_args().collect::<Vec<_>>()
        );

        // Spawn the process
        let mut process = cmd.spawn().map_err(EWindowError::ProcessSpawnError)?;

        // Get stdin handle
        let stdin = process
            .stdin
            .take()
            .ok_or(EWindowError::CommunicationError)?;
        let mut stdin = BufWriter::new(stdin);

        // Get stderr handle to listen for ready signal
        let stderr = process
            .stderr
            .take()
            .ok_or(EWindowError::CommunicationError)?;

        // Get stdout handle to read results
        let stdout = process
            .stdout
            .take()
            .ok_or(EWindowError::CommunicationError)?;

        println!("[DEBUG] e_window process spawned, waiting for READY signal...");

        // Spawn a task to read stderr and wait for ready signal
        let (ready_tx, ready_rx) =
            tokio::sync::oneshot::channel::<std::result::Result<(), String>>();
        let (window_info_tx, window_info_rx) =
            tokio::sync::mpsc::unbounded_channel::<(u32, u32, i32, i32)>();
        let (result_tx, result_rx) = tokio::sync::mpsc::unbounded_channel::<String>();

        let _stderr_task = tokio::task::spawn_blocking(move || {
            use std::io::{BufRead, BufReader};
            let mut stderr_reader = BufReader::new(stderr);
            let mut line = String::new();
            let mut ready_tx = Some(ready_tx);

            loop {
                line.clear();
                match stderr_reader.read_line(&mut line) {
                    Ok(0) => {
                        // EOF reached, process ended
                        if let Some(tx) = ready_tx.take() {
                            let _ = tx.send(Err(
                                "e_window process ended before ready signal".to_string()
                            ));
                        }
                        break;
                    }
                    Ok(_) => {
                        // Check if this line contains the ready signal
                        if line.contains("[e_window] READY") {
                            if let Some(tx) = ready_tx.take() {
                                let _ = tx.send(Ok(()));
                                println!("[DEBUG] Received READY signal from e_window");
                            }
                        }

                        // Check for WINDOW_INFO and parse it
                        if line.contains("[e_window] WINDOW_INFO") {
                            // Parse: [e_window] WINDOW_INFO size=350x120 pos=0x0
                            if let Some(size_start) = line.find("size=") {
                                if let Some(pos_start) = line.find("pos=") {
                                    let size_part = &line[size_start + 5..];
                                    let pos_part = &line[pos_start + 4..];

                                    if let Some(size_end) = size_part.find(' ') {
                                        let size_str = &size_part[..size_end];
                                        let pos_str = pos_part.trim();

                                        // Parse size: "350x120"
                                        if let Some(x_pos) = size_str.find('x') {
                                            let width_str = &size_str[..x_pos];
                                            let height_str = &size_str[x_pos + 1..];

                                            // Parse position: "0x0"
                                            if let Some(x_pos_pos) = pos_str.find('x') {
                                                let x_str = &pos_str[..x_pos_pos];
                                                let y_str = &pos_str[x_pos_pos + 1..];

                                                if let (Ok(width), Ok(height), Ok(x), Ok(y)) = (
                                                    width_str.parse::<u32>(),
                                                    height_str.parse::<u32>(),
                                                    x_str.parse::<i32>(),
                                                    y_str.parse::<i32>(),
                                                ) {
                                                    let _ =
                                                        window_info_tx.send((width, height, x, y));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Continue printing stderr output for debugging
                        print!("[e_window stderr] {}", line);
                    }
                    Err(e) => {
                        if let Some(tx) = ready_tx.take() {
                            let _ = tx.send(Err(format!("Failed to read e_window stderr: {}", e)));
                        }
                        break;
                    }
                }
            }
        });

        // Task to read result messages from stdout
        let result_tx_clone = result_tx.clone();
        let _stdout_task = tokio::task::spawn_blocking(move || {
            use std::io::{BufRead, BufReader};
            let mut stdout_reader = BufReader::new(stdout);
            let mut line = String::new();
            let mut last_result: Option<String> = None;

            loop {
                line.clear();
                match stdout_reader.read_line(&mut line) {
                    Ok(0) => {
                        // EOF reached, process ended
                        // If we captured a result but haven't sent it yet, send it now
                        if let Some(result) = last_result.take() {
                            let _ = result_tx_clone.send(result);
                        }
                        break;
                    }
                    Ok(_) => {
                        // Check for result messages
                        if line.starts_with("RESULT:") {
                            let result = line
                                .strip_prefix("RESULT:")
                                .unwrap_or(&line)
                                .trim()
                                .to_string();
                            last_result = Some(result.clone());
                            // Send immediately but also keep a copy in case process exits quickly
                            let _ = result_tx_clone.send(result);
                        }

                        // Continue printing stdout output for debugging
                        print!("[e_window stdout] {}", line);
                    }
                    Err(_) => {
                        // Error reading, but still try to send last result if we have one
                        if let Some(result) = last_result.take() {
                            let _ = result_tx_clone.send(result);
                        }
                        break;
                    }
                }
            }
        });

        // Wait for the ready signal with timeout
        match tokio::time::timeout(tokio::time::Duration::from_secs(35), ready_rx).await {
            Ok(Ok(Ok(()))) => {
                println!("[DEBUG] e_window is ready to accept commands");
            }
            Ok(Ok(Err(e))) => {
                return Err(EWindowError::ConfigError(format!(
                    "e_window ready signal error: {}",
                    e
                )));
            }
            Ok(Err(_)) => {
                return Err(EWindowError::ConfigError(
                    "Ready signal channel closed".to_string(),
                ));
            }
            Err(_) => {
                return Err(EWindowError::ConfigError(
                    "Timeout waiting for e_window ready signal".to_string(),
                ));
            }
        }

        // Send initial input if provided to prevent default card template usage
        if let Some(ref initial_input) = config.initial_input {
            println!("[DEBUG] Sending initial input to prevent default card template");
            stdin
                .write_all(initial_input.as_bytes())
                .map_err(|_| EWindowError::CommunicationError)?;
            stdin
                .flush()
                .map_err(|_| EWindowError::CommunicationError)?;
        }

        // Don't await the stderr_task - let it continue running in the background to consume stderr
        // This prevents the e_window process from blocking on stderr writes

        let window = Self {
            process,
            stdin,
            current_size: None,
            current_position: None,
            window_info_rx,
            result_rx,
        };
        Ok(window)
    }

    /// Launch with default configuration
    pub async fn launch_default() -> Result<Self> {
        Self::launch(WindowConfig::default()).await
    }

    /// Launch window and immediately show a card
    pub async fn launch_with_card(config: WindowConfig, card: Card) -> Result<Self> {
        let mut window = Self::launch(config).await?;
        window.show_card(card).await?;
        Ok(window)
    }

    /// Send a control command to the window
    pub async fn send_command(&mut self, command: ControlCommand) -> Result<()> {
        let cmd_str = format!("{}\n", command.to_control_string());
        self.stdin
            .write_all(cmd_str.as_bytes())
            .map_err(|_| EWindowError::CommunicationError)?;
        self.stdin
            .flush()
            .map_err(|_| EWindowError::CommunicationError)?;
        Ok(())
    }

    /// Send multiple commands in sequence (useful for building command queues)
    pub async fn send_commands(&mut self, commands: Vec<ControlCommand>) -> Result<()> {
        for command in commands {
            let cmd_str = format!("{}\n", command.to_control_string());
            self.stdin
                .write_all(cmd_str.as_bytes())
                .map_err(|_| EWindowError::CommunicationError)?;
        }
        self.stdin
            .flush()
            .map_err(|_| EWindowError::CommunicationError)?;
        Ok(())
    }

    /// Show a card in the window
    pub async fn show_card(&mut self, card: Card) -> Result<()> {
        self.send_command(ControlCommand::BeginDocument).await?;

        let document = card.to_document();
        for line in document.lines() {
            // Send all lines, including empty ones (they're important for e_window protocol)
            self.stdin
                .write_all(line.as_bytes())
                .map_err(|_| EWindowError::CommunicationError)?;
            self.stdin
                .write_all(b"\n")
                .map_err(|_| EWindowError::CommunicationError)?;
        }

        self.stdin
            .flush()
            .map_err(|_| EWindowError::CommunicationError)?;

        self.send_command(ControlCommand::EndDocument).await?;
        Ok(())
    }

    /// Send raw document content (for backward compatibility)
    pub async fn send_document(&mut self, content: &str) -> Result<()> {
        self.stdin
            .write_all(content.as_bytes())
            .map_err(|_| EWindowError::CommunicationError)?;
        self.stdin
            .flush()
            .map_err(|_| EWindowError::CommunicationError)?;
        Ok(())
    }

    /// Set window title
    pub async fn set_title(&mut self, title: &str) -> Result<()> {
        self.send_command(ControlCommand::SetTitle(title.to_string()))
            .await
    }

    /// Set window position and size immediately
    pub async fn set_rect(&mut self, x: i32, y: i32, width: u32, height: u32) -> Result<()> {
        self.send_command(ControlCommand::SetRect {
            x,
            y,
            width,
            height,
        })
        .await
    }

    /// Wait for window info update and return actual size and position
    /// Useful for getting the real window dimensions after content is loaded
    pub async fn wait_for_window_info(&mut self, timeout_ms: u64) -> Result<(u32, u32, i32, i32)> {
        let start_time = std::time::Instant::now();
        let timeout_duration = Duration::from_millis(timeout_ms);

        // Keep getting messages until timeout, return the latest one
        let mut latest_info = None;

        loop {
            let remaining_time = timeout_duration.saturating_sub(start_time.elapsed());
            if remaining_time.is_zero() {
                break;
            }

            match tokio::time::timeout(
                std::cmp::min(remaining_time, Duration::from_millis(100)),
                self.window_info_rx.recv(),
            )
            .await
            {
                Ok(Some((width, height, x, y))) => {
                    latest_info = Some((width, height, x, y));
                    // Continue to drain any additional messages for a short time
                }
                Ok(None) => return Err(EWindowError::CommunicationError),
                Err(_) => break, // Timeout on this iteration
            }
        }

        if let Some((width, height, x, y)) = latest_info {
            self.current_size = Some((width, height));
            self.current_position = Some((x, y));
            Ok((width, height, x, y))
        } else {
            Err(EWindowError::ConfigError(
                "Timeout waiting for window info".to_string(),
            ))
        }
    }

    /// Get the current known window size, if available
    pub fn current_size(&self) -> Option<(u32, u32)> {
        self.current_size
    }

    /// Get the current known window position, if available  
    pub fn current_position(&self) -> Option<(i32, i32)> {
        self.current_position
    }

    /// Set window position and size with animation
    pub async fn set_rect_animated(
        &mut self,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        duration_ms: u32,
        easing: crate::AnimationEasing,
    ) -> Result<()> {
        self.send_command(ControlCommand::SetRectEased {
            x,
            y,
            width,
            height,
            duration_ms,
            easing,
        })
        .await
    }

    /// Wait for a specified duration
    pub async fn delay(&mut self, ms: u32) -> Result<()> {
        self.send_command(ControlCommand::Delay(ms)).await
    }

    /// Show a card with automatic expiration field for auto-close functionality
    /// This method automatically adds an expiration field showing countdown until auto-close
    pub async fn show_card_with_auto_close(
        &mut self,
        mut card: Card,
        auto_close_seconds: u32,
    ) -> Result<()> {
        // Automatically add expiration field for the auto-close countdown
        card = card.expiration("auto_close", auto_close_seconds as u64);

        self.show_card(card).await?;

        // Start auto-close timer
        let close_duration = Duration::from_secs(auto_close_seconds as u64);
        tokio::spawn(async move {
            sleep(close_duration).await;
            // Note: We can't close the window from here since we'd need to move `self`
            // The caller should handle the actual closing after this duration
        });

        Ok(())
    }

    /// Launch window with a card that will auto-close after specified seconds
    /// This automatically adds an expiration field and handles the closing
    pub async fn launch_with_auto_close(
        config: WindowConfig,
        card: Card,
        auto_close_seconds: u32,
    ) -> Result<()> {
        let mut window = Self::launch(config).await?;
        window
            .show_card_with_auto_close(card, auto_close_seconds)
            .await?;

        // Wait for the specified duration
        sleep(Duration::from_secs(auto_close_seconds as u64)).await;

        // Close the window
        window.close().await?;

        Ok(())
    }

    /// Close the window
    pub async fn close(mut self) -> Result<()> {
        self.send_command(ControlCommand::Exit).await?;

        // Wait a bit for the process to exit gracefully
        sleep(Duration::from_millis(500)).await;

        // Force kill if still running
        if let Ok(None) = self.process.try_wait() {
            let _ = self.process.kill();
        }

        Ok(())
    }

    /// Check if the process is still running
    pub fn is_running(&mut self) -> bool {
        match self.process.try_wait() {
            Ok(None) => true, // Still running
            _ => false,       // Exited or error
        }
    }

    /// Wait for the process to exit
    pub async fn wait(&mut self) -> Result<std::process::ExitStatus> {
        loop {
            match self.process.try_wait() {
                Ok(Some(status)) => return Ok(status),
                Ok(None) => {
                    sleep(Duration::from_millis(100)).await;
                }
                Err(e) => return Err(EWindowError::ProcessSpawnError(e)),
            }
        }
    }

    /// Wait for a result message from the e_window process
    pub async fn wait_for_result(&mut self, timeout_ms: u64) -> Result<String> {
        use tokio::time::{timeout, Duration};

        // Race between receiving a result and process exit + timeout
        match timeout(Duration::from_millis(timeout_ms), async {
            loop {
                tokio::select! {
                    // Try to receive a result
                    result = self.result_rx.recv() => {
                        match result {
                            Some(result) => return Ok(result),
                            None => return Err(EWindowError::CommunicationError),
                        }
                    }

                    // Check if process has exited every 10ms
                    _ = tokio::time::sleep(Duration::from_millis(10)) => {
                        match self.process.try_wait() {
                            Ok(Some(_exit_status)) => {
                                // Process has exited, try one more time to get a result
                                match self.result_rx.try_recv() {
                                    Ok(result) => return Ok(result),
                                    Err(_) => {
                                        // Give it a tiny bit more time for late messages
                                        tokio::time::sleep(Duration::from_millis(50)).await;
                                        match self.result_rx.try_recv() {
                                            Ok(result) => return Ok(result),
                                            Err(_) => return Err(EWindowError::CommunicationError),
                                        }
                                    }
                                }
                            }
                            Ok(None) => {
                                // Process is still running, continue loop
                                continue;
                            }
                            Err(_) => {
                                // Error checking process status, assume it's dead
                                match self.result_rx.try_recv() {
                                    Ok(result) => return Ok(result),
                                    Err(_) => return Err(EWindowError::CommunicationError),
                                }
                            }
                        }
                    }
                }
            }
        })
        .await
        {
            Ok(result) => result,
            Err(_) => Err(EWindowError::ConfigError(
                "Timeout waiting for result".to_string(),
            )),
        }
    }
}

impl Drop for EWindow {
    fn drop(&mut self) {
        // Try to close gracefully
        let _ = self.stdin.write_all(b"!control: exit\n");
        let _ = self.stdin.flush();

        // Give it a moment, then force kill
        std::thread::sleep(std::time::Duration::from_millis(100));
        let _ = self.process.kill();
    }
}
