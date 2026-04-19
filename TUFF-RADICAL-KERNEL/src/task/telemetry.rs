use core::sync::atomic::{AtomicU64, Ordering};


pub struct Telemetry {
    pub total_polls: AtomicU64,
    pub tasks_spawned: AtomicU64,
    pub tasks_completed: AtomicU64,
    pub last_heartbeat_tick: AtomicU64,
}

pub static GLOBAL_TELEMETRY: Telemetry = Telemetry {
    total_polls: AtomicU64::new(0),
    tasks_spawned: AtomicU64::new(0),
    tasks_completed: AtomicU64::new(0),
    last_heartbeat_tick: AtomicU64::new(0),
};

pub fn log_heartbeat(current_tick: u64) {
    let last = GLOBAL_TELEMETRY.last_heartbeat_tick.load(Ordering::Relaxed);
    if current_tick >= last + 1000 {
        GLOBAL_TELEMETRY.last_heartbeat_tick.store(current_tick, Ordering::Relaxed);
        serial_println!(
            "TUFF-RADICAL-TELEMETRY: tick={} polls={} tasks(s/c)={}/{}",
            current_tick,
            GLOBAL_TELEMETRY.total_polls.load(Ordering::Relaxed),
            GLOBAL_TELEMETRY.tasks_spawned.load(Ordering::Relaxed),
            GLOBAL_TELEMETRY.tasks_completed.load(Ordering::Relaxed)
        );
    }
}
