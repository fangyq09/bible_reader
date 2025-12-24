#![windows_subsystem = "windows"]

mod theme;
mod utils;
mod notes;
mod note_app;
use std::fs;
use rusqlite::Connection;
use eframe::egui;
use egui::{FontDefinitions, FontFamily, FontId, TextStyle, TextFormat};
use egui::text::LayoutJob;
use std::path::PathBuf;
use std::collections::HashMap;
use serde_json;
use uuid::Uuid;
use chrono::Utc;
use crate::theme::{Theme, ThemeColors, apply_theme};
use crate::utils::{
	load_books,
	load_chapters,
	load_chapter_content,
	chapter_number,
	chapter_display_name,
	version_display_name,
	sort_versions_chinese_first,
	book_number_to_abbr,
	readonly_content_text_highlighted,
	highlight_search_terms,
	draw_hover_button,
};
use crate::notes::{Notedb,DisplayMode};
use crate::note_app::NoteApp;

/// 应用状态
struct BibleApp {
	theme: Theme,
	bible_root: PathBuf,
	versions: Vec<String>,
	pub current_version: String,
	books: Vec<(i32, String)>,
	chapters: Vec<String>,
	pub current_book: Option<i32>,
	pub	current_chapter: Option<String>,
	content: String,
	pub current_book_name: Option<String>,
	search_query: String,   // 搜索框内容
	search_results: Vec<(i32, String, i32, String)>,
	text_cache: HashMap<(i32, i32), String>,
	conn: Option<Connection>,  // 持久化连接
	show_search_window: bool, // 控制搜索结果窗口显示
	last_search_query: String,
	highlight_query: Option<String>,
	jump_back_stack: Vec<(String, i32, String)>,   // 译本, 书卷, 章节
	jump_forward_stack: Vec<(String, i32, String)>,
	show_version_menu: bool,
	change_version_menu: bool,
	show_settings_menu: bool,
	show_highlight: bool,
	pub show_notes: bool,
	pub last_appended_notes_chapter: Option<(i32, String)>,
	pub appended_notes_current: Vec<Notedb>,
	pub show_notes_list_window: bool,
	pub notes_cache: Vec<Notedb>,
	pub note_window_open: bool,
	pub current_note: Option<Notedb>,
	pub notes_search_keyword: String,
	pub active_search_type: String,
	editable_mode: bool,
}
///中文字体
pub fn configure_chinese_font(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "chinese_font".to_string(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/SourceHanSansCN-Regular.otf")).into(),
    );
    fonts.families.get_mut(&FontFamily::Proportional).unwrap()
        .insert(0, "chinese_font".to_string());
    fonts.families.get_mut(&FontFamily::Monospace).unwrap()
        .insert(0, "chinese_font".to_string());
    
    ctx.set_fonts(fonts);

    // ---------- 设置文本样式 ----------
    let mut style = (*ctx.style()).clone();
    style.text_styles.insert(TextStyle::Body, FontId::new(16.0, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Button, FontId::new(16.0, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Small, FontId::new(12.0, FontFamily::Proportional));
    
    ctx.set_style(style);
}
///初始化
impl BibleApp {
	fn new(cc: &eframe::CreationContext<'_>) -> Self {
		// ---------- 初始化数据目录 ----------
		let user_data_path = dirs::data_dir()
			.unwrap_or_else(|| PathBuf::from("."))
			.join("bible_reader");

		let sqlite_path = user_data_path.join("sqlite");
		let notes_path = user_data_path.join("notes");

		// 如果目录不存在就创建
		fs::create_dir_all(&sqlite_path).ok();
		fs::create_dir_all(&notes_path).ok();

		let bible_root = sqlite_path.clone();

		// ---------- 复制内置译本 ----------
		let built_in_files: Vec<(&str, &[u8])> = vec![
			("和合本.sqlite3", include_bytes!("../assets/sqlite/cunpss.sqlite3")),
			//("和修本.sqlite3", include_bytes!("../assets/sqlite/rcuvss.sqlite3")),
			//("当代译本.sqlite3", include_bytes!("../assets/sqlite/ccb.sqlite3")),
			//("niv2011.sqlite3", include_bytes!("../assets/sqlite/niv2011.sqlite3")),
			//("sg21.sqlite3", include_bytes!("../assets/sqlite/sg21.sqlite3")),
		];

		for (filename, content) in built_in_files {
			let target = sqlite_path.join(filename);
			if !target.exists() {
				fs::write(&target, content).expect("写入内置译本失败");
			}
		}

		// ---------- 加载中文字体 ----------
		configure_chinese_font(&cc.egui_ctx);

		// ---------- 读取译本 ----------
		let mut versions: Vec<String> = if let Ok(entries) = fs::read_dir(&bible_root) {
			entries
				.flatten()
				.filter_map(|e| {
					let path = e.path();
					if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
						//if ext == "db" || ext == "sqlite3" || ext == "sqlite" {
						if ext == "sqlite3" {
							return Some(path.file_name().unwrap().to_string_lossy().to_string());
						}
					}
					None
					})
				.collect()
				} else {
					Vec::new()
				};

			//versions.sort(); //字典序排列译本
			sort_versions_chinese_first(&mut versions);

			// 你想要优先加载的译本
			let preferred_version = "和合本.sqlite3".to_string();


			// 先创建 app（不加载书卷）
			let mut app = Self {
				theme: Theme::Light,
				bible_root,
				versions,
				current_version: String::new(),
				books: vec![],
				chapters: vec![],
				current_book: None,
				current_chapter: None,
				content: String::new(),
				current_book_name: Some("创世纪".to_string()),
				search_query: String::new(),
				search_results: vec![],
				text_cache: HashMap::new(),
				conn: None, 
				show_search_window: false,
				last_search_query: String::new(),
				highlight_query: None,
				jump_back_stack: Vec::new(),     
				jump_forward_stack: Vec::new(),  
				show_notes: false,
				last_appended_notes_chapter: None, 
				appended_notes_current: Vec::new(),
				current_note: None,
				show_version_menu: false,
				change_version_menu: false,
				show_settings_menu: false,
				show_highlight: false,
				show_notes_list_window: false,
				notes_cache: Vec::new(),
				note_window_open: false,
				notes_search_keyword: String::new(),
				active_search_type: String::new(),
				editable_mode: false,
			};

			// 若没有任何圣经数据库，就不加载，直接返回 app
			if app.versions.is_empty() {
				eprintln!("Warning: 未找到任何圣经数据库文件 (*.db / *.sqlite3)");
				return app;
			}

			//   选择要加载的译本
			let version_to_load = if app.versions.contains(&preferred_version) {
				preferred_version
			} else {
				// 若指定译本不存在就用第一个译本
				app.versions.first().cloned().unwrap_or_default()
			};
			//   调用 on_version_changed
			if !version_to_load.is_empty() {
				app.current_version = version_to_load.clone();

				// 打开数据库并持久化连接
				let db_path = app.bible_root.join(&app.current_version);
				let conn = Connection::open(&db_path).expect("打开数据库失败");
				app.conn = Some(conn);

				app.on_version_changed(version_to_load);
			}
			app
		}
	}

/// 搜索经文
impl BibleApp {
fn perform_search(&mut self) -> rusqlite::Result<()> {
    self.search_results.clear();
    self.text_cache.clear();
		self.highlight_query = None;

    let query = self.search_query.trim();
    if query.is_empty() { return Ok(()); }

    let conn = match &self.conn {
        Some(c) => c,
        None => {
            eprintln!("原始数据库尚未初始化！");
            return Ok(());
        }
    };

		//搜索书卷名与关键词的分隔符
		let separators = [':', '：', '&'];
		let mut book_filter = "";
		let mut content_filter = query;

		for (i, c) in query.char_indices() {
			if separators.contains(&c) {
				// i 是字节索引，c.len_utf8() 是字符长度
				book_filter = query[..i].trim();
				content_filter = query[i + c.len_utf8()..].trim();
				break;
			}
		}

		self.highlight_query = Some(content_filter.to_string());

    let mut sql = String::from(
        "
        SELECT b.number, b.human, c.reference_osis, c.content
        FROM chapters c
        JOIN books b ON c.reference_osis LIKE b.osis || '.%'
        WHERE c.content LIKE ?1
        "
    );

    if !book_filter.is_empty() {
        sql.push_str(" AND b.human LIKE ?2 ");
    }

    sql.push_str(" ORDER BY b.number, c.reference_osis ");

    let mut stmt = conn.prepare(&sql)?;

    let raw_rows: Vec<(i32, String, String, String)> = if !book_filter.is_empty() {
        stmt.query_map(
            rusqlite::params![format!("%{}%", content_filter), format!("%{}%", book_filter)],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        )?.map(|r| r.unwrap()).collect()
    } else {
        stmt.query_map(
            rusqlite::params![format!("%{}%", content_filter)],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        )?.map(|r| r.unwrap()).collect()
    };

    for (book_num, book_name, reference_osis, content) in raw_rows {
        let chap_num = reference_osis.split('.').last().unwrap_or("0").parse::<i32>().unwrap_or(0);
        let snippet = content.lines().find(|l| l.contains(content_filter)).unwrap_or(&content).to_string();
        self.search_results.push((book_num, book_name.clone(), chap_num, snippet));
        self.text_cache.entry((book_num, chap_num)).or_insert(content);
    }

    self.search_results.sort_by(|a, b| {
        let book_cmp = a.0.cmp(&b.0);
        if book_cmp == std::cmp::Ordering::Equal { a.2.cmp(&b.2) } else { book_cmp }
    });

    Ok(())
}
}

///左侧书卷栏目
impl BibleApp {
	fn ui_left_books_panel(&mut self, ctx: &egui::Context, colors: &ThemeColors) {
		let mut selected_book: Option<i32> = None;

		egui::SidePanel::left("books_panel")
			.resizable(true)
			.default_width(150.0)
			.show(ctx, |ui| {

				self.version_menu_button(ui, &colors);

				ui.separator();

				egui::ScrollArea::vertical()
					.auto_shrink([false; 2])
					.show(ui, |ui| {
						for (num, name) in &self.books {
							let is_selected = Some(*num) == self.current_book;
							let bg = if is_selected {
								colors.book_selected_bg
							} else {
								colors.book_unselected_bg
							};
							let txt_color = if is_selected {
								colors.selected_text_color
							} else {
								colors.text_color
							};
							let txt = egui::RichText::new(name.clone())
								.color(txt_color);

							if ui.add(egui::Button::new(txt).fill(bg)).clicked() {
								selected_book = Some(*num);
							}
						}
					});
			});

		if let Some(b) = selected_book {
			self.on_book_selected(b);
		}
	}
}

///中间章节栏目
	impl BibleApp {
		fn ui_left_chapters_panel(&mut self, ctx: &egui::Context, colors: &ThemeColors) {
			let mut chosen: Option<String> = None;
			let book_num = self.current_book;
			let book_abbr = &book_num
            .map(book_number_to_abbr)
            .unwrap_or("未选择");  

			egui::SidePanel::left("chapters_panel")
				.resizable(true)
				.default_width(120.0)
				.show(ctx, |ui| {
					if let Some(_book) = book_num {
						ui.label(format!("章节（{}）",book_abbr));
						ui.separator();

						egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
							for chap in &self.chapters {
								let is_selected = Some(chap) == self.current_chapter.as_ref();

								let bg = if is_selected {
									colors.chapter_selected_bg
								} else {
									colors.chapter_unselected_bg
								};
								let txt_color = if is_selected {
									colors.selected_text_color
								} else {
									colors.text_color
								};

								let txt = egui::RichText::new(chapter_display_name(chap)).color(txt_color);

								if ui.add(egui::Button::new(txt).fill(bg)).clicked() {
									chosen = Some(chap.clone());
								}
							}
						});
					}
				});

			if let (Some(book), Some(chap)) = (book_num, chosen) {
				self.on_chapter_selected(book, chap);
			}
		}
	}

///译本切换按钮
impl BibleApp {
	pub fn version_menu_button(
		&mut self,
		ui: &mut egui::Ui,
		colors: &ThemeColors,
	) {
		// 按钮
		//let button_resp = ui.add(
		//	egui::Button::new(
		//		egui::RichText::new(format!("书卷（{}）", version_display_name(&self.current_version)))
		//		.color(colors.text_color))
		//	.fill(colors.menu_button_bg)
		//);
		let button_resp = ui.scope(|ui| {
			ui.set_max_width(140.0);
			ui.add(
			egui::Button::new(
				egui::RichText::new(format!("书卷（{}）", version_display_name(&self.current_version)))
				.color(colors.text_color))
				.truncate()
				.fill(colors.menu_button_bg)
			)
		}).inner;

		// 切换菜单显示状态
		if button_resp.clicked() {
			self.show_version_menu = !self.show_version_menu;
		}

		// 如果菜单打开，绘制弹出层
		if self.show_version_menu {
			let mut menu_closed = false;

			egui::Area::new("show_version_menu".into())
				.order(egui::Order::Foreground)
				.current_pos(button_resp.rect.left_bottom())
				.show(ui.ctx(), |ui| {
					let popup_frame = egui::Frame {
						fill: colors.menu_button_bg,
						stroke: egui::Stroke::new(2.0, colors.menu_stroke),
						rounding: egui::Rounding::same(4.0),
						inner_margin: egui::Margin::same(4.0),
						..Default::default()
					};

					let item_height = 26.0;
					let rounding = egui::Rounding::same(4.0);

					popup_frame.show(ui, |ui| {
						ui.set_min_width(100.0);
						ui.set_max_width(100.0);

						for ver in self.versions.clone() {
							let size = egui::Vec2::new(ui.available_width(), item_height);
							let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());

							let bg = if resp.clicked() {
								colors.menu_button_active
							} else if resp.hovered() {
								colors.menu_button_hover
							} else {
								colors.item_bg
							};

							let text_color = colors.item_text;

							// 背景
							ui.painter().rect_filled(rect, rounding, bg);

							// 文本
							let text = version_display_name(&ver);
							let text_pos = rect.left_center() + egui::Vec2::new(6.0, 0.0);
							ui.painter().text(
								text_pos,
								egui::Align2::LEFT_CENTER,
								text,
								FontId::proportional(14.0),
								text_color,
							);

							if resp.clicked() {
								self.on_version_changed(ver);
								menu_closed = true;
								return;
							}
						}
					});
				});

			// 点击外部关闭
			let pointer_pos = ui.ctx().input(|i| i.pointer.hover_pos());
			let click_outside = ui.ctx().input(|i| i.pointer.any_click())
				&& !button_resp.rect.contains(pointer_pos.unwrap_or_default());

			if click_outside || menu_closed {
				self.show_version_menu = false;
			}
		}
	}
	pub fn change_version_button(
		&mut self,
		ui: &mut egui::Ui,
		colors: &ThemeColors,
	) {
		// 按钮
		let button_resp = ui.add(
			egui::Button::new(
				egui::RichText::new(format!("📖 {}", version_display_name(&self.current_version)))
				.color(colors.text_color))
			.fill(colors.menu_button_bg)
		);

		// 切换菜单显示状态
		if button_resp.clicked() {
			self.change_version_menu = !self.change_version_menu;
		}

		// 如果菜单打开，绘制弹出层
		if self.change_version_menu {
			let mut menu_closed = false;

			egui::Area::new("change_version_menu".into())
				.order(egui::Order::Foreground)
				.current_pos(button_resp.rect.left_bottom())
				.show(ui.ctx(), |ui| {
					let popup_frame = egui::Frame {
						fill: colors.menu_button_bg,
						stroke: egui::Stroke::new(2.0, colors.menu_stroke),
						rounding: egui::Rounding::same(4.0),
						inner_margin: egui::Margin::same(4.0),
						..Default::default()
					};

					let item_height = 26.0;
					let rounding = egui::Rounding::same(4.0);

					popup_frame.show(ui, |ui| {
						ui.set_min_width(80.0);
						ui.set_max_width(80.0);

						for ver in self.versions.clone() {
							let size = egui::Vec2::new(ui.available_width(), item_height);
							let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());

							let bg = if resp.clicked() {
								colors.menu_button_active
							} else if resp.hovered() {
								colors.menu_button_hover
							} else {
								colors.item_bg
							};

							let text_color = colors.item_text;

							// 背景
							ui.painter().rect_filled(rect, rounding, bg);

							// 文本
							let text = version_display_name(&ver);
							let text_pos = rect.left_center() + egui::Vec2::new(6.0, 0.0);
							ui.painter().text(
								text_pos,
								egui::Align2::LEFT_CENTER,
								text,
								FontId::proportional(14.0),
								text_color,
							);

							if resp.clicked() {
								self.on_version_changed(ver);
								menu_closed = true;
								return;
							}
						}
					});
				});

			// 点击外部关闭
			let pointer_pos = ui.ctx().input(|i| i.pointer.hover_pos());
			let click_outside = ui.ctx().input(|i| i.pointer.any_click())
				&& !button_resp.rect.contains(pointer_pos.unwrap_or_default());

			if click_outside || menu_closed {
				self.change_version_menu = false;
			}
		}
	}
}
/// 设置按钮
impl BibleApp {
	pub fn settings_menu_button(&mut self, ui: &mut egui::Ui, colors: &ThemeColors) {
		let button_resp = ui.add(egui::Button::new(egui::RichText::new(" ⚙ ").color(colors.text_color)).fill(colors.menu_button_bg));

		if button_resp.clicked() {
			self.show_settings_menu = !self.show_settings_menu;
		}

		if self.show_settings_menu {
			let area_id = ui.id().with("settings_menu");

			egui::Area::new(area_id)
				.order(egui::Order::Foreground)
				.current_pos(button_resp.rect.left_bottom())
				.show(ui.ctx(), |ui| {
					let frame = egui::Frame {
						fill: colors.menu_button_bg,
						stroke: egui::Stroke::new(2.0, colors.menu_stroke),
						rounding: egui::Rounding::same(4.0),
						inner_margin: egui::Margin::same(4.0),
						..Default::default()
					};

					let popup_width = 71.0;
					frame.show(ui, |ui| {
						ui.set_min_width(popup_width);
						ui.set_max_width(popup_width);

						//let dark_theme_btn = draw_hover_button(
						//	ui,
						//	"暗色主题",
						//	egui::Vec2::new(70.0, 24.0),
						//	colors
						//);
						//let light_theme_btn = draw_hover_button(
						//	ui,
						//	"浅色主题",
						//	egui::Vec2::new(70.0, 24.0),
						//	colors
						//);

						let toggle_theme_btn = draw_hover_button(
							ui,
							match self.theme {
								Theme::Dark => "浅色主题",
								Theme::Light => "暗色主题",
							},
							egui::Vec2::new(70.0, 24.0),
							colors,
						);

						let clean_highlight_btn = draw_hover_button(
							ui,
							if self.show_highlight { "取消高亮" } else { "显示高亮" },
							egui::Vec2::new(70.0, 24.0),
							colors
						);

						let notes_list_btn = draw_hover_button(
							ui,
							"笔记列表",
							egui::Vec2::new(70.0, 24.0),
							colors
						);

						let add_note_btn = draw_hover_button(
							ui,
							"添加笔记",
							egui::Vec2::new(70.0, 24.0),
							colors
						);

						let toggle_editable_btn = draw_hover_button(
							ui,
							if self.editable_mode { "只读模式" } else { "编辑模式" },
							egui::Vec2::new(70.0, 24.0),
							colors,
						);


						//if dark_theme_btn.clicked()
						//{
						//	self.theme = Theme::Dark;
						//	self.show_settings_menu = false;
						//}

						//if light_theme_btn.clicked() {
						//	self.theme = Theme::Light;
						//	self.show_settings_menu = false;
						//}

						if toggle_theme_btn.clicked() {
							self.theme = match self.theme {
								Theme::Dark => Theme::Light,
								Theme::Light => Theme::Dark,
							};
							self.show_settings_menu = false;
						}

						if clean_highlight_btn.clicked() {
							self.show_highlight = !self.show_highlight;
							self.show_settings_menu = false;
							self.editable_mode = false;
						}

						if notes_list_btn.clicked(){
							self.notes_cache = self.load_notes("notes", "all");
							self.show_notes_list_window = true;
							self.show_settings_menu = false;
						}

						if add_note_btn.clicked(){
							self.open_noteapp_window(None);
							self.show_settings_menu = false;
						}

						if toggle_editable_btn.clicked(){
							self.editable_mode = !self.editable_mode
						}
					});
				});

			let pointer_pos = ui.ctx().input(|i| i.pointer.hover_pos());
			let click_outside =
				ui.ctx().input(|i| i.pointer.any_click())
				&& !button_resp.rect.contains(pointer_pos.unwrap_or_default());

			if click_outside {
				self.show_settings_menu = false;
			}
		}
	}
}

///右侧顶栏
impl BibleApp {
	fn ui_top_toolbar(&mut self, ui: &mut egui::Ui, colors: &ThemeColors) {
		ui.horizontal(|ui| {

			//译本切换按钮
			self.change_version_button(ui, &colors);

			// 书名标签
			let book_name = self.current_book
				.and_then(|num| self.books.iter().find(|(n, _)| *n == num))
				.map(|(_, name)| name.clone())
				.unwrap_or_default();
			self.current_book_name = Some(book_name.clone());
			ui.add(egui::Button::new(book_name)
				.min_size([50.0, 20.0].into())
				.fill(colors.menu_button_bg)
			);

			// 章节标签
			let chapter_name = chapter_display_name(
				&self.current_chapter.clone().unwrap_or_default()
			);
			ui.add(egui::Button::new(chapter_name)
				.fill(colors.menu_button_bg)
			);

			// 搜索框
			ui.add_space(10.0);
			self.ui_search_box(ui, colors);

			ui.add_space(ui.available_width() - 120.0);

			// 复制整章
			let copy_btn = ui.add(
				egui::Button::new(
					egui::RichText::new("复制整章")
					.color(colors.text_color)
				)
				.fill(colors.menu_button_bg) 
			);
			if copy_btn.clicked() {
				ui.ctx().copy_text(self.content.clone());
			}

			// 主题按钮
			self.settings_menu_button(ui, &colors);
		});
	}
}
///搜索框
impl BibleApp {
	fn ui_search_box(&mut self, ui: &mut egui::Ui, colors: &ThemeColors) {
		egui::Frame::none()
			.fill(colors.menu_button_bg)        // 背景色
			.rounding(egui::Rounding::same(4.0))
			.show(ui, |ui| {
				let search = ui.add(
					egui::TextEdit::singleline(&mut self.search_query)
					.hint_text(
						egui::RichText::new("搜索经文")
						.color(colors.comment_text_color)
						.size(14.0)
					)
					.frame(false)
					.desired_width(200.0)
					.min_size(egui::vec2(80.0, 14.0))
				);

				if search.clicked() || search.gained_focus() || search.has_focus(){
					self.active_search_type = "bible".to_string();
				}
				if search.clicked_elsewhere() {
					self.active_search_type = "".to_string();
				}

				let search_focused = search.has_focus();

				// 关键词改变  隐藏旧结果
				if self.search_query != self.last_search_query {
					self.show_search_window = false;
					self.search_results.clear();
					//self.highlight_query = None;
					self.show_highlight = false; 
				}

				// 光标聚焦且关键词没变  显示上次结果
				if search_focused && !self.search_query.is_empty() && self.search_query == self.last_search_query {
					self.show_search_window = true;
				}

				// 响应回车搜索
				if ui.input(|i| i.key_pressed(egui::Key::Enter)) && self.active_search_type == "bible"
					&& !self.search_query.is_empty() {
					//self.perform_search();
					if let Err(e) = self.perform_search() {
						eprintln!("搜索出错: {:?}", e);
					}
					self.show_search_window = true;
					self.last_search_query = self.search_query.clone();
				}

				// 搜索按钮
				let search_btn = ui.add(
					egui::Button::new(
						egui::RichText::new("搜索")
						.color(colors.text_color)
						.size(16.0)
					)
					.fill(colors.menu_button_bg)
				);

				if search_btn.clicked() {
					//self.perform_search();
					if let Err(e) = self.perform_search() {
						eprintln!("搜索出错: {:?}", e);
					}
					self.show_search_window = true;
				}
			});
	}
}

///搜索结果栏目
impl BibleApp {
	fn ui_search_window(&mut self, ctx: &egui::Context, colors: &ThemeColors,) {
		if !self.show_search_window || self.search_results.is_empty() {
			return;
		}

		let mut chosen: Option<(i32, String)> = None;
		let mut close = false;

		let result_count = self.search_results.len();
		let title_text = format!("{}条搜索结果", result_count);
		egui::Window::new(egui::RichText::new(&title_text).size(14.0))
			.title_bar(false)
			.resizable(true)
			.collapsible(false)
			.open(&mut self.show_search_window)
			.default_size([400.0, 600.0])
			.max_width(400.0)
			.default_pos([300.0, 50.0])
			.show(ctx, |ui| {
				//自定义顶栏
				ui.horizontal(|ui| {
					// 左侧：清除按钮
					if ui.add(
						egui::Button::new(egui::RichText::new("清除").size(14.0)).frame(true) 
					).clicked() {
						self.search_results.clear();
						self.search_query.clear();
						self.highlight_query = None;
						self.show_highlight = false; 
					}

					// 中间：标题文字
					ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
						ui.label(egui::RichText::new("搜索结果").size(14.0).strong());
					});

					// 右侧：关闭按钮
					if ui.add(
						egui::Button::new(egui::RichText::new("❌").size(14.0)).frame(true)
					).clicked() {
						close = true;
					}
				});

				ui.separator();

				egui::ScrollArea::vertical().show(ui, |ui| {
					for (book, book_name, chap_num, snippet) in &self.search_results {
						let mut job = LayoutJob::default();
						let body_font_id = egui::FontId::proportional(14.0);

						// 红色部分：版本 + 书卷名 + 章节
						job.append(
							&format!("{} {} {}: ", version_display_name(&self.current_version), book_name, chap_num),
							0.0,
							TextFormat {
								font_id: body_font_id.clone(),
								color: egui::Color32::RED,
								..Default::default()
							},
						);

						// 追加正文高亮
						if let Some(query) = self.highlight_query.as_deref() {
							highlight_search_terms(&snippet, query, colors, &mut job, &body_font_id);
						}

						// 用 Button 显示
						if ui.add(egui::Button::new(job)).clicked() {
							chosen = Some((*book, chap_num.to_string()));
							close = true;
						}
					}
				});
			});

		if let Some((book, chap)) = chosen {
			////self.on_chapter_selected(book, chap);
			let ch_num = chap.parse::<i32>().unwrap_or(1);
			if let Some(content) = self.text_cache.get(&(book, ch_num)).cloned() {
				self.record_jump();
				self.current_book = Some(book);
				self.current_chapter = Some(ch_num.to_string());
				self.content = content;
				self.show_highlight = true; 
			} else {
				self.on_chapter_selected(book, ch_num.to_string());
			}
		}

		if close {
			self.show_search_window = false;
		}
	}
}

///文本显示区
impl BibleApp {
	fn ui_content_panel(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
		egui::ScrollArea::vertical().show(ui, |ui| {

			let theme_colors = apply_theme(ctx, &self.theme);

			let mut text_response = if self.show_highlight {
				readonly_content_text_highlighted(
					ui,
					&self.content,
					&theme_colors,
					self.highlight_query.as_deref(),
				)
			} else {
				if self.editable_mode{
					self.display_text_with_notes(ui, &theme_colors, DisplayMode::Editable)
				} else {
					self.display_text_with_notes(ui, &theme_colors, DisplayMode::ReadOnly)
				}
			};

			self.show_right_click_menu(&mut text_response);

		});
	}
}

///右键菜单
impl BibleApp {
	fn show_right_click_menu(&mut self, response: &mut egui::Response) {
		response.context_menu(|ui| {
			if ui.button("➕ 添加笔记").clicked() { 
				self.open_noteapp_window(None);
				ui.close_menu();
			}

			if ui.button("💬 显示笔记").clicked() { 
				self.show_notes = true;
				self.show_highlight = false; 
				ui.close_menu();
			}

			if ui.button("🍄 隐藏笔记").clicked() { 
				self.show_notes = false;
				ui.close_menu();
			}
		});
	}
}

///打开笔记编辑窗口
impl BibleApp {
	fn open_noteapp_window(&self, note_opt: Option<&Notedb>) {
		let note = if let Some(note) = note_opt {
			note.clone() // 编辑已有笔记
		} else {
			// 新建笔记
			Notedb {
				id: Uuid::new_v4().to_string(),
				created_at: Some(Utc::now().format("%Y-%m-%d").to_string()),
				book_num: self.current_book,
				book_name: self.current_book_name.clone(),
				chapter: self.current_chapter.clone(),
				verse_start: -1,
				char_offset: Some(0),
				version: Some(self.current_version.clone()),
				..Default::default()
			}
		};

		let note_json = match serde_json::to_string(&note) {
			Ok(json) => json,
			Err(e) => {
				eprintln!("序列化笔记失败: {e}");
				return;
			}
		};

		let exe = match std::env::current_exe() {
			Ok(exe) => exe,
			Err(e) => {
				eprintln!("无法获取当前可执行文件路径: {e}");
				return;
			}
		};

		if let Err(e) = std::process::Command::new(exe)
			.arg("--note-window")
				.arg("--note-json")
				.arg(note_json)
				.spawn()
		{
			eprintln!("无法启动笔记窗口: {e}");
		}
	}
}

///版本切换
impl BibleApp {
	fn on_version_changed(&mut self, ver: String) {
		self.record_jump();
		self.search_results.clear();
		self.show_search_window = false;
		self.last_search_query.clear();
		self.text_cache.clear();
		self.highlight_query = None;
		self.show_highlight = false; 
		self.editable_mode = false;

		let old_book = self.current_book;
		let old_chapter = self.current_chapter.clone();

		self.current_version = ver.clone();
		let db_path = self.bible_root.join(&self.current_version);
		self.books = load_books(&db_path);

		// 保持原书卷
		self.current_book = old_book
			.filter(|b| self.books.iter().any(|(n, _)| n == b))
			.or_else(|| self.books.first().map(|(n, _)| *n));

		// --- 打开数据库并持久化连接 ---
		match Connection::open(&db_path) {
			Ok(conn) => {
				self.conn = Some(conn);
			}
			Err(e) => {
				eprintln!("打开数据库失败: {:?}", e);
				self.conn = None;
			}
		}

		if let Some(book) = self.current_book {
			let mut chapters = load_chapters(&db_path, book);
			chapters.sort_by_key(|c| chapter_number(c));
			self.chapters = chapters;

			self.current_chapter = old_chapter
				.filter(|c| self.chapters.contains(c))
				.or_else(|| self.chapters.first().cloned());

			if let Some(ch_str) = self.current_chapter.clone() {
				let ch_num = ch_str.parse().unwrap_or(1);
				self.content = load_chapter_content(&db_path, book, ch_num);
			}
		} else {
			self.chapters.clear();
			self.current_chapter = None;
			self.content.clear();
		}
	}

	fn on_book_selected(&mut self, book_num: i32) {
		self.record_jump();
		self.current_book = Some(book_num);
		let db_path = self.bible_root.join(&self.current_version);
		let mut chapters = load_chapters(&db_path, book_num);
		chapters.sort_by_key(|c| chapter_number(c));
		self.chapters = chapters;
		// 自动选择第一章
		if let Some(first_chapter) = self.chapters.first().cloned() {
			self.current_chapter = Some(first_chapter.clone());

			let ch_num = first_chapter.parse().unwrap_or(1);
			self.content = load_chapter_content(&db_path, book_num, ch_num);
		} else {
			// 该书无章（几乎不会发生）
			self.current_chapter = None;
			self.content.clear();
		}
	}

	fn on_chapter_selected(&mut self, book_num: i32, ch: String) {
		self.record_jump();
		self.current_book = Some(book_num.clone());
		self.current_chapter = Some(ch.clone());
		let ch_num = ch.parse().unwrap_or(1);
		self.content = load_chapter_content(
			&self.bible_root.join(&self.current_version),
			book_num,
			ch_num,
		);
	}
}

///转跳
impl BibleApp {
	fn record_jump(&mut self) {
		if let (Some(book), Some(chap)) = (self.current_book, &self.current_chapter) {
			let current_state = (
				self.current_version.clone(),
				book,
				chap.clone(),
			);

			// 避免连续重复状态
			if self.jump_back_stack.last() != Some(&current_state) {
				self.jump_back_stack.push(current_state.clone());
			}

			// 新操作清空 forward 栈
			self.jump_forward_stack.clear();
		}
	}
	fn jump_back(&mut self) {
		if let Some(prev) = self.jump_back_stack.pop() {
			// 1. 当前状态推入 forward_stack
			if let (Some(book), Some(chap)) = (self.current_book, &self.current_chapter) {
				let current_state = (
					self.current_version.clone(),
					book,
					chap.clone(),
				);
				self.jump_forward_stack.push(current_state);
			}

			// 2. 跳转到 prev 所指内容
			self.apply_state(prev);
		}
	}
	fn jump_forward(&mut self) {
		if let Some(next) = self.jump_forward_stack.pop() {
			// 1. 当前状态推入 back_stack
			if let (Some(book), Some(chap)) = (self.current_book, &self.current_chapter) {
				let current_state = (
					self.current_version.clone(),
					book,
					chap.clone(),
				);
				self.jump_back_stack.push(current_state);
			}

			// 2. 跳转到 next
			self.apply_state(next);
		}
	}
	fn apply_state(&mut self, state: (String, i32, String)) {
		let (ver, book, chap) = state;

		self.current_version = ver.clone();
		self.books = load_books(&self.bible_root.join(&self.current_version));

		self.current_book = Some(book);
		self.chapters = load_chapters(&self.bible_root.join(&self.current_version), book);

		self.current_chapter = Some(chap.clone());
		let ch_num = chap.parse().unwrap_or(1);
		self.content = load_chapter_content(
			&self.bible_root.join(&self.current_version),
			book,
			ch_num,
		);
	}
	fn check_jump_shortcuts(&mut self, ctx: &egui::Context) {
		// 遍历当前帧所有键事件
		for event in &ctx.input(|i| i.events.clone()) {
			if let egui::Event::Key { key, pressed, modifiers, .. } = event {
				if *pressed && modifiers.ctrl {
					match key {
						egui::Key::O => self.jump_back(),
						egui::Key::I => self.jump_forward(),
						_ => {}
					}
				}
			}
		}
	}
}

impl eframe::App for BibleApp {
	fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
		let colors = apply_theme(ctx, &self.theme);
		// 左侧 UI
		self.ui_left_books_panel(ctx, &colors);
		self.ui_left_chapters_panel(ctx, &colors);

		// 中央 UI
		egui::CentralPanel::default().show(ctx, |ui| {
			// 顶部工具栏
			self.ui_top_toolbar(ui, &colors);
			ui.separator();
			// 搜索窗口
			self.ui_search_window(ctx, &colors);

			// 正文内容
			self.ui_content_panel(ctx, ui);

			// 空白处笔记弹窗
			let empty_rect = ui.available_rect_before_wrap();
			let mut empty_resp = ui.allocate_rect(empty_rect, egui::Sense::click());
			self.show_right_click_menu(&mut empty_resp);

		});

		//show_note_window(ctx, &colors, &mut self.open_note);
		self.show_note_window(ctx, &colors);

		self.show_notes_list_window(ctx, &colors);

		// 检测快捷键
		self.check_jump_shortcuts(ctx);
	}
}

fn main() -> eframe::Result<()> {
	let args: Vec<String> = std::env::args().collect();
	if args.len() > 1 && args[1] == "--note-window" {
		let mut note_json: Option<String> = None;
		let mut i = 1;
		while i < args.len() {
			match args[i].as_str() {
				"--note-json" => {
					if let Some(v) = args.get(i + 1) {
						note_json = Some(v.clone());
					}
					i += 1;
				}
				_ => {}
			}
			i += 1;
		}

		// 反序列化 JSON 为 Notedb
		let note_data: Notedb = if let Some(nj) = note_json {
			serde_json::from_str(&nj).unwrap()
		} else {
			Notedb {
				id: Uuid::new_v4().to_string(),
				created_at: Some(Utc::now().format("%Y-%m-%d").to_string()),
				..Default::default()
			}
		};

		let options = eframe::NativeOptions {
			renderer: eframe::Renderer::Wgpu,
			viewport: egui::ViewportBuilder::default()
				.with_inner_size([600.0, 600.0])
				.with_title("撰写笔记"),
				..Default::default()
		};
		eframe::run_native(
			"撰写笔记",
			options,
			Box::new(move |cc| {
				configure_chinese_font(&cc.egui_ctx);
				Ok(Box::new(NoteApp { 
					note: note_data,
				}))
			}),
		)

	} else {
		let options = eframe::NativeOptions {
			renderer: eframe::Renderer::Wgpu,
			viewport: egui::ViewportBuilder::default()
				.with_inner_size([1200.0, 800.0])
				.with_title("圣经阅读器"),
				..Default::default()
		};

		eframe::run_native(
			"圣经阅读器",
			options,
			Box::new(|cc| Ok(Box::new(BibleApp::new(cc)))),
		)
	}
}

