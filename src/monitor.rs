use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::{HashSet, HashMap};
use std::time::Duration;
use std::sync::Mutex;

use crate::config::add_log;
use crate::fixer::fix_single_file;

// 全局任务追踪
lazy_static::lazy_static! {
    static ref ACTIVE_TASKS: Mutex<HashMap<PathBuf, std::thread::ThreadId>> = Mutex::new(HashMap::new());
}

#[derive(Clone)]
pub struct MonitorInstance {
    stop_flags: Vec<Arc<AtomicBool>>,
    monitor_threads: Vec<std::thread::ThreadId>,
}

impl MonitorInstance {
    pub fn stop(&self) {
        // 停止所有监控线程
        for flag in &self.stop_flags {
            flag.store(true, Ordering::SeqCst);
        }
        
        // 清理待处理任务
        {
            let mut tasks = ACTIVE_TASKS.lock().unwrap();
            tasks.clear();
            add_log("🛑 任务队列已清空，不再接受新任务".to_string());
        }
        
        // 等待监控线程退出
        std::thread::sleep(Duration::from_millis(100));
        
        add_log("🛑 所有监控已停止，任务已清理".to_string());
    }

    pub fn is_running(&self) -> bool {
        self.stop_flags.iter().any(|f| !f.load(Ordering::SeqCst))
    }
}

// 监控多个目录
pub fn start_watch_multiple(dirs: Vec<PathBuf>) -> anyhow::Result<MonitorInstance> {
    let mut stop_flags = Vec::new();
    let mut monitor_threads = Vec::new();
    
    for dir in dirs {
        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_flag_clone = stop_flag.clone();
        let dir_clone = dir.clone();

        let handle = std::thread::Builder::new()
            .name(format!("monitor-{}", dir_clone.file_name().unwrap().to_str().unwrap()))
            .spawn(move || {
                add_log(format!("[监控]开始监控目录: {}", dir_clone.display()));
                let mut known_files = HashSet::new();

                scan_dir(&dir_clone, &mut known_files);

                while !stop_flag_clone.load(Ordering::SeqCst) {
                    std::thread::sleep(Duration::from_secs(1));
                    
                    // 检查是否已停止
                    if stop_flag_clone.load(Ordering::SeqCst) {
                        break;
                    }
                    
                    let mut current_files = HashSet::new();
                    scan_dir(&dir_clone, &mut current_files);

                    for path in current_files.difference(&known_files) {
                        // 检查是否已停止
                        if stop_flag_clone.load(Ordering::SeqCst) {
                            break;
                        }
                        
                        if path.extension().and_then(|e| e.to_str()) == Some("SC2Replay") {
                            let path_clone = path.clone();
                            let stop_flag_check = stop_flag_clone.clone();
                            
                            let task_handle = std::thread::spawn(move || {
                                // 防抖延迟前检查
                                if stop_flag_check.load(Ordering::SeqCst) {
                                    return;
                                }
                                
                                std::thread::sleep(Duration::from_millis(500));
                                
                                // 处理前再次检查
                                if stop_flag_check.load(Ordering::SeqCst) {
                                    return;
                                }
                                
                                // 检查是否被标记为停止
                                {   
                                    let tasks = ACTIVE_TASKS.lock().unwrap();
                                    if tasks.get(&path_clone).is_none() {
                                        add_log(format!("[取消]任务已取消: {}", path_clone.file_name().unwrap().to_str().unwrap_or("unknown")));
                                        return;
                                    }
                                }
                                
                                // 再次检查停止标志
                                if stop_flag_check.load(Ordering::SeqCst) {
                                    add_log(format!("[停止]任务已停止: {}", path_clone.file_name().unwrap().to_str().unwrap_or("unknown")));
                                    return;
                                }
                                
                                if let Err(e) = fix_single_file(&path_clone) {
                                    add_log(format!("[失败]处理失败 {}: {}", path_clone.display(), e));
                                } else {
                                    add_log(format!("[成功]修复成功: {}", path_clone.file_name().unwrap().to_str().unwrap()));
                                }
                            });
                            
                            // 记录任务
                            let mut tasks = ACTIVE_TASKS.lock().unwrap();
                            tasks.insert(path.clone(), task_handle.thread().id());
                            
                            // 任务完成后移除记录
                            let path_clone2 = path.clone();
                            std::thread::spawn(move || {
                                // 等待任务完成
                                let _ = task_handle.join();
                                // 移除任务记录
                                if let Ok(mut tasks) = ACTIVE_TASKS.lock() {
                                    tasks.remove(&path_clone2);
                                }
                            });
                        }
                    }

                    known_files = current_files;
                }

                add_log(format!("🛑 监控线程退出: {}", dir_clone.display()));
            })?;
        
        monitor_threads.push(handle.thread().id());
        stop_flags.push(stop_flag);
    }

    Ok(MonitorInstance { 
        stop_flags,
        monitor_threads 
    })
}

pub fn start_watch_async(dir: PathBuf) -> anyhow::Result<MonitorInstance> {
    start_watch_multiple(vec![dir])
}

fn scan_dir(dir: &PathBuf, files: &mut HashSet<PathBuf>) {
    // 不在这里检查是否停止，让调用方负责检查
    
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                files.insert(path);
            }
        }
    }
}

// 兼容原有接口
pub fn start_watch(dir: PathBuf, stop_rx: crossbeam_channel::Receiver<()>) -> anyhow::Result<MonitorInstance> {
    let instance = start_watch_async(dir.clone())?;
    let instance_clone = instance.clone();
    
    std::thread::spawn(move || {
        let _ = stop_rx.recv();
        instance_clone.stop();
    });
    
    Ok(instance)
}