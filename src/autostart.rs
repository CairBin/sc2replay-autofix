#[cfg(windows)]
pub fn set_auto_start(enable: bool) -> anyhow::Result<()> {
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};
    use std::env;
    use std::path::PathBuf;

    let exe_path = env::current_exe()?;
    let exe_str = exe_path.to_str().ok_or_else(|| anyhow::anyhow!("无效的程序路径"))?;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu.create_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Run")?;

    if enable {
        key.set_value("SC2ReplayFixer", &exe_str)?;
    } else {
        key.delete_value("SC2ReplayFixer").ok();
    }

    Ok(())
}

/// 检查当前程序是否已设置为开机自启动
#[cfg(windows)]
pub fn get_auto_start_status() -> bool {
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};
    use std::env;
    
    // 获取当前程序路径
    match env::current_exe() {
        Ok(exe_path) => {
            match exe_path.to_str() {
                Some(exe_str) => {
                    // 打开注册表
                    match RegKey::predef(HKEY_CURRENT_USER)
                        .open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Run")
                    {
                        Ok(key) => {
                            // 尝试读取注册表值
                            match key.get_value::<String, _>("SC2ReplayFixer") {
                                Ok(saved_path) => saved_path == exe_str,
                                Err(_) => false,
                            }
                        }
                        Err(_) => false,
                    }
                }
                None => false,
            }
        }
        Err(_) => false,
    }
}

#[cfg(not(windows))]
pub fn set_auto_start(_enable: bool) -> anyhow::Result<()> {
    Ok(())
}