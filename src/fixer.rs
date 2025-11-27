use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use anyhow::Context;
use walkdir::WalkDir;

use crate::config::add_log;

// 对被选中的目录下的录像进行修复
pub fn batch_fix_dir(dir: &Path)->anyhow::Result<()>{
    if !dir.exists(){
        return Err(anyhow::anyhow!("目录不存在: {}", dir.display()));
    }

    for entry in walkdir::WalkDir::new(dir).min_depth(1).max_depth(1){
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("SC2Replay"){
            if let Err(e) = fix_single_file(path) {
                let err_msg = format!("[失败]修复失败 {}: {}", path.display(), e);
                add_log(err_msg);
            } else {
                // 记录成功修复的日志
                add_log(format!("[成功]修复成功: {}", path.file_name().unwrap().to_str().unwrap_or("unknown")));
            }
        }
    }

    Ok(())
}

// 分别选中所有账户的录像目录
pub fn batch_fix_dirs(dirs:&[PathBuf])->anyhow::Result<()>{    
    let mut any_error = false;
    
    for dir in dirs{
        if dir.exists(){
            add_log(format!("进入录像目录：{}", dir.display()));
            // 不使用?操作符，而是显式处理错误，确保即使一个目录失败也能继续处理其他目录
            if let Err(e) = batch_fix_dir(dir) {
                add_log(format!("[失败]处理目录{}时出错: {}", dir.display(), e));
                any_error = true;
            }
        }
    }
    
    // 如果有任何错误，返回错误信息
    if any_error {
        Err(anyhow::anyhow!("部分目录修复失败，请查看日志详情"))
    } else {
        Ok(())
    }
}



// 修复常量
const SEARCH_BYTES: &[u8] = &[0x09, 0x00, 0x04, 0x09, 0x00, 0x06, 0x09, 0x00];
const TARGET_BYTES: &[u8] = &[0x09, 0x0A, 0x04, 0x09, 0x00, 0x06, 0x09, 0x1E];
const SCAN_LIMIT: usize = 128;  // 前128字节

pub fn fix_single_file(input_path: &Path) -> anyhow::Result<()>{
    // 跳过名称后面为-FIXED的录像，因为这表示此录像已经被修复
    if input_path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.contains("-FIXED"))
        .unwrap_or(false){
            return Ok(())
        }
    
    // 检验文件类型
    if input_path.extension().and_then(|e| e.to_str()) != Some("SC2Replay"){
        return Err(anyhow::anyhow!(format!("此文件不是SC2Replay文件，文件为{}", input_path.display())));
    }

    // 如果是录像文件则读取
    let mut data = Vec::new();
    File::open(input_path)
        .with_context(|| format!("无法打开文件: {}", input_path.display() ))?
        .read_to_end(&mut data)?;

    // 找查目标字节序列
    let offset = find_bytes_offset(&data, SEARCH_BYTES)
        .with_context(|| "未找到目标字节序列，可能录像能够正常工作")?;

    // 替换字节序列
    data.splice(offset..offset + SEARCH_BYTES.len(), TARGET_BYTES.iter().cloned());
    // 生成输出路径
    let output_path = generate_output_path(input_path);

    // 写入修复后的文件
    File::create(&output_path)
        .with_context(|| format!("无法创建文件: {}", output_path.display()))?
        .write_all(&data)?;

    add_log(format!("修复完成: {}", output_path.display()));
    Ok(())
}

// 查找字节序列偏移
fn find_bytes_offset(data: &[u8], search: &[u8]) -> Option<usize> {
    let search_len = search.len();
    let data_len = data.len().min(SCAN_LIMIT);

    for i in 0..=data_len - search_len {
        if data[i..i + search_len] == *search {
            return Some(i);
        }
    }

    None
}


// 生成修复后的文件路径
fn generate_output_path(input_path: &Path) -> PathBuf {
    let stem = input_path
        .file_stem()
        .unwrap_or_default()
        .to_str()
        .unwrap_or("fixed");
    let parent = input_path.parent().unwrap_or(Path::new("."));
    parent.join(format!("{}-FIXED.SC2Replay", stem))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_find_bytes_offset() {
        // 测试字节序列查找
        let data = vec![0x00, 0x01, 0x09, 0x00, 0x04, 0x09, 0x00, 0x06, 0x09, 0x00, 0x02];
        let offset = find_bytes_offset(&data, SEARCH_BYTES);
        
        assert_eq!(offset, Some(2));
    }

    #[test]
    fn test_generate_output_path() {
        let input_path = PathBuf::from("test.SC2Replay");
        let output_path = generate_output_path(&input_path);
        
        assert_eq!(output_path, PathBuf::from("test-FIXED.SC2Replay"));
    }
}