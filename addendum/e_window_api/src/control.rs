/// Control commands that can be sent to an e_window instance
#[derive(Debug, Clone)]
pub enum ControlCommand {
    /// Exit the window
    Exit,
    
    /// Set window position and size immediately
    SetRect {
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    },
    
    /// Set window position and size with animation
    SetRectEased {
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        duration_ms: u32,
        easing: AnimationEasing,
    },
    
    /// Set window title
    SetTitle(String),
    
    /// Begin a new document
    BeginDocument,
    
    /// End the current document
    EndDocument,
    
    /// Add content to the current document
    Content(String),
    
    /// Wait for a specified duration
    Delay(u32),
}

/// Animation easing types supported by e_window
#[derive(Debug, Clone)]
pub enum AnimationEasing {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    Bounce,
    Elastic,
    Back,
    Custom(String),
}

impl std::fmt::Display for AnimationEasing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnimationEasing::Linear => write!(f, "linear"),
            AnimationEasing::EaseIn => write!(f, "ease-in"),
            AnimationEasing::EaseOut => write!(f, "ease-out"),
            AnimationEasing::EaseInOut => write!(f, "ease-in-out"),
            AnimationEasing::Bounce => write!(f, "bounce"),
            AnimationEasing::Elastic => write!(f, "elastic"),
            AnimationEasing::Back => write!(f, "back"),
            AnimationEasing::Custom(easing) => write!(f, "{}", easing),
        }
    }
}

impl ControlCommand {
    /// Convert the command to the e_window control protocol format
    pub fn to_control_string(&self) -> String {
        match self {
            ControlCommand::Exit => "!control: exit".to_string(),
            
            ControlCommand::SetRect { x, y, width, height } => {
                format!("!control: set_rect {} {} {} {}", x, y, width, height)
            }
            
            ControlCommand::SetRectEased { x, y, width, height, duration_ms, easing } => {
                format!(
                    "!control: set_rect_eased {} {} {} {} {} {}",
                    x, y, width, height, duration_ms, easing
                )
            }
            
            ControlCommand::SetTitle(title) => {
                format!("!control: set_title {}", title)
            }
            
            ControlCommand::BeginDocument => "!control: begin_document".to_string(),
            
            ControlCommand::EndDocument => "!control: end_document".to_string(),
            
            ControlCommand::Content(content) => {
                format!("!control: content {}", content)
            }
            
            ControlCommand::Delay(ms) => {
                format!("!control: delay {}", ms)
            }
        }
    }
}

impl std::fmt::Display for ControlCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_control_string())
    }
}