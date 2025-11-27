use lazy_static::lazy_static;
use std::sync::Mutex;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use crate::message::{AppMessage, MESSAGE_SENDER};

lazy_static! {
    pub static ref GLOBAL_STATE: Mutex<crate::message::AppState> = Mutex::new(crate::message::AppState::default());
}

pub fn add_log(text: String) {
    // 使用clone避免值移动
    let _ = MESSAGE_SENDER.send(AppMessage::AddLog(text.clone()));
    
    // 兼容原有全局状态
    if let Ok(mut state) = GLOBAL_STATE.lock() {
        state.log.push(text);
        if state.log.len() > 200 {
            state.log.remove(0);
        }
    }
}

pub fn set_replay_dir(dir: PathBuf) {
    // 使用clone避免值移动
    let _ = MESSAGE_SENDER.send(AppMessage::SetReplayDir(dir.clone()));
    
    if let Ok(mut state) = GLOBAL_STATE.lock() {
        state.replay_dir = dir;
    }
}

pub fn set_watcher_running(running: bool) {
    let _ = MESSAGE_SENDER.send(AppMessage::SetWatcherRunning(running));
    
    if let Ok(mut state) = GLOBAL_STATE.lock() {
        state.watcher_running = running;
    }
}

// 递归查找所有Replays/Multiplayer目录
pub fn find_sc2_replay_dirs(base_dir: &Path) -> Vec<PathBuf> {
    let mut replay_dirs = Vec::new();
    
    if !base_dir.exists() {
        return replay_dirs;
    }
    
    // 遍历Accounts目录下的所有子目录
    let accounts_dir = base_dir.join("Accounts");
    if accounts_dir.exists() {
        for entry in WalkDir::new(&accounts_dir)
            .min_depth(1)
            .max_depth(10)
            .into_iter()
            .filter_map(|e| e.ok()) {
            
            let path = entry.path();
            if path.is_dir() {
                // 检查是否包含Replays/Multiplayer子目录
                let replay_path = path.join("Replays").join("Multiplayer");
                if replay_path.exists() {
                    replay_dirs.push(replay_path);
                }
            }
        }
    }
    
    // 如果没找到，尝试基础目录下的Replays
    if replay_dirs.is_empty() {
        let default_replay = base_dir.join("Replays").join("Multiplayer");
        if default_replay.exists() {
            replay_dirs.push(default_replay);
        }
    }
    
    replay_dirs
}

pub fn find_sc2_replay_dir() -> Option<PathBuf> {
    document_dir().map(|d| d.join("StarCraft II"))
}

pub fn document_dir() -> Option<PathBuf> {
    dirs::document_dir()
}