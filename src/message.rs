use std::path::PathBuf;
use crossbeam_channel::{unbounded, Sender, Receiver};

#[derive(Debug, Clone)]
pub enum AppMessage {
    SetReplayDir(PathBuf),
    AddLog(String),
    SetWatcherRunning(bool),
    ToggleAutoFix(bool),
    ToggleAutoStart(bool),
    None,
}

lazy_static::lazy_static! {
    static ref CHANNEL: (Sender<AppMessage>, Receiver<AppMessage>) = unbounded();
    
    pub static ref MESSAGE_SENDER: Sender<AppMessage> = CHANNEL.0.clone();
    pub static ref MESSAGE_RECEIVER: Receiver<AppMessage> = CHANNEL.1.clone();
}

#[derive(Default, Clone)]
pub struct AppState {
    pub replay_dir: PathBuf,
    pub auto_fix: bool,
    pub auto_start: bool,
    pub watcher_running: bool,
    pub monitor_instance: Option<crate::monitor::MonitorInstance>,
    pub log: Vec<String>,
}

impl AppState {
    pub fn process_message(&mut self, msg: AppMessage) {
        match msg {
            AppMessage::SetReplayDir(dir) => {
                self.replay_dir = dir;
                self.log.push(format!("📂 已选择目录: {}", self.replay_dir.display()));
            }
            AppMessage::AddLog(text) => {
                self.log.push(text);
                if self.log.len() > 200 {
                    self.log.remove(0);
                }
            }
            AppMessage::SetWatcherRunning(running) => {
                self.watcher_running = running;
            }
            AppMessage::ToggleAutoFix(val) => {
                self.auto_fix = val;
            }
            AppMessage::ToggleAutoStart(val) => {
                self.auto_start = val;
            }
            AppMessage::None => {}
        }
    }
}