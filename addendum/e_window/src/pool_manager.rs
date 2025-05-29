use eframe::{egui, Frame};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct PoolManagerApp {
    pub pool_size: usize,
    pub rate_ms: u64,
    pub last_spawn: Arc<Mutex<Instant>>,
    pub spawned: Arc<Mutex<usize>>,
    pub children: Arc<Mutex<Vec<std::process::Child>>>,
    pub shutdown: Arc<std::sync::atomic::AtomicBool>, // Add this line
}

impl PoolManagerApp {
    pub fn new(pool_size: usize, rate_ms: u64) -> Self {
        Self {
            pool_size,
            rate_ms,
            last_spawn: Arc::new(Mutex::new(Instant::now())),
            spawned: Arc::new(Mutex::new(0)),
            children: Arc::new(Mutex::new(Vec::new())),
            shutdown: Arc::new(std::sync::atomic::AtomicBool::new(false)), // Initialize here
        }
    }
}

impl eframe::App for PoolManagerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("e_window Pool Manager");
            ui.label(format!("Target pool size: {}", self.pool_size));
            ui.label(format!("Spawn rate: {} ms", self.rate_ms));
            let spawned = *self.spawned.lock().unwrap();
            ui.label(format!("Total windows spawned: {}", spawned));
            let last = *self.last_spawn.lock().unwrap();
            ui.label(format!("Last spawn: {:.1?} ago", last.elapsed()));
            ui.label("This window manages the pool and will keep at least N windows open.");
        });
        ctx.request_repaint_after(Duration::from_millis(500));
    }
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.shutdown
            .store(true, std::sync::atomic::Ordering::Relaxed);
        let mut children = self.children.lock().unwrap();
        for child in children.iter_mut() {
            let _ = child.kill();
        }
        children.clear();
    }
}
