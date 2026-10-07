use std::io::{self, Write};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

const BAR_WIDTH: usize = 40;
const UPDATE_INTERVAL_MS: Duration = Duration::from_millis(100);

pub struct ProgressBar {
    total:       u64,
    current:     AtomicU64,
    start_time:  Instant,
    prefix:      String,
    last_update: Mutex<Instant>,
}

impl ProgressBar {
    pub fn with_prefix(total: u64, prefix: &str) -> Self {
        Self {
            total,
            current: AtomicU64::new(0),
            start_time: Instant::now(),
            prefix: prefix.to_string(),
            last_update: Mutex::new(Instant::now()),
        }
    }

    pub fn inc(&self, delta: u64) {
        self.current.fetch_add(delta, Ordering::Relaxed);
        self.maybe_display();
    }

    fn maybe_display(&self) {
        let mut last = self.last_update.lock().expect("last_update lock poisoned");
        if last.elapsed() < UPDATE_INTERVAL_MS {
            return;
        }
        *last = Instant::now();
        drop(last);
        self.display();
    }

    fn display(&self) {
        let current = self.current.load(Ordering::Relaxed);
        let percent = if self.total > 0 {
            (current * 100).checked_div(self.total).unwrap_or(100)
        } else {
            100
        };

        let filled = (percent * BAR_WIDTH as u64) / 100;
        let bar = format!("[{}{}] {percent}%", "#".repeat(filled as usize), "-".repeat(BAR_WIDTH - filled as usize));

        let prefix_str = if self.prefix.is_empty() { String::new() } else { format!("{} ", self.prefix) };

        print!("\r{prefix_str}{bar}");
        io::stdout().flush().ok();
    }

    pub fn finish(&self) {
        let elapsed = self.start_time.elapsed();
        let clear_width = self.prefix.len() + BAR_WIDTH + 10;
        print!("\r{}", " ".repeat(clear_width));
        print!("\r");
        if self.prefix.is_empty() {
            println!("done in {}.", format_duration(elapsed));
        } else {
            println!("{} done in {}.", self.prefix, format_duration(elapsed));
        }
    }
}

fn format_duration(d: Duration) -> String {
    let secs = d.as_secs_f64();
    if secs < 1.0 {
        format!("{}ms", d.as_millis())
    } else if secs < 60.0 {
        format!("{secs:.2}s")
    } else {
        let mins = secs.floor() / 60.0;
        let remainder = secs % 60.0;
        format!("{}m {:.2}s", mins as i64, remainder)
    }
}
