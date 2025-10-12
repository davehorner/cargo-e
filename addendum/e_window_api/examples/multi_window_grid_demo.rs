// Example: Multi-window grid demo using the e_window_api
// This demonstrates creating multiple concurrent windows arranged in a grid,
// then shuffling them around the screen

use e_window_api::{get_screen_size, AnimationEasing, Card, EWindow, WindowConfig};
use std::collections::HashMap;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting multi-window grid demo with e_window_api...");

    // Get screen dimensions dynamically
    let (screen_width, screen_height) = get_screen_size();
    println!("Detected screen size: {}x{}", screen_width, screen_height);

    // Calculate grid dimensions (3x3 grid with some margin)
    let margin = 50;
    let window_width = 350;
    let window_height = 250;

    // Calculate optimal positioning for 3x3 grid
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

    // Define grid cell information
    let grid_cells = [
        (0, 0, "Top-Left", "🌅"),
        (1, 0, "Top-Center", "☀️"),
        (2, 0, "Top-Right", "🌄"),
        (0, 1, "Middle-Left", "🌲"),
        (1, 1, "Center", "🎯"),
        (2, 1, "Middle-Right", "🌳"),
        (0, 2, "Bottom-Left", "🌊"),
        (1, 2, "Bottom-Center", "🏔️"),
        (2, 2, "Bottom-Right", "🌙"),
    ];

    println!("Creating {} windows in grid formation...", grid_cells.len());

    // Create all windows concurrently
    let mut windows = HashMap::new();
    let mut window_tasks = Vec::new();

    for (col, row, name, emoji) in grid_cells.iter() {
        let x = x_positions[*col];
        let y = y_positions[*row];

        let config = WindowConfig::new()
            .title(&format!("Grid Cell: {}", name))
            .size(window_width, window_height)
            .position(x, y);

        let cell_name = name.to_string();
        let cell_emoji = emoji.to_string();
        let cell_position = (*col, *row);

        // Launch window asynchronously
        let task = async move {
            let window = EWindow::launch(config).await?;
            Ok::<(String, EWindow), Box<dyn std::error::Error>>((cell_name, window))
        };

        window_tasks.push((cell_position, cell_emoji, task));
    }

    // Wait for all windows to launch
    for (position, emoji, task) in window_tasks {
        let (name, window) = task.await?;
        windows.insert(position, (name.clone(), emoji, window));
        println!("✅ Launched window: {}", name);
    }

    println!("All {} windows created successfully!", windows.len());

    // Show initial cards for each window
    println!("Displaying initial cards...");
    for ((col, row), (name, emoji, window)) in windows.iter_mut() {
        let card = Card::new()
            .window_title(&format!("{} {}", emoji, name))
            .field("position", &format!("Column {}, Row {}", col + 1, row + 1))
            .field(
                "coordinates",
                &format!("({}, {})", x_positions[*col], y_positions[*row]),
            )
            .field("emoji", &**emoji)
            .card_title(&format!("Grid Cell: {} {}", emoji, name))
            .body_line(&format!(
                "Initial position: Column {}, Row {}",
                col + 1,
                row + 1
            ))
            .body_line(&format!("Window size: {}x{}", window_width, window_height))
            .body_line("")
            .body_line("This window is part of a 3x3 grid demonstration.")
            .body_line("All windows will move and shuffle positions shortly...")
            .body_line("")
            .body_line("🎭 Multi-Window Grid Demo 🎭");

        window.show_card(card).await?;
    }

    println!("Waiting 3 seconds before starting animations...");
    sleep(Duration::from_secs(3)).await;

    // First shuffle: Rotate all windows clockwise around the grid
    println!("🔄 Starting clockwise rotation...");
    let rotation_sequence = [
        // Outer ring clockwise: TL -> TC -> TR -> MR -> BR -> BC -> BL -> ML -> back to TL
        ((0, 0), (1, 0)), // Top-Left -> Top-Center
        ((1, 0), (2, 0)), // Top-Center -> Top-Right
        ((2, 0), (2, 1)), // Top-Right -> Middle-Right
        ((2, 1), (2, 2)), // Middle-Right -> Bottom-Right
        ((2, 2), (1, 2)), // Bottom-Right -> Bottom-Center
        ((1, 2), (0, 2)), // Bottom-Center -> Bottom-Left
        ((0, 2), (0, 1)), // Bottom-Left -> Middle-Left
        ((0, 1), (0, 0)), // Middle-Left -> Top-Left
        // Center stays in center
        ((1, 1), (1, 1)), // Center -> Center (no move)
    ];

    // Execute rotation with animation
    for ((from_col, from_row), (to_col, to_row)) in rotation_sequence.iter() {
        if let Some((name, emoji, window)) = windows.get_mut(&(*from_col, *from_row)) {
            let target_x = x_positions[*to_col];
            let target_y = y_positions[*to_row];

            println!(
                "Moving {} {} from ({},{}) to ({},{})",
                emoji, name, from_col, from_row, to_col, to_row
            );

            // Update the card to show movement
            let move_card = Card::new()
                .window_title(&format!("{} {} - Moving!", emoji, name))
                .field("from", &format!("({}, {})", from_col, from_row))
                .field("to", &format!("({}, {})", to_col, to_row))
                .field("status", "🚀 In Motion")
                .card_title(&format!("🎪 {} is moving!", name))
                .body_line(&format!(
                    "From: Column {}, Row {}",
                    from_col + 1,
                    from_row + 1
                ))
                .body_line(&format!("To: Column {}, Row {}", to_col + 1, to_row + 1))
                .body_line("")
                .body_line("Clockwise rotation in progress...");

            window.show_card(move_card).await?;

            // Animate to new position
            window
                .set_rect_animated(
                    target_x,
                    target_y,
                    window_width,
                    window_height,
                    2000,
                    AnimationEasing::EaseInOut,
                )
                .await?;
        }

        sleep(Duration::from_millis(300)).await; // Stagger the movements
    }

    println!("Rotation complete! Waiting 2 seconds...");
    sleep(Duration::from_secs(2)).await;

    // Second shuffle: Random swaps
    println!("🎲 Starting random shuffle...");
    let swap_pairs = [
        ((0, 0), (2, 2)), // Top-Left <-> Bottom-Right
        ((1, 0), (1, 2)), // Top-Center <-> Bottom-Center
        ((2, 0), (0, 2)), // Top-Right <-> Bottom-Left
        ((0, 1), (2, 1)), // Middle-Left <-> Middle-Right
    ];

    for ((pos1_col, pos1_row), (pos2_col, pos2_row)) in swap_pairs.iter() {
        // We need to get both windows mutably, but HashMap doesn't allow that directly
        // So we'll handle them sequentially instead of simultaneously

        let target1_x = x_positions[*pos2_col];
        let target1_y = y_positions[*pos2_row];
        let target2_x = x_positions[*pos1_col];
        let target2_y = y_positions[*pos1_row];

        // Get names and emojis first
        let (name1, emoji1) = if let Some((name, emoji, _)) = windows.get(&(*pos1_col, *pos1_row)) {
            (name.clone(), emoji.clone())
        } else {
            continue;
        };

        let (name2, emoji2) = if let Some((name, emoji, _)) = windows.get(&(*pos2_col, *pos2_row)) {
            (name.clone(), emoji.clone())
        } else {
            continue;
        };

        println!("Swapping {} {} <-> {} {}", emoji1, name1, emoji2, name2);

        // Animate first window
        if let Some((_, _, window1)) = windows.get_mut(&(*pos1_col, *pos1_row)) {
            let swap_card1 = Card::new()
                .window_title(&format!("{} {} - Swapping!", emoji1, name1))
                .field("swapping_with", &format!("{} {}", emoji2, name2))
                .field("status", "🔄 Swapping")
                .card_title(&format!("🎭 {} is swapping with {}!", name1, name2));

            window1.show_card(swap_card1).await?;

            window1
                .set_rect_animated(
                    target1_x,
                    target1_y,
                    window_width,
                    window_height,
                    1500,
                    AnimationEasing::EaseInOut,
                )
                .await?;
        }

        // Animate second window
        if let Some((_, _, window2)) = windows.get_mut(&(*pos2_col, *pos2_row)) {
            let swap_card2 = Card::new()
                .window_title(&format!("{} {} - Swapping!", emoji2, name2))
                .field("swapping_with", &format!("{} {}", emoji1, name1))
                .field("status", "🔄 Swapping")
                .card_title(&format!("🎭 {} is swapping with {}!", name2, name1));

            window2.show_card(swap_card2).await?;

            window2
                .set_rect_animated(
                    target2_x,
                    target2_y,
                    window_width,
                    window_height,
                    1500,
                    AnimationEasing::EaseInOut,
                )
                .await?;
        }

        sleep(Duration::from_millis(500)).await; // Brief pause between swaps
    }

    println!("Random shuffle complete! Waiting 2 seconds...");
    sleep(Duration::from_secs(2)).await;

    // Final: Return all windows to original positions
    println!("🏠 Returning all windows to original positions...");
    for ((col, row), (name, emoji, window)) in windows.iter_mut() {
        let original_x = x_positions[*col];
        let original_y = y_positions[*row];

        let final_card = Card::new()
            .window_title(&format!("{} {} - Returning Home", emoji, name))
            .field(
                "original_position",
                &format!("Column {}, Row {}", col + 1, row + 1),
            )
            .field("status", "🏠 Going Home")
            .card_title(&format!("🎯 {} returning to original position!", name))
            .body_line(&format!("Back to: Column {}, Row {}", col + 1, row + 1))
            .body_line("")
            .body_line("Multi-window grid demo completing...");

        window.show_card(final_card).await?;

        window
            .set_rect_animated(
                original_x,
                original_y,
                window_width,
                window_height,
                2000,
                AnimationEasing::EaseInOut,
            )
            .await?;
    }

    println!("All windows returned to original positions!");
    sleep(Duration::from_secs(3)).await;

    // Show completion message on all windows
    for ((col, row), (name, emoji, window)) in windows.iter_mut() {
        let completion_card = Card::new()
            .window_title(&format!("{} {} - Demo Complete!", emoji, name))
            .field("total_windows", "9")
            .field(
                "final_position",
                &format!("Column {}, Row {}", col + 1, row + 1),
            )
            .field("demo_status", "✅ Complete")
            .card_title("🎉 Multi-Window Grid Demo Complete! 🎉")
            .body_line("")
            .body_line("Demonstrations performed:")
            .body_line("✅ Concurrent window creation")
            .body_line("✅ Grid formation and positioning")
            .body_line("✅ Coordinated animations")
            .body_line("✅ Clockwise rotation")
            .body_line("✅ Random position swapping")
            .body_line("✅ Return to original positions")
            .body_line("")
            .body_line("All windows will close in 5 seconds...")
            .body_line("")
            .body_line("Thank you for watching! 🎭");

        window.show_card(completion_card).await?;
    }

    println!("Demo complete! Closing all windows in 5 seconds...");
    sleep(Duration::from_secs(5)).await;

    // Close all windows
    for ((col, row), (name, emoji, window)) in windows.into_iter() {
        println!("Closing {} {} at ({}, {})", emoji, name, col, row);
        window.close().await?;
    }

    println!("Multi-window grid demo completed successfully!");
    println!("🎉 All {} windows closed gracefully!", grid_cells.len());

    Ok(())
}
