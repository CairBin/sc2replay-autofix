mod autostart;
mod config;
mod fixer;
mod message;
mod monitor;
mod utils;

use config::{add_log, document_dir, find_sc2_replay_dirs};
use eframe::egui;
use message::{AppMessage, AppState, MESSAGE_RECEIVER, MESSAGE_SENDER};
use rfd::FileDialog;
use std::path::PathBuf;
use std::time::Duration;

struct SC2ReplayFixerApp {
    state: AppState,
    all_replay_dirs: Vec<PathBuf>, // 存储所有找到的Replays目录
}

impl Default for SC2ReplayFixerApp {
    fn default() -> Self {
        let base_dir = document_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("StarCraft II");

        let all_replay_dirs = find_sc2_replay_dirs(&base_dir);

        // 检查当前是否已设置开机自启动
        let auto_start_enabled = autostart::get_auto_start_status();
        
        Self {
            state: AppState {
                replay_dir: base_dir,
                auto_fix: true,
                auto_start: auto_start_enabled,
                watcher_running: false,
                monitor_instance: None,
                log: vec![
                    "🚀 SC2Replay修复工具已启动".to_string(),
                    format!("📂 找到{}个录像目录", all_replay_dirs.len()),
                ],
            },
            all_replay_dirs,
        }
    }
}

impl eframe::App for SC2ReplayFixerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 处理消息队列
        while let Ok(msg) = MESSAGE_RECEIVER.try_recv() {
            self.state.process_message(msg);
        }

        // 强制UI刷新
        ctx.request_repaint_after(Duration::from_millis(5));

        egui::CentralPanel::default().show(ctx, |ui| {
            // 设置背景色防止黑屏
            ui.visuals_mut().panel_fill = egui::Color32::from_rgb(248, 248, 248);

            // 标题
            ui.heading("SC2Replay自动修复工具");
            ui.separator();

            // 基础目录选择
            ui.horizontal(|ui| {
                ui.label("SC2基础目录:");
                let dir_str = self.state.replay_dir.to_str().unwrap_or("").to_string();
                let mut dir_edit = dir_str.clone();
                ui.text_edit_singleline(&mut dir_edit);

                if ui.button("选择目录").clicked() {
                    std::thread::spawn(|| {
                        if let Some(dir) = FileDialog::new().pick_folder() {
                            let all_replay_dirs = find_sc2_replay_dirs(&dir);
                            config::set_replay_dir(dir);

                            // 更新所有录像目录
                            let mut app = SC2ReplayFixerApp::default();
                            app.all_replay_dirs = all_replay_dirs;

                            //add_log(format!("📂 重新扫描到{}个录像目录", all_replay_dirs.len()));
                        }
                    });
                }
            });

            // 显示找到的录像目录
            ui.collapsing(
                format!("已找到{}个录像目录", self.all_replay_dirs.len()),
                |ui| {
                    for dir in &self.all_replay_dirs {
                        ui.label(dir.to_str().unwrap_or(""));
                    }
                },
            );

            ui.add_space(10.0);

            // 功能开关
            let mut auto_fix = self.state.auto_fix;
            // 如果监控正在运行，则禁用复选框
            let checkbox_response = if self.state.watcher_running {
                ui.scope(|ui| {
                    ui.set_enabled(false);
                    ui.checkbox(&mut auto_fix, "自动修复新录像")
                }).inner
            } else {
                ui.checkbox(&mut auto_fix, "自动修复新录像")
            };
            
            if checkbox_response.changed() {
                self.state.auto_fix = auto_fix;
                let _ = MESSAGE_SENDER.send(AppMessage::ToggleAutoFix(auto_fix));
            }

            let mut auto_start = self.state.auto_start;
            if ui.checkbox(&mut auto_start, "开机自动启动").changed() {
                self.state.auto_start = auto_start;
                let _ = MESSAGE_SENDER.send(AppMessage::ToggleAutoStart(auto_start));
            }

            if ui.button("保存设置").clicked() {
                let auto_start = self.state.auto_start;
                std::thread::spawn(move || match autostart::set_auto_start(auto_start) {
                    Ok(_) => add_log("✅ 设置已保存".to_string()),
                    Err(e) => add_log(format!("❌ 设置失败: {}", e)),
                });
            }

            ui.add_space(10.0);

            // 操作按钮
            ui.horizontal(|ui| {
                // 批量修复所有目录
                if ui.button("批量修复所有录像").clicked() {
                    let dirs = self.all_replay_dirs.clone();
                    std::thread::spawn(move || {
                            add_log("[处理] 开始批量修复所有目录...".to_string());
                            // 如果fixer.rs有batch_fix_dirs函数（接收目录列表）
                            if let Err(e) = fixer::batch_fix_dirs(&dirs) {
                                // 传入&[PathBuf]切片
                                add_log(format!("[失败] 批量修复失败: {}", e));
                            } else {
                                add_log("[成功] 所有目录修复完成".to_string());
                            }
                        });
                }

                // 监控开关
                if self.state.watcher_running {
                    if ui
                        .button(
                            egui::RichText::new("停止监控")
                                .color(egui::Color32::from_rgb(220, 0, 0)),
                        )
                        .clicked()
                    {
                        // 立即停止监控
                        if let Some(instance) = &self.state.monitor_instance {
                            instance.stop();
                        }

                        // 强制更新状态
                        self.state.watcher_running = false;
                        self.state.monitor_instance = None;

                        // 额外清理
                        add_log("🛑 监控已停止，禁止新任务创建".to_string());
                    }
                } else {
                    // 只有当自动修复复选框被勾选时才启用启动监控按钮
                    let start_btn = ui.scope(|ui| {
                        ui.set_enabled(self.state.auto_fix);
                        ui.button(
                            egui::RichText::new("启动监控").color(egui::Color32::from_rgb(0, 160, 0)),
                        )
                    }).inner;
                    
                    if start_btn.clicked() {
                        let dirs = self.all_replay_dirs.clone();
                        if dirs.is_empty() {
                            add_log("❌ 未找到任何录像目录".to_string());
                            return;
                        }

                        // 立即更新UI状态
                        self.state.watcher_running = true;
                        let ctx_clone = ctx.clone();

                        // 同步启动监控，确保正确保存实例
                        match monitor::start_watch_multiple(dirs) {
                            Ok(instance) => {
                                // 保存监控实例到当前UI状态
                                self.state.monitor_instance = Some(instance);
                                add_log("[成功] 多目录监控启动成功".to_string());
                            }
                            Err(e) => {
                                add_log(format!("[失败] 监控启动失败: {}", e));
                                self.state.watcher_running = false;
                            }
                        }
                        ctx.request_repaint();
                    }
                }
            });

            ui.add_space(20.0);

            // 日志区域
            ui.group(|ui| {
                ui.label("操作日志:");
                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for line in &self.state.log {
                            // 日志颜色区分
                            let text = if line.starts_with("[失败]") {
                                egui::RichText::new(line).color(egui::Color32::RED)
                            } else if line.starts_with("[成功]") {
                                egui::RichText::new(line).color(egui::Color32::GREEN)
                            } else if line.starts_with("🔄") || line.contains("批量修复") {
                                egui::RichText::new(line).color(egui::Color32::BLUE)
                            } else if line.starts_with("[监控]") || line.starts_with("[停止]") {
                                egui::RichText::new(line)
                                    .color(egui::Color32::from_rgb(255, 140, 0))
                            } else {
                                egui::RichText::new(line)
                            };
                            ui.label(text);
                        }
                    });
            });

            ui.add_space(10.0);
            ui.label("[提示] 本地处理，文件不上传 | 仅支持SC2.5.0.15.95687版本");
        });
    }
}

fn load_global_font(ctx: &egui::Context) {
    let mut fonts = eframe::egui::FontDefinitions::default();
    
    // 使用微软雅黑字体作为主要字体
    fonts.font_data.insert(
        "msyh".to_owned(),
        eframe::egui::FontData::from_static(include_bytes!("C:\\Windows\\Fonts\\msyh.ttc")),
    );
    
    // 配置比例字体
    let proportional_fonts = fonts
        .families
        .get_mut(&eframe::egui::FontFamily::Proportional)
        .unwrap();
    
    // 将微软雅黑字体添加到比例字体列表的开头，作为首选字体
    proportional_fonts.insert(0, "msyh".to_owned());
    
    // 配置等宽字体
    let monospace_fonts = fonts
        .families
        .get_mut(&eframe::egui::FontFamily::Monospace)
        .unwrap();
    
    // 将微软雅黑字体添加到等宽字体列表的开头
    monospace_fonts.insert(0, "msyh".to_owned());
    
    // 应用字体配置
    ctx.set_fonts(fonts);
    
    // 可以考虑调整字体大小以获得更好的对齐效果
    // 这里使用默认大小，但通过统一字体确保更好的对齐
}

fn main() -> anyhow::Result<()> {
    let native_options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(900.0, 700.0)),
        default_theme: eframe::Theme::Light,
        vsync: true,
        renderer: eframe::Renderer::Glow,
        ..eframe::NativeOptions::default()
    };

    eframe::run_native(
        "SC2Replay修复工具",
        native_options,
        Box::new(|_cc| {
            load_global_font(&_cc.egui_ctx);
            Box::new(SC2ReplayFixerApp::default())
        }),
    )
    .map_err(|e| anyhow::anyhow!("GUI启动失败: {}", e))?;

    Ok(())
}
