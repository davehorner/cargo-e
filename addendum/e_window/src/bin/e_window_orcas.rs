#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use egui_tiles::{Tile, TileId, Tiles};
// use std::sync::{Arc, Mutex}; // Unused
use std::thread;
// use std::collections::HashMap; // Unused
use e_window::position_grid::{PositionGrid, SelectionMode};
use std::sync::mpsc::{self, Receiver, Sender};

#[cfg(target_os = "windows")]
use dashmap::DashMap;

#[cfg(target_os = "windows")]
use std::sync::OnceLock;

#[cfg(target_os = "windows")]
use std::process::Command;

#[cfg(target_os = "windows")]
use e_grid::window_easing::{default_easing, WindowAnimationCmd, WindowAnimationFramework};
#[cfg(target_os = "windows")]
use std::sync::OnceLock as AnimationOnceLock;

#[cfg(target_os = "windows")]
static WINDOW_ANIMATION_FRAMEWORK: AnimationOnceLock<WindowAnimationFramework> =
    AnimationOnceLock::new();
#[cfg(target_os = "windows")]
// use windows::Win32::UI::WindowsAndMessaging::{FindWindowA, SetWindowPos, SWP_NOSIZE, SWP_NOZORDER}; // Unused
#[cfg(target_os = "windows")]
static CHROME_WINDOW_INFO_MAP: OnceLock<DashMap<(isize, usize), ChromeWindowInfo>> =
    OnceLock::new();

#[cfg(target_os = "windows")]
#[derive(Debug, Clone)]
pub struct ChromeWindowInfo {
    pub nr: usize,
    pub hwnd: Option<isize>,
    pub pid: Option<u32>,
    pub launched: bool,
    pub userdata: Option<u32>,
}

// Stateless grid rect request/response channel
#[cfg(target_os = "windows")]
use std::sync::mpsc::{sync_channel, SyncSender};

#[cfg(target_os = "windows")]
pub enum GridRectRequest {
    GetRect {
        pane_nr: usize,
        respond_to: SyncSender<Option<(i32, i32, i32, i32)>>,
    },
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Pane {
    pub nr: usize,
    pub url: String,
    pub launched: bool,
    pub selection_mode: SelectionMode,
    pub grid: Option<PositionGrid>,
    /// Offset in logical points from the top of the pane to the top of the grid (for animation alignment)
    pub grid_offset_top: f32,
    // Removed: pub rect_selection: Option<((usize, usize), (usize, usize))>,
}

impl std::fmt::Debug for Pane {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pane")
            .field("nr", &self.nr)
            .field("url", &self.url)
            .field("launched", &self.launched)
            .finish()
    }
}

impl Pane {
    pub fn with_nr(nr: usize) -> Self {
        Self {
            nr,
            url: format!(
                "debugchrome:https://hundredrabbits.github.io/Orca/?!openwindow&!id={}",
                nr
            ),
            launched: false,
            selection_mode: SelectionMode::SingleClick,
            grid: None,
            grid_offset_top: 0.0,
        }
    }

    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        debugchrome_tx: &Option<Sender<DebugChromeCmd>>,
    ) -> egui_tiles::UiResponse {
        ui.label(format!("Pane {}", self.nr));
        let mut moved = false;

        // --- Use PositionGrid widget for grid selection ---
        ui.label("Position Grid:");
        // Allow user to select selection mode
        egui::ComboBox::from_label("Selection Mode")
            .selected_text(match self.selection_mode {
                SelectionMode::SingleClick => "Single Click",
                SelectionMode::FollowSingle => "Follow Single",
                SelectionMode::ClickAndDrag => "Click and Drag",
                SelectionMode::FollowClickAndDrag => "Follow Click and Drag",
                SelectionMode::ClickAndClick => "Click and Click",
                SelectionMode::FollowClickAndClick => "Follow Click and Click",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut self.selection_mode,
                    SelectionMode::SingleClick,
                    "Single Click",
                );
                ui.selectable_value(
                    &mut self.selection_mode,
                    SelectionMode::FollowSingle,
                    "Follow Single",
                );
                ui.selectable_value(
                    &mut self.selection_mode,
                    SelectionMode::ClickAndDrag,
                    "Click and Drag",
                );
                ui.selectable_value(
                    &mut self.selection_mode,
                    SelectionMode::FollowClickAndDrag,
                    "Follow Click and Drag",
                );
                ui.selectable_value(
                    &mut self.selection_mode,
                    SelectionMode::ClickAndClick,
                    "Click and Click",
                );
                ui.selectable_value(
                    &mut self.selection_mode,
                    SelectionMode::FollowClickAndClick,
                    "Follow Click and Click",
                );
            });

        // Only recreate grid if layout parameters change, otherwise adjust existing grid
        let available_width = ui.available_size().x;
        let (tmp_grid, char_size) = PositionGrid::from_text_style(
            None,
            ui,
            egui::TextStyle::Heading,
            egui::Color32::LIGHT_BLUE,
            None,
        );
        let min_cols = 2;
        let cell_width = char_size.x;
        let max_cols = ((available_width / cell_width).floor() as usize).max(min_cols);
        let recreate = match &self.grid {
            Some(g) => g.cols() as usize != max_cols,
            None => true,
        };
        if recreate {
            // Preserve selection if possible
            let prev_selected = self.grid.as_ref().and_then(|g| {
                if let (Some(a), Some(e)) = (g.selection_anchor, g.selection_end) {
                    Some((a, e))
                } else {
                    None
                }
            });
            let (mut grid, _char_size) = PositionGrid::from_text_style(
                Some(max_cols as u32),
                ui,
                egui::TextStyle::Heading,
                egui::Color32::LIGHT_BLUE,
                None,
            );
            grid.selection_mode = self.selection_mode;
            if let Some((anchor, end)) = prev_selected {
                grid.set_selection(Some(anchor), Some(end));
            }
            self.grid = Some(grid);
        } else if let Some(grid) = &mut self.grid {
            // Adjust columns if needed
            if grid.cell_count_x() != max_cols {
                grid.set_cols(max_cols);
            }
            // Adjust selection mode if changed
            grid.selection_mode = self.selection_mode;
        }
        // Now use the grid for UI
        if let Some(grid) = &mut self.grid {
            let (rect, _response) = ui.allocate_exact_size(grid.grid_size, egui::Sense::click());
            grid.set_rect(rect);
            let pane_min_rect_top = ui.min_rect().top();
            let grid_offset_top = rect.top() - pane_min_rect_top;
            self.grid_offset_top = grid_offset_top;
            let clicked = grid.ui(ui);
            match self.selection_mode {
                SelectionMode::SingleClick | SelectionMode::FollowSingle => {
                    if let Some((gx, gy)) = clicked {
                        moved = true;
                        println!(
                            "[Pane {}] PositionGrid set to ({}, {}) (offset above grid: {})",
                            self.nr, gx, gy, self.grid_offset_top
                        );
                        grid.set_selected_cell(Some((gx, gy)));
                    }
                }
                SelectionMode::ClickAndDrag
                | SelectionMode::FollowClickAndDrag
                | SelectionMode::ClickAndClick
                | SelectionMode::FollowClickAndClick => {
                    if let (Some(anchor), Some(end)) = (grid.selection_anchor, grid.selection_end) {
                        ui.label(format!(
                            "Rectangular selection: anchor=({}, {}), end=({}, {})",
                            anchor.0, anchor.1, end.0, end.1
                        ));
                    }
                }
            }
        }

        if ui.button("Launch DebugChrome").clicked() {
            if let Some(tx) = debugchrome_tx {
                let _ = tx.send(DebugChromeCmd::Launch(self.nr, self.url.clone()));
                println!("[Pane {}] Sent launch command for DebugChrome", self.nr);
            }
        }
        #[cfg(target_os = "windows")]
        {
            if let Some(map) = CHROME_WINDOW_INFO_MAP.get() {
                for entry in map.iter() {
                    if entry.value().nr == self.nr {
                        ui.label(format!("HWND: {:?}", entry.value().hwnd));
                        ui.label(format!("PID: {:?}", entry.value().pid));
                        ui.label(format!("Launched: {}", entry.value().launched));
                    }
                }
            }
        }
        egui_tiles::UiResponse::None
    }
}

pub enum DebugChromeCmd {
    Launch(usize, String),
    Move(usize, i32, i32),
    MoveToRect {
        hwnd: isize,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
    },
}

struct TreeBehavior {
    simplification_options: egui_tiles::SimplificationOptions,
    tab_bar_height: f32,
    gap_width: f32,
    add_child_to: Option<egui_tiles::TileId>,
    debugchrome_tx: Option<Sender<DebugChromeCmd>>,
}

impl Default for TreeBehavior {
    fn default() -> Self {
        Self {
            simplification_options: Default::default(),
            tab_bar_height: 24.0,
            gap_width: 2.0,
            add_child_to: None,
            debugchrome_tx: None,
        }
    }
}

impl TreeBehavior {
    fn ui(&mut self, ui: &mut egui::Ui) {
        let Self {
            simplification_options,
            tab_bar_height,
            gap_width,
            add_child_to: _,
            debugchrome_tx: _,
        } = self;

        egui::Grid::new("behavior_ui")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("All panes must have tabs:");
                ui.checkbox(&mut simplification_options.all_panes_must_have_tabs, "");
                ui.end_row();

                ui.label("Join nested containers:");
                ui.checkbox(
                    &mut simplification_options.join_nested_linear_containers,
                    "",
                );
                ui.end_row();

                ui.label("Tab bar height:");
                ui.add(
                    egui::DragValue::new(tab_bar_height)
                        .range(0.0..=100.0)
                        .speed(1.0),
                );
                ui.end_row();

                ui.label("Gap width:");
                ui.add(egui::DragValue::new(gap_width).range(0.0..=20.0).speed(1.0));
                ui.end_row();
            });
    }
}

impl egui_tiles::Behavior<Pane> for TreeBehavior {
    fn pane_ui(
        &mut self,
        ui: &mut egui::Ui,
        _tile_id: egui_tiles::TileId,
        view: &mut Pane,
    ) -> egui_tiles::UiResponse {
        view.ui(ui, &self.debugchrome_tx)
    }

    fn tab_title_for_pane(&mut self, view: &Pane) -> egui::WidgetText {
        format!("Pane {}", view.nr).into()
    }

    fn top_bar_right_ui(
        &mut self,
        _tiles: &egui_tiles::Tiles<Pane>,
        ui: &mut egui::Ui,
        tile_id: egui_tiles::TileId,
        _tabs: &egui_tiles::Tabs,
        _scroll_offset: &mut f32,
    ) {
        if ui.button("➕").clicked() {
            self.add_child_to = Some(tile_id);
        }
    }

    fn tab_bar_height(&self, _style: &egui::Style) -> f32 {
        self.tab_bar_height
    }

    fn gap_width(&self, _style: &egui::Style) -> f32 {
        self.gap_width
    }

    fn simplification_options(&self) -> egui_tiles::SimplificationOptions {
        self.simplification_options
    }

    fn is_tab_closable(&self, _tiles: &Tiles<Pane>, _tile_id: TileId) -> bool {
        true
    }

    fn on_tab_close(&mut self, tiles: &mut Tiles<Pane>, tile_id: TileId) -> bool {
        if let Some(tile) = tiles.get(tile_id) {
            match tile {
                Tile::Pane(pane) => {
                    let tab_title = self.tab_title_for_pane(pane);
                    println!(
                        "[Diagnostics] Closing tab: {}, tile ID: {tile_id:?}",
                        tab_title.text()
                    );
                }
                Tile::Container(container) => {
                    println!("[Diagnostics] Closing container: {:?}", container.kind());
                    let children_ids = container.children();
                    for child_id in children_ids {
                        if let Some(Tile::Pane(pane)) = tiles.get(*child_id) {
                            let tab_title = self.tab_title_for_pane(pane);
                            println!(
                                "[Diagnostics] Closing tab: {}, tile ID: {tile_id:?}",
                                tab_title.text()
                            );
                        }
                    }
                }
            }
        }
        true
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
struct MyApp {
    tree: egui_tiles::Tree<Pane>,
    #[cfg_attr(feature = "serde", serde(skip))]
    behavior: TreeBehavior,
    debugchrome_tx: Option<Sender<DebugChromeCmd>>,
    #[cfg_attr(feature = "serde", serde(skip))]
    debugchrome_rx: Option<Receiver<DebugChromeCmd>>,
    #[cfg_attr(feature = "serde", serde(skip))]
    debugchrome_thread: Option<thread::JoinHandle<()>>,
    #[cfg(target_os = "windows")]
    gridrect_tx: Option<SyncSender<GridRectRequest>>,
    #[cfg(target_os = "windows")]
    gridrect_rx: Option<std::sync::mpsc::Receiver<GridRectRequest>>,
    #[cfg(target_os = "windows")]
    pixels_per_point: f32,
}

impl Default for MyApp {
    fn default() -> Self {
        let mut next_view_nr = 0;
        let mut gen_view = || {
            let view = Pane::with_nr(next_view_nr);
            next_view_nr += 1;
            view
        };

        let mut tiles = egui_tiles::Tiles::default();
        let mut tabs = vec![];
        let tab_tile = {
            let children = (0..4).map(|_| tiles.insert_pane(gen_view())).collect();
            tiles.insert_tab_tile(children)
        };
        tabs.push(tab_tile);
        tabs.push({
            let children = (0..4).map(|_| tiles.insert_pane(gen_view())).collect();
            tiles.insert_horizontal_tile(children)
        });
        tabs.push({
            let children = (0..4).map(|_| tiles.insert_pane(gen_view())).collect();
            tiles.insert_vertical_tile(children)
        });
        tabs.push({
            let cells = (0..6).map(|_| tiles.insert_pane(gen_view())).collect();
            tiles.insert_grid_tile(cells)
        });
        tabs.push(tiles.insert_pane(gen_view()));
        let root = tiles.insert_tab_tile(tabs);
        let tree = egui_tiles::Tree::new("my_tree", root, tiles);

        // Channel for debugchrome background thread
        let (tx, rx) = mpsc::channel();

        #[cfg(target_os = "windows")]
        {
            CHROME_WINDOW_INFO_MAP.get_or_init(DashMap::new);
        }

        // gridrect channel for stateless lookup
        #[cfg(target_os = "windows")]
        let (gridrect_tx, gridrect_rx) = sync_channel::<GridRectRequest>(8);
        let debugchrome_thread = None; // Will be set in main

        let mut behavior = TreeBehavior::default();
        behavior.debugchrome_tx = Some(tx.clone());

        Self {
            tree,
            behavior,
            debugchrome_tx: Some(tx.clone()),
            debugchrome_rx: Some(rx),
            debugchrome_thread,
            #[cfg(target_os = "windows")]
            gridrect_tx: Some(gridrect_tx),
            #[cfg(target_os = "windows")]
            gridrect_rx: Some(gridrect_rx),
            #[cfg(target_os = "windows")]
            pixels_per_point: 1.0,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(target_os = "windows")]
        {
            self.pixels_per_point = ctx.pixels_per_point();
            use std::sync::mpsc::TryRecvError;
            if let Some(gridrect_rx) = &self.gridrect_rx {
                let scale = self.pixels_per_point;
                loop {
                    match gridrect_rx.try_recv() {
                        Ok(GridRectRequest::GetRect {
                            pane_nr,
                            respond_to,
                        }) => {
                            // Find the pane and its grid rect
                            let rect = self.tree.tiles.iter().find_map(|(_id, tile)| {
                                if let egui_tiles::Tile::Pane(pane) = tile {
                                    if pane.nr == pane_nr {
                                        if let Some(grid) = &pane.grid {
                                            let r = grid.rect;
                                            // Scale to physical pixels
                                            return Some((
                                                (r.left() * scale).round() as i32,
                                                (r.top() * scale).round() as i32,
                                                (r.width() * scale).round() as i32,
                                                (r.height() * scale).round() as i32,
                                            ));
                                        }
                                    }
                                }
                                None
                            });
                            let _ = respond_to.send(rect);
                        }
                        Err(TryRecvError::Empty) => break,
                        Err(TryRecvError::Disconnected) => break,
                    }
                }
            }
        }

        egui::SidePanel::left("tree")
            .resizable(true)
            .show(ctx, |ui| {
                if ui.button("Reset").clicked() {
                    *self = Default::default();
                }
                self.behavior.ui(ui);
                ui.separator();
                ui.collapsing("Diagnostics", |ui| {
                    #[cfg(target_os = "windows")]
                    {
                        if let Some(map) = CHROME_WINDOW_INFO_MAP.get() {
                            for entry in map.iter() {
                                ui.label(format!("Key {:?}: {:?}", entry.key(), entry.value()));
                            }
                        }
                    }
                });
                ui.separator();
                ui.collapsing("Tree", |ui| {
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                    let tree_debug = format!("{:#?}", self.tree);
                    ui.monospace(&tree_debug);
                });
                ui.separator();
                if let Some(root) = self.tree.root() {
                    let tiles = &mut self.tree.tiles;
                    tree_ui(ui, &mut self.behavior, tiles, root);
                }
                if let Some(parent) = self.behavior.add_child_to.take() {
                    let new_child = self.tree.tiles.insert_pane(Pane::with_nr(100));
                    if let Some(egui_tiles::Tile::Container(egui_tiles::Container::Tabs(tabs))) =
                        self.tree.tiles.get_mut(parent)
                    {
                        tabs.add_child(new_child);
                        tabs.set_active(new_child);
                    }
                }
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            self.tree.ui(&mut self.behavior, ui);
        });
    }

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        #[cfg(feature = "serde")]
        eframe::set_value(_storage, eframe::APP_KEY, &self);
    }
}

fn tree_ui(
    ui: &mut egui::Ui,
    behavior: &mut dyn egui_tiles::Behavior<Pane>,
    tiles: &mut egui_tiles::Tiles<Pane>,
    tile_id: egui_tiles::TileId,
) {
    let text = format!(
        "{} - {tile_id:?}",
        behavior.tab_title_for_tile(tiles, tile_id).text()
    );
    let Some(mut tile) = tiles.remove(tile_id) else {
        println!("[Diagnostics] Missing tile {tile_id:?}");
        return;
    };
    let default_open = true;
    egui::collapsing_header::CollapsingState::load_with_default_open(
        ui.ctx(),
        ui.id().with((tile_id, "tree")),
        default_open,
    )
    .show_header(ui, |ui| {
        ui.label(text);
        let mut visible = tiles.is_visible(tile_id);
        ui.checkbox(&mut visible, "Visible");
        tiles.set_visible(tile_id, visible);
    })
    .body(|ui| match &mut tile {
        egui_tiles::Tile::Pane(_) => {}
        egui_tiles::Tile::Container(container) => {
            let mut kind = container.kind();
            egui::ComboBox::from_label("Kind")
                .selected_text(format!("{kind:?}"))
                .show_ui(ui, |ui| {
                    for alternative in egui_tiles::ContainerKind::ALL {
                        ui.selectable_value(&mut kind, alternative, format!("{alternative:?}"))
                            .clicked();
                    }
                });
            if kind != container.kind() {
                container.set_kind(kind);
            }
            for &child in container.children() {
                tree_ui(ui, behavior, tiles, child);
            }
        }
    });
    tiles.insert(tile_id, tile);
}


#[cfg(target_os = "windows")]
fn main() -> Result<(), eframe::Error> {
    env_logger::init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1000.0, 700.0]),
        ..Default::default()
    };
    eframe::run_native(
        "egui_tiles orcas advanced demo",
        options,
        Box::new(|_cc| {
            #[cfg_attr(not(feature = "serde"), allow(unused_mut))]
            let mut app = MyApp::default();
            // Initialize the animation framework once
            WINDOW_ANIMATION_FRAMEWORK.get_or_init(|| WindowAnimationFramework::new());
            // Start debugchrome thread with gridrect_tx
            let debugchrome_rx = app.debugchrome_rx.take().unwrap();
            let gridrect_tx = app.gridrect_tx.as_ref().unwrap().clone();
            app.debugchrome_thread = Some(start_debugchrome_thread_with_gridrect(
                debugchrome_rx,
                gridrect_tx,
            ));
            #[cfg(feature = "serde")]
            if let Some(storage) = _cc.storage {
                if let Some(state) = eframe::get_value(storage, eframe::APP_KEY) {
                    app = state;
                }
            }
            Ok(Box::new(app))
        }),
    )
}

#[cfg(not(target_os = "windows"))]
fn main() {
    println!("e_window_orcas is only supported on Windows.");
}

#[cfg(target_os = "windows")]
fn animate_hwnd_to_grid(hwnd: isize, rect: (i32, i32, i32, i32), duration_ms: u32) {
    if let Some(fw) = WINDOW_ANIMATION_FRAMEWORK.get() {
        let _ = fw.tx.send(WindowAnimationCmd::Animate {
            hwnd,
            x: rect.0,
            y: rect.1,
            w: rect.2,
            h: rect.3,
            duration_ms,
            easing: default_easing(),
        });
    }
}

#[cfg(target_os = "windows")]
fn start_debugchrome_thread_with_gridrect(
    rx: Receiver<DebugChromeCmd>,
    gridrect_tx: SyncSender<GridRectRequest>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        println!("[DebugChrome] Background thread started and waiting for commands");
        use std::io::{BufRead, BufReader};
        while let Ok(cmd) = rx.recv() {
            match cmd {
                DebugChromeCmd::Launch(nr, url) => {
                    println!("[DebugChrome] Launching Chrome for pane {}: {}", nr, url);
                    println!("[DebugChrome] To run manually: debugchrome \"{}\"", url);
                    let mut child = match Command::new("debugchrome")
                        .arg(&url)
                        .arg("--redirect-seconds")
                        .arg(4.to_string())
                        .stdout(std::process::Stdio::piped())
                        .spawn()
                    {
                        Ok(child) => child,
                        Err(e) => {
                            println!("[DebugChrome] Failed to spawn debugchrome: {}", e);
                            continue;
                        }
                    };
                    let stdout = child.stdout.take().unwrap();
                    let reader = BufReader::new(stdout);
                    let mut hwnd: Option<isize> = None;
                    let mut pid: Option<u32> = None;
                    for line in reader.lines() {
                        if let Ok(line) = line {
                            println!("[DebugChrome][stdout] {}", line);
                            if line.contains("HWND:") {
                                if let Some(hwnd_str) = line.split("HWND:").nth(1) {
                                    let hwnd_str = hwnd_str.trim();
                                    if hwnd_str.starts_with("0x") {
                                        if let Ok(hwnd_val) = isize::from_str_radix(
                                            hwnd_str.trim_start_matches("0x"),
                                            16,
                                        ) {
                                            hwnd = Some(hwnd_val);
                                        }
                                    } else if let Ok(hwnd_val) = hwnd_str.parse::<isize>() {
                                        hwnd = Some(hwnd_val);
                                    }
                                    println!("[DebugChrome] Found HWND: {}", hwnd_str);
                                }
                            }
                            if line.contains("PID:") {
                                if let Some(pid_str) = line.split("PID:").nth(1) {
                                    let pid_str =
                                        pid_str.trim().split_whitespace().next().unwrap_or("");
                                    if let Ok(pid_val) = pid_str.parse::<u32>() {
                                        pid = Some(pid_val);
                                        println!("[DebugChrome] Found PID: {}", pid_val);
                                    }
                                }
                            }
                            if hwnd.is_some() && pid.is_some() {
                                break;
                            }
                        }
                    }
                    let userdata: u32 = nr as u32;
                    // --- Insert or update HWND and PID for this pane ---
                    #[cfg(target_os = "windows")]
                    {
                        if let Some(map) = CHROME_WINDOW_INFO_MAP.get() {
                            if let Some(hwnd_val) = hwnd {
                                println!(
                                    "[DebugChrome] Inserting/updating HWND 0x{:X} for pane {}",
                                    hwnd_val, nr
                                );
                                map.insert(
                                    (hwnd_val, nr),
                                    ChromeWindowInfo {
                                        nr,
                                        hwnd,
                                        pid,
                                        launched: true,
                                        userdata: Some(userdata),
                                    },
                                );
                            }
                            // Print the full map after insert for debugging
                            println!("[DebugChrome] CHROME_WINDOW_INFO_MAP after insert:");
                            for entry in map.iter() {
                                println!("  Key {:?}: {:?}", entry.key(), entry.value());
                            }
                        }
                    }
                    println!("[DebugChrome] Inserted window info for pane {}: hwnd={:?}, pid={:?}, userdata={:?}", nr, hwnd, pid, userdata);
                    // --- Request grid rect from main thread ---
                    if let Some(hwnd_val) = hwnd {
                        let (rect_tx, rect_rx) = sync_channel(1);
                        gridrect_tx
                            .send(GridRectRequest::GetRect {
                                pane_nr: nr,
                                respond_to: rect_tx,
                            })
                            .unwrap();
                        let rect = rect_rx.recv().unwrap_or(None);
                        let target_rect = rect.unwrap_or((100, 100, 800, 600));
                        animate_hwnd_to_grid(hwnd_val, target_rect, 1000);
                    }
                }
                DebugChromeCmd::Move(nr, _x, _y) => {
                    // Look up HWND for this pane
                    // #[cfg(target_os = "windows")]
                    // {
                    //     if let Some(map) = CHROME_WINDOW_INFO_MAP.get() {
                    //         if let Some(info) = map.get(&nr) {
                    //             if let Some(hwnd_val) = info.hwnd {
                    //                 // Request the latest grid rect for this pane
                    //                 let (rect_tx, rect_rx) = sync_channel(1);
                    //                 gridrect_tx.send(GridRectRequest::GetRect { pane_nr: nr, respond_to: rect_tx }).unwrap();
                    //                 let rect = rect_rx.recv().unwrap_or(None);
                    //                 let target_rect = rect.unwrap_or((100, 100, 800, 600));
                    //                 println!("[DebugChrome] Move command for pane {}: animating hwnd={:?} to rect {:?}", nr, hwnd_val, target_rect);
                    //                 animate_hwnd_to_grid(hwnd_val, target_rect, 1000);
                    //             } else {
                    //                 println!("[DebugChrome] Move command for pane {}: HWND not found", nr);
                    //             }
                    //         } else {
                    //             println!("[DebugChrome] Move command for pane {}: info not found in map", nr);
                    //         }
                    //     }
                    // }
                }
                DebugChromeCmd::MoveToRect { .. } => {}
            }
            // No need to poll for gridrect requests here; handled in main thread
        }
        println!("[DebugChrome] Background thread exiting");
    })
}
