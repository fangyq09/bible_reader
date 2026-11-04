use eframe::egui;
use crate::notes::{Notedb};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use pulldown_cmark::{Parser, Event};

#[derive(PartialEq, Clone, Copy)]
pub enum BodyViewMode { 
    Markdown,
    PlainText,
}
pub struct NoteReaderApp {
    pub note: Notedb,
		pub markdown_cache: CommonMarkCache,
		pub view_mode: BodyViewMode,
}

pub fn configure_reader_visuals(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::light();
    //visuals.panel_fill = egui::Color32::from_rgb(232, 245, 233);
    //visuals.window_fill = egui::Color32::from_rgb(245, 250, 245);
    //visuals.override_text_color = Some(egui::Color32::BLACK);
    visuals.panel_fill = egui::Color32::from_rgb(250, 250, 249);
    visuals.window_fill = egui::Color32::from_rgb(255, 255, 255);
    visuals.override_text_color = Some(egui::Color32::from_rgb(40, 44, 52));
    ctx.set_visuals(visuals);
}

impl eframe::App for NoteReaderApp {
	fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
		// 底部操作按钮面板（删除、复制、编辑）
		egui::TopBottomPanel::bottom("reader_bottom_panel").show(ctx, |ui| {
			ui.add_space(5.0);
			ui.horizontal(|ui| {
				let btn_w = 80.0;
				let btn_h = 28.0;

				// 左侧：删除按钮（固定操作 notes 表）
				ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
					if ui.add_sized([btn_w, btn_h], egui::Button::new("🗑删除")).clicked() {
						if let Err(e) = crate::notes::delete_note("notes", &self.note.id) {
							eprintln!("删除笔记失败: {:?}", e);
						} else {
							println!("已删除笔记 id={}", self.note.id);
							ctx.send_viewport_cmd(egui::ViewportCommand::Close);
						}
					}
				});

				// 右侧：复制与编辑按钮
				ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
					// 编辑按钮
					if ui.add_sized([btn_w, btn_h], egui::Button::new("✏编辑")).clicked() {
						// 1. 将完整的笔记对象序列化为 JSON
						if let Ok(note_json) = serde_json::to_string(&self.note) {
							// 2. 获取当前可执行文件路径
							if let Ok(exe) = std::env::current_exe() {
								// 3. 使用与 open_noteapp_window 完全一致的参数结构拉起编辑器进程，category 固定为 "notes"
								if let Err(e) = std::process::Command::new(exe)
									.arg("--note-window")
										.arg("--note-json")
										.arg(note_json)
										.arg("--category")
										.arg("notes")
										.spawn()
								{
									eprintln!("无法启动笔记窗口: {e}");
								}
							}
						}
						// 4. 关闭当前的阅读窗口
						ctx.send_viewport_cmd(egui::ViewportCommand::Close);
					}

					ui.add_space(20.0);

					let body = self.note.body.as_deref().unwrap_or("");

					// 根据当前的视图模式（Markdown 还是 纯文本）动态渲染不同的复制按钮
					match self.view_mode {
						BodyViewMode::Markdown => {
							// Markdown 模式下：显示两个按钮（复制源代码 & 复制去符号纯文本）
							if ui.add_sized([btn_w + 15.0, btn_h], egui::Button::new("📋复制纯文本")).clicked() {
								let plain = strip_markdown(body);
								ctx.copy_text(plain);
							}
							ui.add_space(10.0);
							if ui.add_sized([btn_w + 15.0, btn_h], egui::Button::new("📋复制源代码")).clicked() {
								ctx.copy_text(body.to_string());
							}
						}
						BodyViewMode::PlainText => {
							// 纯文本模式下：只显示一个常规的复制按钮
							if ui.add_sized([btn_w, btn_h], egui::Button::new("📋复制")).clicked() {
								ctx.copy_text(body.to_string());
							}
						}
					}
				});
			});
			ui.add_space(5.0);
		});

		// 中央内容展示面板
		egui::CentralPanel::default().show(ctx, |ui| {
			ui.heading(self.note.title.as_deref().unwrap_or("无标题笔记"));

			//ui.horizontal(|ui| {
			//	// 检查 reference 是否存在且不为空字符串
			//	if let Some(ref_text) = self.note.reference.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
			//		ui.label(egui::RichText::new(format!("📍 引用: {ref_text}")).weak());
			//	}
			//	if let Some(ref time) = self.note.created_at {
			//		//ui.label(egui::RichText::new(format!("🕒 {time}")).weak());
			//		let date_only = if time.len() >= 10 { &time[0..10] } else { time };
			//		ui.label(egui::RichText::new(format!("🕒 {date_only}")).weak());
			//	}

			//	// 靠右对齐显示视图切换下拉菜单
			//	ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
			//		egui::ComboBox::from_id_salt("view_mode_selector")
			//			.selected_text(match self.view_mode {
			//				BodyViewMode::PlainText => "📄 纯文本视图",
			//				BodyViewMode::Markdown => "✨ Markdown 视图",
			//			})
			//		.show_ui(ui, |ui| {
			//			ui.selectable_value(&mut self.view_mode, BodyViewMode::PlainText, "📄 纯文本视图");
			//			ui.selectable_value(&mut self.view_mode, BodyViewMode::Markdown, "✨ Markdown 视图");
			//		});
			//	});
			//});
			ui.horizontal(|ui| {
				// 检查 reference 是否存在且不为空字符串
				if let Some(ref_text) = self.note.reference.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
					ui.label(egui::RichText::new(format!("📍 引用: {ref_text}")).weak());
				}
				if let Some(ref time) = self.note.created_at {
					let date_only = if time.len() >= 10 { &time[0..10] } else { time };
					ui.label(egui::RichText::new(format!("🕒 {date_only}")).weak());
				}

				ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
					let display_name = match self.view_mode {
						BodyViewMode::PlainText => "📄 纯文本视图",
						BodyViewMode::Markdown => "✨Markdown视图",
					};

					ui.menu_button(display_name, |ui| {
						if ui.button("📄 纯文本视图").clicked() {
							self.view_mode = BodyViewMode::PlainText;
							ui.close_kind(egui::UiKind::Menu)
						}
						if ui.button("✨Markdown视图").clicked() {
							self.view_mode = BodyViewMode::Markdown;
							ui.close_kind(egui::UiKind::Menu)
						}
					});
				});
			});

			ui.separator();

			// 评论正文只读展示区域
			egui::ScrollArea::vertical().show(ui, |ui| {
				let body = self.note.body.as_deref().unwrap_or("（无内容）");
				match self.view_mode {
					BodyViewMode::Markdown => {
						CommonMarkViewer::new().show(ui, &mut self.markdown_cache, body);
					}
					BodyViewMode::PlainText => {
						ui.add( egui::Label::new(egui::RichText::new(body)).selectable(true));
					}
				}
			});
		});
	}
}

fn strip_markdown(markdown: &str) -> String {
    let parser = Parser::new(markdown);
    let mut plain_text = String::new();
    
    for event in parser {
        match event {
            Event::Text(text) | Event::Code(text) => {
                plain_text.push_str(&text);
            }
            Event::SoftBreak | Event::HardBreak => {
                plain_text.push('\n');
            }
            Event::End(tag) => {
                match tag {
                    pulldown_cmark::TagEnd::Paragraph 
                    | pulldown_cmark::TagEnd::Heading(_) // 🌟 Fixed: Added (_) for tuple variant
                    | pulldown_cmark::TagEnd::Item 
                    | pulldown_cmark::TagEnd::CodeBlock => {
                        plain_text.push('\n');
                    }
                    _ => {}
                }
            }
            Event::Start(tag) => {
                match tag {
                    pulldown_cmark::Tag::Paragraph 
                    | pulldown_cmark::Tag::Heading { .. } => {
                        if !plain_text.is_empty() && !plain_text.ends_with('\n') {
                            plain_text.push('\n');
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    
    plain_text.trim().to_string()
}
