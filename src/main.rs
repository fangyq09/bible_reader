#![windows_subsystem = "windows"]

mod theme;
mod utils;
mod notes;
mod note_app;
use std::fs;
use rusqlite::Connection;
use eframe::egui;
use egui::{FontDefinitions, FontFamily, FontId, TextFormat};
use egui::text::LayoutJob;
use std::path::PathBuf;
use std::collections::HashMap;
use serde_json;
use uuid::Uuid;
use chrono::Utc;
use crate::theme::{Theme, ThemeColors, apply_theme, get_theme_colors};
use crate::utils::{
	load_books,
	load_chapters,
	load_chapter_content,
	chapter_number,
	chapter_display_name,
	version_display_name,
	sort_versions_chinese_first,
	book_number_to_abbr,
	highlight_search_terms,
	draw_hover_button,
};
use crate::notes::{Notedb};
use crate::note_app::NoteApp;

#[derive(PartialEq, Clone)]
struct ContentState {
    version: String,
    book: Option<i32>,
    chapter: Option<String>,
    query: Option<String>,
    theme: Theme,
    show_highlight: bool,
}

#[derive(Clone, PartialEq)]
struct NavState {
    version: String,
    book: Option<i32>,
    chapter: Option<String>,
}

#[derive(Debug, Clone)]
struct ParallelVerse {
    version: String,
    text: String,
}

// 应用状态
struct BibleApp {
	theme: Theme,
	ui_initialized: bool,
	bible_root: PathBuf,
	versions: Vec<String>,
	pub current_version: String,
	books: Vec<(i32, String)>,
	chapters: Vec<String>,
	pub current_book: Option<i32>,
	pub	current_chapter: Option<String>,
	content: String,
	pub current_book_name: Option<String>,
	chapter_panel_title: String,
	search_query: String,   // 搜索框内容
	search_results: Vec<(i32, String, i32, String)>,
	text_cache: HashMap<(i32, i32), String>,
	conn: Option<Connection>,  // 持久化连接
	conn_version: Option<String>,
	show_search_window: bool, // 控制搜索结果窗口显示
	last_search_query: String,
	highlight_query: Option<String>,
	jump_back_stack: Vec<NavState>,
	jump_forward_stack: Vec<NavState>,
	number_marks: HashMap<u8, NavState>,
	show_version_menu: bool,
	change_version_menu: bool,
	show_settings_menu: bool,
	show_highlight: bool,
	pub show_notes: bool,
	pub show_import_export_window: bool,
	pub last_appended_notes_state: Option<NavState>,
	pub appended_notes_current: Vec<Notedb>,
	pub show_notes_list_window: bool,
	pub notes_cache: Vec<Notedb>,
	pub note_window_open: bool,
	pub current_note: Option<Notedb>,
	pub notes_search_keyword: String,
	pub active_search_type: String,
	editable_mode: bool,
	content_layout: Option<egui::text::LayoutJob>, 
	last_state: Option<ContentState>,
	selected_verse_num: Option<i32>,
	pub parallel_verses: Vec<ParallelVerse>,
	pub show_parallel_window: bool,
	pub parallel_window_pos: Option<egui::Pos2>,
}
//中文字体
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
}
//初始化
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
			("和修本.sqlite3", include_bytes!("../assets/sqlite/rcuvss.sqlite3")),
			("当代译本.sqlite3", include_bytes!("../assets/sqlite/ccb.sqlite3")),
			("niv2011.sqlite3", include_bytes!("../assets/sqlite/niv2011.sqlite3")),
			("sg21.sqlite3", include_bytes!("../assets/sqlite/sg21.sqlite3")),
		];

		for (filename, content) in built_in_files {
			let target = sqlite_path.join(filename);
			if !target.exists() {
				fs::write(&target, content).expect("写入内置译本失败");
			}
		}

		// ---------- 加载中文字体 ----------
		configure_chinese_font(&cc.egui_ctx);

		// ---------- 应用初始主题 (关键) ----------
		//apply_theme(&cc.egui_ctx, &Theme::Light);

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
				ui_initialized: true,
				bible_root,
				versions,
				current_version: String::new(),
				books: vec![],
				chapters: vec![],
				current_book: None,
				current_chapter: None,
				content: String::new(),
				current_book_name: Some("创世纪".to_string()),
				chapter_panel_title: "章节（创）".to_string(),
				search_query: String::new(),
				search_results: vec![],
				text_cache: HashMap::new(),
				conn: None, 
				conn_version: None,
				show_search_window: false,
				last_search_query: String::new(),
				highlight_query: None,
				jump_back_stack: Vec::new(),     
				jump_forward_stack: Vec::new(),  
				number_marks: HashMap::new(),
				show_notes: false,
				show_import_export_window: false,
				last_appended_notes_state: None, 
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
				content_layout: None,
				last_state: None,
				selected_verse_num: None,
				parallel_verses: Vec::new(),
				show_parallel_window: false,
				parallel_window_pos: None,
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
				app.open_current_db();
				app.on_version_changed(version_to_load);
			}
			app
		}
	}

// 连接数据库
impl BibleApp {
	fn open_current_db(&mut self) {
		if self.current_version.is_empty() {
			return;
		}

		// 如果已经为当前版本打开了连接，就直接返回
		if self.conn.is_some() && self.conn_version.as_deref() == Some(&self.current_version) {
			return;
		}

		let db_path = self.bible_root.join(&self.current_version);

		//let conn = Connection::open(&db_path)
		//	.unwrap_or_else(|e| {
		//		panic!("打开数据库失败 {:?}: {}", db_path, e);
		//	});

		//self.conn = Some(conn);
		//self.conn_version = Some(self.current_version.clone());
		match Connection::open(&db_path) {
			Ok(conn) => {
				self.conn = Some(conn);
				self.conn_version = Some(self.current_version.clone());
			}
			Err(e) => {
				eprintln!("打开数据库失败 {:?}: {}", db_path, e);

				// 明确进入“无数据库”状态
				self.conn = None;
				self.conn_version = None;

				// 可选：清空依赖 DB 的数据
				self.books.clear();
				self.chapters.clear();
				self.content.clear();
			}
		}
	}
}

// 搜索经文
impl BibleApp {
fn perform_search(&mut self) -> rusqlite::Result<()> {
    self.search_results.clear();
    self.text_cache.clear();
		self.highlight_query = None;

		self.open_current_db();
		let conn = self.conn.as_ref().unwrap();
    //let conn = match &self.conn {
    //    Some(c) => c,
    //    None => {
    //        eprintln!("原始数据库尚未初始化！");
    //        return Ok(());
    //    }
    //};

    let query = self.search_query.trim();
    if query.is_empty() { return Ok(()); }


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

//左侧书卷栏目
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
//impl BibleApp {
//	fn ui_left_books_panel(&mut self, ctx: &egui::Context, colors: &ThemeColors) {
//		let mut selected_book: Option<i32> = None;
//
//		egui::SidePanel::left("books_panel")
//			.resizable(true)
//			.default_width(150.0)
//			.show(ctx, |ui| {
//				self.version_menu_button(ui, &colors);
//				ui.separator();
//
//				let row_height = ui.text_style_height(&egui::TextStyle::Body) 
//					+ ui.spacing().item_spacing.y;
//
//				egui::ScrollArea::vertical()
//					.auto_shrink([false; 2])
//					.show_rows(ui, row_height, self.books.len(), |ui, row_range| {
//						for i in row_range {
//							let (num, name) = &self.books[i];
//							let is_selected = Some(*num) == self.current_book;
//
//							let bg = if is_selected {
//								colors.book_selected_bg
//							} else {
//								colors.book_unselected_bg
//							};
//							let txt_color = if is_selected {
//								colors.selected_text_color
//							} else {
//								colors.text_color
//							};
//
//							let txt = egui::RichText::new(name).color(txt_color);
//
//							if ui.add(egui::Button::new(txt).fill(bg)).clicked() {
//								selected_book = Some(*num);
//							}
//						}
//					});
//			});
//
//		if let Some(b) = selected_book {
//			self.on_book_selected(b);
//		}
//	}
//}

//中间章节栏目
impl BibleApp {
	fn ui_left_chapters_panel(&mut self, ctx: &egui::Context, colors: &ThemeColors) {
		let mut chosen: Option<String> = None;
		let book_num = self.current_book;
		//let book_abbr = &book_num.map(book_number_to_abbr).unwrap_or("未选择");  

		egui::SidePanel::left("chapters_panel")
			.resizable(true)
			.default_width(120.0)
			.show(ctx, |ui| {
				if let Some(_book) = book_num {
					ui.label(&self.chapter_panel_title);
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

							let txt = egui::RichText::new(chapter_display_name(chap))
								.color(txt_color);

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

//译本切换按钮
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
						corner_radius: egui::CornerRadius::same(4),
						inner_margin: egui::Margin::same(4),
						..Default::default()
					};

					let item_height = 26.0;
					let rounding = egui::CornerRadius::same(4);

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
						corner_radius:egui::CornerRadius::same(4),
						inner_margin: egui::Margin::same(4),
						..Default::default()
					};

					let item_height = 26.0;
					let rounding = egui::CornerRadius::same(4);

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
// 设置按钮
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
						corner_radius: egui::CornerRadius::same(4),
						inner_margin: egui::Margin::same(4),
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


						let import_export_btn = draw_hover_button(
							ui,
							"导入导出",
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
							apply_theme(ui.ctx(), &self.theme);
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

						if import_export_btn.clicked(){
							self.show_import_export_window = true;
							self.show_settings_menu = false;
							self.show_parallel_window = false;
						}

						if toggle_editable_btn.clicked(){
							self.editable_mode = !self.editable_mode;
							self.show_notes = false;
							self.show_parallel_window = false;
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

// 笔记导出窗口
impl BibleApp {
	pub fn import_export_window(&mut self, ctx: &egui::Context, colors: &ThemeColors) {
		if !self.show_import_export_window {
			return;
		}

		let screen_rect = ctx.available_rect();


		let area_res = egui::Area::new(egui::Id::new("import_export_menu"))
			.order(egui::Order::Foreground)
			.fixed_pos(screen_rect.center())
			.pivot(egui::Align2::CENTER_CENTER)
			.show(ctx, |ui| {
				egui::Frame::default()
					.fill(colors.menu_button_bg)
					.stroke(egui::Stroke::new(1.0, colors.menu_stroke))
					.corner_radius(egui::CornerRadius::same(6))
					.shadow(egui::Shadow {
						offset: [0, 4],
						blur: 12,
						spread: 0,
						color: egui::Color32::from_black_alpha(40),
					})
				.inner_margin(egui::Margin::symmetric(6, 8)) // 增加一点边距
					.show(ui, |ui| {
						ui.set_width(100.0);
						// 设置间距，让菜单项不那么拥挤
						ui.spacing_mut().item_spacing.y = 4.0;

						ui.vertical_centered_justified(|ui| {
							// 使用 selectable_label 来获得 hover 效果
							if ui.add(egui::Button::selectable(false, "导入译本")).clicked() {
								self.import_bible_logic();
								self.show_import_export_window = false;
							}

							if ui.add(egui::Button::selectable(false, "导入笔记")).clicked() {
								self.import_notes_logic();
								self.show_import_export_window = false;
							}

							if ui.add(egui::Button::selectable(false, "导出笔记")).clicked() {
								self.export_notes_logic();
								self.show_import_export_window = false;
							}

							if ui.add(egui::Button::selectable(false, "笔记同步")).clicked() {
								self.show_import_export_window = false;
							}
						});
					});
			});

		// 点击外部关闭逻辑
		if ctx.input(|i| i.pointer.any_pressed()) {
			// response.interact_rect 是 Area 实际占据的物理区域
			if let Some(pos) = ctx.pointer_interact_pos() {
				if !area_res.response.rect.contains(pos) {
					self.show_import_export_window = false;
				}
			}
		}

		if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
			self.show_import_export_window = false;
		}
	}
}

//右侧顶栏
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
//搜索框
impl BibleApp {
	fn ui_search_box(&mut self, ui: &mut egui::Ui, colors: &ThemeColors) {
		egui::Frame::new()
			.fill(colors.menu_button_bg)        // 背景色
			.corner_radius(egui::CornerRadius::same(4))
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

//搜索结果栏目
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

//文本显示区
impl BibleApp {
	fn ui_content_panel(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {

		let current_state = ContentState {
			version: self.current_version.clone(),
			book: self.current_book,
			chapter: self.current_chapter.clone(),
			query: self.highlight_query.clone(),
			theme: self.theme,
			show_highlight: self.show_highlight,
		};
		if self.content_layout.is_none() || self.last_state.as_ref() != Some(&current_state) {
			let theme_colors = get_theme_colors(ctx, &self.theme);
			self.content_layout = Some(self.prepare_content_layout(ui, &theme_colors));
			self.last_state = Some(current_state);
		}

		let body_font_id = ui.style().text_styles[&egui::TextStyle::Body].clone();
		egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
			if self.editable_mode {
				let text_edit = egui::TextEdit::multiline(&mut self.content)
					.desired_width(ui.available_width() - 12.0)
					.frame(false)
					.interactive(true) 
					.clip_text(false)
				.font(body_font_id);
				ui.add(text_edit);
			} else {
				let content_rect = ui.available_rect_before_wrap();
				let mut bg_response = ui.interact(content_rect, ui.id().with("bg_sense"), egui::Sense::click());
				self.show_right_click_menu(&mut bg_response, false);
				if let Some(layout) = &self.content_layout {
					ui.set_width(ui.available_width() - 12.0);
					let mut text_response = ui.add(
						egui::Label::new(layout.clone())
						.sense(egui::Sense::click())
						.selectable(true),
					);
					// --- 右键点击位置解析节号 ---
					if text_response.secondary_clicked() {
						self.show_parallel_window = false;
						if let Some(pointer_pos) = ctx.pointer_interact_pos() {
							let relative_pos = pointer_pos - text_response.rect.min;
							let mut job = layout.clone();
							job.wrap.max_width = text_response.rect.width();
							let galley = ctx.fonts_mut(|fonts| fonts.layout_job(job));
							let cursor = galley.cursor_from_pos(relative_pos);
							let char_idx = cursor.index;
							self.selected_verse_num = parse_verse_num_at_index(&self.content, char_idx);
							self.parallel_window_pos = Some(pointer_pos);
						}
					}
					self.show_right_click_menu(&mut text_response, true);
				}
			}
			if self.show_notes {
				self.get_appended_notes();
				self.show_appended_notes(ui);
			}
		});
	}

	fn prepare_content_layout(&self, ui: &egui::Ui, colors: &ThemeColors) -> egui::text::LayoutJob {
		let mut job = egui::text::LayoutJob::default();
		let body_font_id = ui.style().text_styles[&egui::TextStyle::Body].clone();

		if self.show_highlight {
			if let Some(query) = self.highlight_query.as_deref() {
				if !query.is_empty() {
					highlight_search_terms(&self.content, query, colors, &mut job, &body_font_id);
					return job; 
				}
			}
		}

		job.append(
			&self.content,
			0.0,
			egui::TextFormat {
				font_id: body_font_id,
				color: colors.text_color,
				..Default::default()
			},
		);
		job
	}
}


fn parse_verse_num_at_index(content: &str, char_idx: usize) -> Option<i32> {
    if content.is_empty() {
        return None;
    }

    let chars: Vec<char> = content.chars().collect();
    let len = chars.len();
    let mut start = char_idx;
    let mut end = char_idx;

    // 如果光标在数字内部，向左找到数字开头
    while start > 0 && chars[start - 1].is_ascii_digit() {
        start -= 1;
    }

    // 向右找到数字末尾
    while end < len && chars[end].is_ascii_digit() {
        end += 1;
    }

    // 提取数字字符串
    if start < end {
        let digits: String = chars[start..end].iter().collect();
        if let Ok(verse) = digits.parse::<i32>() {
            if verse <= 176 {
                return Some(verse);
            }
        }
    }

    // 如果光标不在数字内部，则向左扫描最多2000个字符
    let mut i = char_idx.min(len);
    let scan_limit = i.saturating_sub(2000);
    let mut digits = String::new();
    let mut found_digit = false;

    while i > scan_limit {
        i -= 1;
        let c = chars[i];

        if c.is_ascii_digit() {
            digits.insert(0, c);
            found_digit = true;
        } else if found_digit {
            if c != '\n' && !c.is_whitespace() {
                digits.clear();
                found_digit = false;
                continue;
            }

            if let Ok(verse) = digits.parse::<i32>() {
                if verse <= 176 {
                    return Some(verse);
                }
            }

            digits.clear();
            found_digit = false;
        }
    }

    // 如果扫描结束，最后检查一次
    if found_digit {
        if let Ok(verse) = digits.parse::<i32>() {
            if verse <= 176 {
                return Some(verse);
            }
        }
    }

    None
}

//右键菜单
	impl BibleApp {
		fn show_right_click_menu(&mut self, response: &mut egui::Response, show_parallel_option: bool) {
			response.context_menu(|ui| {
				if ui.button("➕ 添加笔记").clicked() { 
					self.open_noteapp_window(None);
					ui.close_kind(egui::UiKind::Menu)
				}

				if ui.button("💬 显示笔记").clicked() { 
					self.show_notes = true;
					self.show_highlight = false; 
					ui.close_kind(egui::UiKind::Menu)
				}

				if ui.button("🍄 隐藏笔记").clicked() { 
					self.show_notes = false;
					ui.close_kind(egui::UiKind::Menu)
				}

				if show_parallel_option {
					if ui.button("⇔ 经文对比").clicked() { 
						if let Some(verse) = self.selected_verse_num {
							self.load_parallel_verses(verse);
							self.show_parallel_window = true;
						}
						ui.close_kind(egui::UiKind::Menu)
					}
				}
		});
	}
}

impl BibleApp {
	fn show_parallel_window(&mut self, ctx: &egui::Context) {
		if !self.show_parallel_window {
			return;
		}

		//  先拷贝需要的数据
		let header = {
			let book = self.current_book_name.clone().unwrap_or("—".into());
			let chapter = self.current_chapter.clone().unwrap_or("?".into());
			let verse = self
				.selected_verse_num
				.map(|v| v.to_string())
				.unwrap_or("?".into());

			format!("{} {}:{}", book, chapter, verse)
		};

		let verses = self.parallel_verses.clone();

		//  再画窗口
		let default_width = 300.0;
		let default_height = 250.0;
		let offset = 10.0;
		// 计算窗口初始位置
		let pos = self.parallel_window_pos.map(|p| {
			let screen_rect = ctx.content_rect();
			let mut x = p.x;
			let mut y = p.y + offset; // 默认下方偏移

			// 如果下方空间不足，让窗口从上方弹出
			if y + default_height > screen_rect.bottom() {
				y = (p.y - default_height - offset).max(screen_rect.top());
			}

			// 保证 x 不超出屏幕
			x = x.min(screen_rect.right() - default_width).max(screen_rect.left());

			egui::pos2(x, y)
		});

		// 创建窗口
		let mut window = egui::Window::new(header)
			.open(&mut self.show_parallel_window)
			.resizable(true)
			.vscroll(true)
			.default_width(default_width)   
			.default_height(default_height); 

		// 如果有点击位置，则设置窗口初始位置
		if let Some(p) = pos {
			window = window.current_pos(p);
		}

		window.show(ctx, |ui| {
			let small_font = FontId::new(14.0, egui::FontFamily::Proportional);
			for pv in verses {
				ui.horizontal_wrapped(|ui| {
					ui.label(
						egui::RichText::new(version_display_name(&pv.version))
						.font(small_font.clone())
						.strong()
						.color(egui::Color32::from_rgb(200, 30, 30)),
					);
					//ui.label(&pv.text);
					ui.label(egui::RichText::new(&pv.text).font(small_font.clone()));
				});
				ui.separator();
			}
		});
	}
}

//打开笔记编辑窗口
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

//版本切换
impl BibleApp {
	fn on_version_changed(&mut self, ver: String) {
		self.search_results.clear();
		self.show_search_window = false;
		self.last_search_query.clear();
		self.text_cache.clear();
		self.highlight_query = None;
		self.show_highlight = false; 
		self.editable_mode = false;
		self.show_parallel_window = false;

		let old_book = self.current_book;
		let old_chapter = self.current_chapter.clone();

		if self.current_book.is_some() {
			self.record_jump();
		}

		self.current_version = ver.clone();

    self.conn = None;           // 先清掉旧连接
    self.open_current_db();     // 调用新函数

		if let Some(conn) = &self.conn {
			self.books = load_books(conn);

			// 保持原书卷，如果原书卷不存在就选择第一个
			self.current_book = old_book
				.filter(|b| self.books.iter().any(|(n, _)| n == b))
				.or_else(|| self.books.first().map(|(n, _)| *n));

			// 如果有书卷，加载章节
			if let Some(book) = self.current_book {
				let mut chapters = load_chapters(conn, book);
				chapters.sort_by_key(|c| chapter_number(c));
				self.chapters = chapters;

				// 尝试保持原章节，否则选择第一章
				self.current_chapter = old_chapter
					.filter(|c| self.chapters.contains(c))
					.or_else(|| self.chapters.first().cloned());

				// 加载章节内容
				if let Some(ch_str) = &self.current_chapter {
					let ch_num = ch_str.parse().unwrap_or(1);
					self.content = load_chapter_content(conn, book, ch_num);
				} else {
					self.content.clear();
				}
			} else {
				// 当前版本没有书卷（极少情况）
				self.chapters.clear();
				self.current_chapter = None;
				self.content.clear();
			}
		} 
		// 清空布局和状态
		self.content_layout = None;
		self.last_state = None;
	}

	fn on_book_selected(&mut self, book_num: i32) {
		if self.current_book == Some(book_num) {
			return; 
		}
		if self.current_book.is_some() {
			self.record_jump();
    }
		self.show_parallel_window = false;
		self.show_search_window = false;
		self.current_book = Some(book_num);
		let abbr = book_number_to_abbr(book_num);
    self.chapter_panel_title = format!("章节（{}）", abbr);

		// 确保数据库已经打开
		if let Some(conn) = &self.conn {
			// 加载章节列表
			let mut chapters = load_chapters(conn, book_num);
			chapters.sort_by_key(|c| chapter_number(c));
			self.chapters = chapters;

			// 自动选择第一章
			if let Some(first_chapter) = self.chapters.first().cloned() {
				self.current_chapter = Some(first_chapter.clone());

				let ch_num = first_chapter.parse().unwrap_or(1);
				self.content = load_chapter_content(conn, book_num, ch_num);

				// 清理布局与状态
				self.content_layout = None;
				self.last_state = None;
			} else {
				// 该书没有章节（极少情况）
				self.current_chapter = None;
				self.content.clear();
				self.content_layout = None;
				self.last_state = None;
			}
		} 
	}

	fn on_chapter_selected(&mut self, book_num: i32, ch: String) {
		//self.record_jump();
		if self.current_book == Some(book_num) && self.current_chapter == Some(ch.clone()) {
			return;
		}
		if self.current_chapter.is_some() {
			self.record_jump();
    }
		self.show_parallel_window = false;
		self.show_search_window = false;
		self.current_book = Some(book_num.clone());
		self.current_chapter = Some(ch.clone());
		let ch_num = ch.parse().unwrap_or(1);
		if let Some(conn) = &self.conn {
			self.content = load_chapter_content(conn, book_num, ch_num);
		}
	}
}

//转跳
impl BibleApp {
	fn record_jump(&mut self) {
		if let Some(current) = self.current_nav_state() {
			if self.jump_back_stack.last() != Some(&current) {
				self.jump_back_stack.push(current);
			}
			self.jump_forward_stack.clear();
		}
	}
	fn jump_back(&mut self) {
		if let Some(prev) = self.jump_back_stack.pop() {
			// 当前状态推入 forward 栈
			if let Some(current) = self.current_nav_state() {
				self.jump_forward_stack.push(current);
			}
			// 应用跳转
			self.apply_nav_state(prev);
		}
	}
	fn jump_forward(&mut self) {
		if let Some(next) = self.jump_forward_stack.pop() {
			// 当前状态推入 back 栈
			if let Some(current) = self.current_nav_state() {
				self.jump_back_stack.push(current);
			}
			// 应用跳转
			self.apply_nav_state(next);
		}
	}
	fn apply_nav_state(&mut self, state: NavState) {
		let NavState { version, book, chapter } = state;

		if self.current_version != version {
			self.current_version = version.clone();
			self.on_version_changed(version);
		}

		self.current_book = book;
		self.current_chapter = chapter.clone();

		if let (Some(book_id), Some(ch)) = (book, chapter) {
			let ch_num = ch.parse().unwrap_or(1);
			if let Some(conn) = &self.conn {
				let mut chapters = load_chapters(conn, book_id);
				chapters.sort_by_key(|c| chapter_number(c));
				self.chapters = chapters;
				self.content = load_chapter_content(conn, book_id, ch_num);
			}
		}
	}

	fn current_nav_state(&self) -> Option<NavState> {
		Some(NavState {
			version: self.current_version.clone(),
			book: self.current_book,
			chapter: self.current_chapter.clone(),
		})
	}
	fn set_number_mark(&mut self, n: u8) {
		if let Some(nav) = self.current_nav_state() {
			self.number_marks.insert(n, nav);
		}
	}

	fn jump_to_number_mark(&mut self, n: u8) {
		if let Some(target) = self.number_marks.get(&n).cloned() {
			self.record_jump();
			self.apply_nav_state(target);
		}
	}
	fn check_jump_shortcuts(&mut self, ctx: &egui::Context) {
		// 遍历当前帧所有键事件
		for event in &ctx.input(|i| i.events.clone()) {
			if let egui::Event::Key { key, pressed, modifiers, .. } = event {

				if !pressed {
					continue;
				}

				// Ctrl + O / I
				if modifiers.ctrl {
					match key {
						egui::Key::O => self.jump_back(),
						egui::Key::I => self.jump_forward(),
						_ => {}
					}
				}

				// Alt + 数字：设置标记
				if modifiers.alt {
					match key {
						egui::Key::Num1 => self.set_number_mark(1),
						egui::Key::Num2 => self.set_number_mark(2),
						egui::Key::Num3 => self.set_number_mark(3),
						egui::Key::Num4 => self.set_number_mark(4),
						egui::Key::Num5 => self.set_number_mark(5),
						egui::Key::Num6 => self.set_number_mark(6),
						egui::Key::Num7 => self.set_number_mark(7),
						egui::Key::Num8 => self.set_number_mark(8),
						egui::Key::Num9 => self.set_number_mark(9),
						_ => {}
					}
				}

				// F1–F9：跳转
				match key {
					egui::Key::F1 => self.jump_to_number_mark(1),
					egui::Key::F2 => self.jump_to_number_mark(2),
					egui::Key::F3 => self.jump_to_number_mark(3),
					egui::Key::F4 => self.jump_to_number_mark(4),
					egui::Key::F5 => self.jump_to_number_mark(5),
					egui::Key::F6 => self.jump_to_number_mark(6),
					egui::Key::F7 => self.jump_to_number_mark(7),
					egui::Key::F8 => self.jump_to_number_mark(8),
					egui::Key::F9 => self.jump_to_number_mark(9),
					_ => {}
				}
			}
		}
	}
}

//浮窗
impl BibleApp {
    fn ui_overlays(&mut self, ctx: &egui::Context, colors: &ThemeColors) {
			//show_note_window(ctx, &colors, &mut self.open_note);
			self.show_note_window(ctx, &colors);
			self.show_notes_list_window(ctx, &colors);
			self.import_export_window(ctx, &colors);
			self.show_parallel_window(ctx);
    }
}

impl eframe::App for BibleApp {
	fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
		//apply_theme(ctx, &self.theme);//挪到初始化与切换主题按钮处
		if self.ui_initialized {
			apply_theme(ctx, &self.theme);
			self.ui_initialized = false; 
		}
		let colors = get_theme_colors(ctx, &self.theme);
		//测试输入法
		//ctx.input(|i| {
		//	for e in &i.events {
		//		if matches!(e, egui::Event::Ime(_)) {
		//			eprintln!("{:?}", e);
		//		}
		//	}
		//});
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
			//let empty_rect = ui.available_rect_before_wrap();
			//let mut empty_resp = ui.allocate_rect(empty_rect, egui::Sense::click());
			//self.show_right_click_menu(&mut empty_resp);
		});


		self.ui_overlays(ctx, &colors);

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
