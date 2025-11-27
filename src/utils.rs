use std::path::{Path, PathBuf};
use dirs::document_dir;
use walkdir::WalkDir;

/*
    查找SC2文档目录
    C:\Users\系统用户名\Documents\StarCraft II\
*/
pub fn find_sc2_replay_dir() -> Option<PathBuf>{
    if let Some(doc_dir) = document_dir(){
        let base_dir = doc_dir.join("StarCraft II").join("Accounts");
        if let Some(path) = scan_replay_dir_recursize(&base_dir){
            return Some(path);
        }
    }
    None
}

// 扫描所有Replays/Multiplayer目录
fn scan_all_replay_dirs(base_dir: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if !base_dir.exists() {
        return dirs;
    }

    for entry in WalkDir::new(base_dir)
        .min_depth(1)
        .max_depth(10)
    {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue, // 跳过错误（如权限问题）
        };

        let path = entry.path();

        // 检查是否是Replays/Multiplayer目录
        if path.file_name() == Some(std::ffi::OsStr::new("Multiplayer")) 
            && path.parent().and_then(|p| p.file_name()) == Some(std::ffi::OsStr::new("Replays"))
            && entry.file_type().is_dir() 
        {
            dirs.push(path.to_path_buf());
        }
    }

    dirs
}

// 递归扫描所有子目录，找到第一个Replays/Multiplayer
fn scan_replay_dir_recursize(base_dir: &Path) -> Option<PathBuf>{
    if !base_dir.exists(){
        return None;
    }

    // 最大限度防止无限递归
    for entry in WalkDir::new(base_dir).min_depth(1).max_depth(10){
        let entry = entry.ok()?;
        let path = entry.path();

        // 匹配Replays/Multiplayer目录
        if path.file_name() == Some(std::ffi::OsStr::new("Multiplayer")) 
            && path.parent().and_then(|p| p.file_name()) == Some(std::ffi::OsStr::new("Replays"))
            && entry.file_type().is_dir() 
        {
            return Some(path.to_path_buf());
        }
    }
    None
}

// 获取所有SC2录像目录，支持多账号
pub fn find_all_replay_dirs() ->Vec<PathBuf>{
    let mut dirs = Vec::new();

    if let Some(doc_dir) = document_dir(){
        let base_dir = doc_dir.join("StarCraft II").join("Accounts");
        dirs.extend(scan_all_replay_dirs(&base_dir));
    }
    dirs
}


#[cfg(test)]
mod tests{
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_find_sc2_replay_dir(){
        let dir = find_sc2_replay_dir();
        println!("找到SC2录像路径: {:?}", dir);
    
        if let Some(path) = dir{
            assert!(path.to_string_lossy().contains("StarCraft II"));
            assert!(path.ends_with("Multiplayer"));
        }
    }

    #[test]
    fn test_find_all_replay_dirs() {
        let dirs = find_all_replay_dirs();
        println!("找到的所有录像目录: {:?}", dirs);
        
        // 验证返回的路径都是有效的Multiplayer目录
        for dir in dirs {
            assert!(dir.ends_with("Multiplayer"));
            assert!(dir.parent().unwrap().ends_with("Replays"));
        }
    }

}