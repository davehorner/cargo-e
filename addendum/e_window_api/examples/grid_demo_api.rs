// Example: Grid demo using the e_window_api
// This demonstrates showing grid information and window positioning using the high-level API

use e_window_api::{get_screen_size, AnimationEasing, Card, EWindow, WindowConfig};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting grid demo with e_window_api...");

    // Get screen dimensions dynamically
    let (screen_width, screen_height) = get_screen_size();
    println!("Detected screen size: {}x{}", screen_width, screen_height);

    // Calculate grid dimensions (3x3 grid with some margin)
    let margin = 50;
    let usable_width = screen_width.saturating_sub(margin * 2);
    let usable_height = screen_height.saturating_sub(margin * 2);

    let cell_width = usable_width / 3;
    let cell_height = usable_height / 3;

    println!("Grid calculations:");
    println!("  Usable area: {}x{}", usable_width, usable_height);
    println!("  Cell size: {}x{}", cell_width, cell_height);

    // Make window size reasonable (not too big)
    let window_width = std::cmp::min(400, cell_width);
    let window_height = std::cmp::min(300, cell_height);

    // Configure the window
    let config = WindowConfig::new()
        .title("PositionGrid Demo")
        .size(window_width, window_height)
        .position(100, 100);

    // Launch the window
    let mut window = EWindow::launch(config).await?;

    // Create the initial grid information card
    let intro_card = Card::new()
        .window_title("PositionGrid 3x3 Demo")
        .field("demo_type", "Grid Positioning")
        .field("version", "1.0")
        .field("author", "Dave Horner")
        .field(
            "screen_size",
            &format!("{}x{}", screen_width, screen_height),
        )
        .card_title("Welcome to the GRID DEMO using e_window_api!")
        .header("Grid Positioning Demonstration")
        .caption("API-driven window management")
        .body_line("This demo shows how to use the API to demonstrate grid positioning concepts.")
        .body_line("")
        .body_line("Features demonstrated:")
        .body_line("- Dynamic screen size detection")
        .body_line("- Window positioning and sizing")
        .body_line("- Dynamic content updates")
        .body_line("- Grid cell positioning simulation")
        .body_line("- Animated window movement to grid positions")
        .body_line("")
        .body_line(&format!(
            "Detected screen: {}x{}",
            screen_width, screen_height
        ))
        .body_line(&format!("Grid cells: {}x{}", cell_width, cell_height))
        .body_line("The window will move to different grid positions automatically...");

    // Show the intro card
    window.show_card(intro_card).await?;
    sleep(Duration::from_secs(3)).await;

    // Define grid positions (simulating a 3x3 grid on screen)
    // Calculate positions based on screen size, accounting for window size
    // so the rightmost/bottommost windows don't go off screen
    let right_margin = screen_width.saturating_sub(window_width + margin);
    let bottom_margin = screen_height.saturating_sub(window_height + margin);

    let x_positions = [
        margin as i32,
        ((screen_width - window_width) / 2) as i32, // Center horizontally
        (right_margin) as i32,                      // Right edge minus window width and margin
    ];

    let y_positions = [
        margin as i32,
        ((screen_height - window_height) / 2) as i32, // Center vertically
        (bottom_margin) as i32,                       // Bottom edge minus window height and margin
    ];

    let grid_positions = [
        (x_positions[0], y_positions[0], "Top-Left"),
        (x_positions[1], y_positions[0], "Top-Center"),
        (x_positions[2], y_positions[0], "Top-Right"),
        (x_positions[0], y_positions[1], "Middle-Left"),
        (x_positions[1], y_positions[1], "Center"),
        (x_positions[2], y_positions[1], "Middle-Right"),
        (x_positions[0], y_positions[2], "Bottom-Left"),
        (x_positions[1], y_positions[2], "Bottom-Center"),
        (x_positions[2], y_positions[2], "Bottom-Right"),
    ];

    // Debug: Print all positions
    println!("Grid positions:");
    for (i, &(x, y, name)) in grid_positions.iter().enumerate() {
        println!("  {}: {} at ({}, {})", i + 1, name, x, y);
    }

    // Demonstrate moving to each grid position
    for (i, &(x, y, position_name)) in grid_positions.iter().enumerate() {
        let grid_card = Card::new()
            .window_title(&format!("Grid Position: {}", position_name))
            .size(window_width, window_height)
            .position(x, y)
            .field("position", position_name)
            .field(
                "grid_x",
                &((x - margin as i32) / cell_width as i32).to_string(),
            )
            .field(
                "grid_y",
                &((y - margin as i32) / cell_height as i32).to_string(),
            )
            .field("cell_index", &i.to_string())
            .card_title(&format!("Grid Cell: {}", position_name))
            .header("Position Demonstration")
            .caption(&format!("Cell {}/9", i + 1))
            .body_line(&format!("Position: ({}, {})", x, y))
            .body_line(&format!("Size: {}x{}", window_width, window_height))
            .body_line(&format!("Cell index: {}/9", i + 1))
            .body_line("")
            .body_line(&format!("Screen: {}x{}", screen_width, screen_height))
            .body_line("This demonstrates precise positioning")
            .body_line("using the e_window_api interface.");

        // Show the card for this position
        window.show_card(grid_card).await?;

        // Animate to the grid position
        window
            .set_rect_animated(
                x,
                y,
                window_width,
                window_height,
                1000,
                AnimationEasing::EaseInOut,
            )
            .await?;

        // Wait at this position
        sleep(Duration::from_millis(2000)).await;
    }

    // Final demonstration: full window view
    let final_card = Card::new()
        .window_title("Grid Demo Complete")
        .size(window_width, window_height)
        .position(100, 100)
        .field("status", "Complete")
        .field("total_positions", "9")
        .field("demo_type", "Grid Navigation")
        .field(
            "screen_size",
            &format!("{}x{}", screen_width, screen_height),
        )
        .card_title("Grid positioning demonstration complete!")
        .header("Demo Summary")
        .caption("All 9 grid positions visited")
        .body_line("The window has visited all 9 grid positions:")
        .body_line("")
        .body_line("Grid Layout (3x3):")
        .body_line("┌─────────────┬─────────────┬─────────────┐")
        .body_line("│  Top-Left   │ Top-Center  │  Top-Right  │")
        .body_line("├─────────────┼─────────────┼─────────────┤")
        .body_line("│ Middle-Left │   Center    │Middle-Right │")
        .body_line("├─────────────┼─────────────┼─────────────┤")
        .body_line("│ Bottom-Left │Bottom-Center│Bottom-Right │")
        .body_line("└─────────────┴─────────────┴─────────────┘")
        .body_line("")
        .body_line(&format!("Screen size: {}x{}", screen_width, screen_height))
        .body_line(&format!("Grid cells: {}x{}", cell_width, cell_height))
        .body_line("Each position demonstrates precise window control")
        .body_line("using the e_window_api's positioning capabilities.")
        .body_line("")
        .body_line("Window will auto-close with countdown timer in 5 seconds...");

    window.show_card_with_auto_close(final_card, 5).await?;
    window
        .set_rect_animated(
            100,
            100,
            window_width,
            window_height,
            1000,
            AnimationEasing::EaseInOut,
        )
        .await?;

    // Wait for auto-close (5 seconds)
    sleep(Duration::from_secs(5)).await;

    window.close().await?;
    println!("Grid demo completed successfully with countdown timer!");

    Ok(())
}
