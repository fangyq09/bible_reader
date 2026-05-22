use eframe::egui;
use egui::{RichText,ScrollArea};
use crate::notes::{Notedb,AppConfig,save_note,delete_note,sync_to_turso,delete_from_turso};

pub struct NoteApp {
		pub note: Notedb,
		pub config: AppConfig,
}
fn note_visuals() -> egui::Visuals {
	let mut v = egui::Visuals::light();

	// ====== 背景 ======
	v.window_fill = egui::Color32::from_rgb(248, 248, 245);
	v.panel_fill  = egui::Color32::from_rgb(248, 248, 245);
	v.extreme_bg_color = egui::Color32::from_rgb(235, 235, 230);

	// ====== 分割线 ======
	v.widgets.noninteractive.bg_stroke =
		egui::Stroke::new(1.0, egui::Color32::from_rgb(220, 220, 215));

	// ====== 普通控件 ======
	v.widgets.inactive.bg_fill = egui::Color32::from_rgb(242, 242, 238);
	//v.widgets.inactive.bg_fill = egui::Color32::from_rgb(248, 248, 245);

	v.widgets.hovered.bg_fill = egui::Color32::from_rgb(230, 230, 225);

	v.widgets.active.bg_fill = egui::Color32::from_rgb(220, 220, 215);

	// ====== 输入框 ======
	v.widgets.inactive.bg_stroke =
		egui::Stroke::new(1.0, egui::Color32::from_rgb(200, 200, 195));

	//v.widgets.inactive.bg_stroke = egui::Stroke::NONE; // 去掉默认边框

	v.widgets.hovered.bg_stroke =
		egui::Stroke::new(1.0, egui::Color32::from_rgb(180, 180, 175));

	// ====== 选中文本 ======
	v.selection.bg_fill =
		egui::Color32::from_rgb(180, 205, 235);

	v.selection.stroke =
		egui::Stroke::new(1.0, egui::Color32::from_rgb(120, 160, 210));

	v
}
impl eframe::App for NoteApp {
	fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

		ctx.set_visuals(note_visuals());

		let note = &mut self.note;

		egui::TopBottomPanel::bottom("note_bottom_panel").show(ctx, |ui| {
			ui.add_space(5.0);
			ui.horizontal(|ui| {
				let btn_w = 80.0;
				let btn_h = 28.0;
				ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
					if ui.add_sized([btn_w, btn_h], egui::Button::new("🗑删除"))
						.on_hover_cursor(egui::CursorIcon::Default)
							.clicked() {
								if let Err(e) = delete_note("notes", &note.id) {
									eprintln!("删除笔记失败 id={}: {:?}", note.id, e);
								} else {
									println!("删除笔记 id={}", note.id);
										let cfg = &self.config;
										if cfg.sync_enabled && !cfg.turso_url.is_empty() && !cfg.turso_token.is_empty() {
											let cfg_clone = cfg.clone();
											let id_clone = note.id.clone();
											let ctx_clone = ctx.clone();
											tokio::spawn(async move {
												//match delete_from_turso("notes", &id_clone, &cfg_clone).await {
												//	Ok(_) => {
												//		println!("✅ 云端同步删除成功 id={}", id_clone);
												//		ctx_clone.request_repaint(); 
												//	}
												//	Err(e) => eprintln!("❌ 云端同步删除失败: {:?}", e),
												//}
												let conn_result = crate::notes::get_or_create_conn(
                        None, 
                        cfg_clone.turso_url.clone(), 
                        cfg_clone.turso_token.clone()
                    ).await;

                    match conn_result {
                        Ok(conn) => {
                            // 2. 传入连接引用 &conn
                            match delete_from_turso("notes", &id_clone, &conn).await {
                                Ok(_) => {
                                    println!("✅ 云端同步删除成功 id={}", id_clone);
                                    ctx_clone.request_repaint(); 
                                    // 如果删除后需要关闭窗口，取消下面这行的注释
                                    // ctx_clone.send_viewport_cmd(egui::ViewportCommand::Close);
                                }
                                Err(e) => eprintln!("❌ 云端删除失败: {:?}", e),
                            }
                        }
                        Err(e) => eprintln!("❌ 无法连接云端执行删除: {:?}", e),
                    }
											});
										}
								}
									//ctx.request_repaint_of(egui::ViewportId::ROOT);
									//ctx.send_viewport_cmd(egui::ViewportCommand::Close);
							}
				});
				ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
					if ui.add_sized([btn_w, btn_h], egui::Button::new("保存"))
						.on_hover_cursor(egui::CursorIcon::Default)
							.clicked() {
								// 1. 先执行原有的本地保存逻辑
								save_note("notes", note);
								// 2. 开启后台异步任务进行同步
								let cfg = &self.config;
								if cfg.sync_enabled && !cfg.turso_url.is_empty() && !cfg.turso_token.is_empty() {
								let note_clone = note.clone();
								let cfg_clone = self.config.clone();
								let ctx_clone = ctx.clone();
									tokio::spawn(async move {
										//match sync_to_turso("notes", &note_clone, &cfg_clone).await {
										//	Ok(_) => {
										//		println!("云端同步成功！");
										//		ctx_clone.send_viewport_cmd(egui::ViewportCommand::Close);
										//	}
										//	Err(e) => {
										//		eprintln!("云端同步失败: {}", e);
										//		ctx_clone.send_viewport_cmd(egui::ViewportCommand::Close);
										//	}
										//}
										let conn_result = crate::notes::get_or_create_conn(
											None, 
											cfg_clone.turso_url.clone(), 
											cfg_clone.turso_token.clone()
										).await;

										match conn_result {
											Ok(conn) => {
												// 这里依然能享受到复用：sync_to_turso 会使用上面刚建好的这一个 conn
												let _ = sync_to_turso("notes", &note_clone, &conn).await;
												ctx_clone.send_viewport_cmd(egui::ViewportCommand::Close);
											}
											Err(e) => {
												eprintln!("云端连接失败: {}", e);
												ctx_clone.send_viewport_cmd(egui::ViewportCommand::Close);
											}
										}
									});
								} else {
									ctx.request_repaint_of(egui::ViewportId::ROOT);
									ctx.send_viewport_cmd(egui::ViewportCommand::Close);
								}
					}
				});
			});
			ui.add_space(2.0);
		});

		egui::CentralPanel::default().show(ctx, |ui| {
			let label_width = 90.0;
			ui.collapsing("笔记标题", |ui| {
				ui.horizontal(|ui| {
					ui.add_sized([label_width, 0.0],
						egui::Label::new(RichText::new("主题：").size(14.0)));
					let subject_text_edit = egui::TextEdit::singleline(
						note.subject.get_or_insert(String::new())).hint_text("例如：查经，灵修")
						.desired_width(ui.available_width());
					ui.add(subject_text_edit);
				});
				ui.horizontal(|ui| {
					ui.add_sized([label_width, 0.0], 
						egui::Label::new(RichText::new("标题：").size(14.0)));
					let title_text_edit = egui::TextEdit::singleline(
						note.title.get_or_insert(String::new()))
						.desired_width(ui.available_width());
					ui.add(title_text_edit);
				});
				ui.horizontal(|ui| {
					ui.add_sized([label_width, 0.0], 
						egui::Label::new(RichText::new("关键词：").size(14.0)));
					let keyword_text_edit = egui::TextEdit::singleline(
						note.keywords.get_or_insert(String::new()))
						.desired_width(ui.available_width());
					ui.add(keyword_text_edit);
				});
				ui.horizontal(|ui| {
					ui.add_sized([label_width, 0.0],
						egui::Label::new(RichText::new("引用经文：").size(14.0)));
					let ref_text_edit = egui::TextEdit::singleline(
						note.reference.get_or_insert(String::new()))
						.desired_width(ui.available_width());
					ui.add(ref_text_edit);
				});
			});

			ui.separator();

			ScrollArea::vertical()
				.auto_shrink([false; 2]) // 允许在垂直方向填满空间
				.show(ui, |ui| {
					ui.add(
						egui::TextEdit::multiline(note.body.get_or_insert(String::new()))
						.hint_text("笔记正文")
						.frame(false)
						.desired_width(ui.available_width()) // 宽度铺满
						.desired_rows(10) // 初始最小行数
						.font(egui::FontId::proportional(16.0))
					);

				});
			//ui.separator();
		});

	}
}
