use e_window_api::get_screen_size;

fn main() {
    let (screen_width, screen_height) = get_screen_size();
    println!("Detected screen size: {}x{}", screen_width, screen_height);
    
    // Test notification positioning (400x150)
    let notification_width = 400u32;
    let notification_height = 150u32;
    let notification_x = (screen_width.saturating_sub(notification_width + 20)) / 2;
    let notification_y = (screen_height.saturating_sub(notification_height + 20)) / 2;
    
    println!("Notification positioning:");
    println!("  Size: {}x{}", notification_width, notification_height);
    println!("  Position: ({}, {})", notification_x, notification_y);
    
    // Test message box positioning (450x250)
    let msgbox_width = 450u32;
    let msgbox_height = 250u32;
    let msgbox_x = (screen_width.saturating_sub(msgbox_width)) / 2;
    let msgbox_y = (screen_height.saturating_sub(msgbox_height)) / 2;
    
    println!("MessageBox positioning:");
    println!("  Size: {}x{}", msgbox_width, msgbox_height);
    println!("  Position: ({}, {})", msgbox_x, msgbox_y);
}