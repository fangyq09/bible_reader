use uuid::Uuid;
use chrono::Utc;
use serde::{Serialize, Deserialize};
use crate::theme::ThemeColors;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Notedb {
    pub id: String,
    pub book_num: Option<i32>,
    pub book_name: Option<String>,
    pub chapter: Option<String>,
    pub verse_start: i32,
    pub char_offset: Option<i32>,
    pub title: Option<String>,
    pub keywords: Option<String>,
    pub reference: Option<String>,
    pub body: Option<String>,
    pub subject: Option<String>,
    pub version: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

// 右键菜单（添加笔记）
pub fn show_right_click_menu(
	response: &mut egui::Response,
	show_notes_state: &mut bool,
	version: &String,
	book_num: Option<i32>, 
	book_name: &Option<String>, 
	chapter: &Option<String>,
) {
	response.context_menu(|ui| {
		if ui.button("➕ 添加笔记").clicked() { 
			// 生成新笔记
			let note = Notedb {
				id: Uuid::new_v4().to_string(),
				created_at: Some(Utc::now().format("%Y-%m-%d").to_string()),
				book_num,
				book_name: book_name.clone(),
				chapter: chapter.clone(),
				verse_start: -1,
				char_offset: Some(0),
				version: Some(version.clone()),
				..Default::default()
			};

			// 序列化成 JSON 传给子进程
			let note_json = serde_json::to_string(&note).unwrap();

			if let Err(e) = std::process::Command::new(std::env::current_exe().unwrap())
				.arg("--note-window")
					.arg("--note-json")
					.arg(note_json)
					.spawn()
			{
				eprintln!("无法启动笔记窗口: {e}");
			}

			ui.close_kind(egui::UiKind::Menu);
		}
		if ui.button("💬显示笔记").clicked() { 
			*show_notes_state = true; 
			ui.close_kind(egui::UiKind::Menu);
		}
		if ui.button("🍄隐藏笔记").clicked() { 
			*show_notes_state = false; 
			ui.close_kind(egui::UiKind::Menu);
		}
	});
}

pub fn readonly_text_with_comments(
	ui: &mut eframe::egui::Ui,
	text: &str, 
	version: &str,
	book_num: Option<i32>,
	chapter: Option<String>,
	open_note: &mut Option<Notedb>,
) -> eframe::egui::Response {
	let body_font_id = ui.style().text_styles[&egui::TextStyle::Body].clone();
	let mut mutable_content = text.to_owned();

	let text_edit = egui::TextEdit::multiline(&mut mutable_content)
		.desired_width(ui.available_width() - 12.0)
		.frame(false)
		.interactive(true) 
		.clip_text(false)
		.font(body_font_id);

	let response = ui.add(text_edit);

	show_appended_notes(ui, version, book_num, chapter, open_note);
	response
}


fn show_appended_notes(
	ui: &mut eframe::egui::Ui,
	version: &str,
	book_num: Option<i32>,
	chapter: Option<String>,
	open_note: &mut Option<Notedb>,
) {
	let appended_notes = load_notes("notes", version, book_num, chapter);

	if appended_notes.is_empty() {
		return;
	}

	ui.add_space(10.0);
	ui.separator();
	ui.add_space(8.0);

	for note in appended_notes {
		ui.horizontal(|ui| {
			ui.label("📝");

			let title = note.title.as_deref().unwrap_or("<无标题>");
			let subject = note.subject.as_deref().unwrap_or("");

			let display_text = if let Some(reference) = note.reference.as_deref() {
				if !reference.is_empty() {
					format!("【{}】「{}」 （{}）", subject, title, reference)
				} else {
					format!("【{}】「{}」", subject, title)
				}
			} else {
				format!("【{}】「{}」", subject, title)
			};
			let btn = ui.link(display_text);

			if btn.clicked() {
				*open_note = Some(note.clone());
			}
		});

		ui.add_space(5.0);
	}
}

pub fn show_note_window(ctx: &egui::Context, colors: &ThemeColors, open_note: &mut Option<Notedb>) {
	if let Some(note) = open_note.clone() {
		egui::Area::new("note_window_area".into())
			.default_pos([300.0, 200.0])
			.show(ctx, |ui| {
				egui::Frame::window(ui.style()).show(ui, |ui| {
					ui.set_max_size(egui::vec2(500.0, 400.0));
					egui::containers::ScrollArea::vertical()
						.show(ui, |ui| {
							ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
								ui.heading(note.title.as_deref().unwrap_or("笔记"));
								if let Some(reference) = note.reference.as_deref() {
									if !reference.is_empty() {
										ui.label(
											egui::RichText::new(format!("引用：{}", reference))
											.size(10.0)
											.color(colors.comment_text_color),
										);
									}
								}
							});
							ui.separator();
							ui.label(note.body.as_deref().unwrap_or("<无内容>"));
						});

					ui.add_space(20.0);

					// 底部按钮区域
					ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
						ui.horizontal(|ui| {
							let btn_w = 80.0;
							let btn_h = 28.0;
							//let total_w = ui.available_width();
							//let spacing = (total_w - 3.0 * btn_w) / 2.0 - 7.0;

							// 左侧删除按钮
							if ui.add_sized([btn_w, btn_h], egui::Button::new("🗑删除")).clicked() {
								if let Err(e) = delete_note("notes", &note.id) {
									eprintln!("删除笔记失败 id={}: {:?}", note.id, e);
								} else {
									//println!("删除笔记 id={}", note.id);
									*open_note = None;
								}
							}
							ui.add_space(15.0);

							if let Some(created) = &note.created_at {
								ui.label(
									egui::RichText::new(format!("创建: {}", created))
									.size(10.0)
									.color(colors.comment_text_color)
								);
							}
							ui.add_space(20.0);

							// 中间编辑按钮，用空格或间距实现居中
							//ui.add_space(spacing);
							if ui.add_sized([btn_w, btn_h], egui::Button::new("编辑")).clicked() {
								if let Some(note) = open_note.clone() {
									// 将笔记序列化成 JSON
									let note_json = serde_json::to_string(&note).unwrap();

									// 启动独立进程
									if let Err(e) = std::process::Command::new(std::env::current_exe().unwrap())
										.arg("--note-window")
											.arg("--note-json")
											.arg(note_json)
											.spawn()
									{
										eprintln!("无法启动笔记编辑窗口: {e}");
									} else {
										*open_note = None;
									}
								}
							}

							ui.add_space(15.0);

							if let Some(updated) = &note.updated_at {
								ui.label(
									egui::RichText::new(format!("修改: {}", updated))
									.size(10.0)
									.color(colors.comment_text_color)
								);
							}
							ui.add_space(15.0);

							// 右侧关闭按钮
							ui.with_layout(egui::Layout::right_to_left(Default::default()), |ui| {
								if ui.add_sized([btn_w, btn_h], egui::Button::new("关闭")).clicked() {
									*open_note = None;
								}
							});
						});
						ui.separator();
					});
				});
			});
	}
}
pub fn load_notes(
	category: &str,
	version: &str,
	book_num: Option<i32>,
	chapter: Option<String>,
) -> Vec<Notedb> {
	let mut notes = Vec::new();

	let notes_dir = dirs::data_dir().unwrap().join("bible_reader/notes");
	let db_path = notes_dir.join("note.db");

	let conn = match rusqlite::Connection::open(&db_path) {
		Ok(c) => c,
		Err(_) => return notes,
	};

	// 如果 book_num 或 chapter 为空，直接返回空 Vec
	let book_num = match book_num {
		Some(b) => b,
		None => return notes,
	};
	let chapter = match chapter {
		Some(c) => c,
		None => return notes,
	};

	let mut conditions = vec!["book_num = ?1", "chapter = ?2", "version = ?3"];
	if category != version {
		conditions.push("verse_start < 0");
	}
	let where_clause = conditions.join(" AND ");

	let sql = format!(
		"SELECT 
						id, 
						book_num, 
						book_name, 
						chapter, 
						verse_start, 
						char_offset,
						title,
						keywords,
						reference,
						body,
						subject,
						version,
						created_at,
						updated_at
				FROM {}
				WHERE {} 
				ORDER BY updated_at DESC;",
				category, where_clause
		);

		let mut stmt = match conn.prepare(&sql) {
				Ok(s) => s,
				Err(e) => {
						eprintln!("SQL 解析失败: {:?}", e);
						return notes;
				}
		};

		let rows = stmt.query_map(rusqlite::params![book_num, chapter, version], |row| {
				Ok(Notedb {
						id: row.get(0)?,
						book_num: row.get(1)?,
						book_name: row.get(2)?,
						chapter: row.get(3)?,
						verse_start: row.get(4)?,
						char_offset: row.get(5)?,
						title: row.get(6)?,
						keywords: row.get(7)?,
						reference: row.get(8)?,
						body: row.get(9)?,
						subject: row.get(10)?,
						version: row.get(11)?,
						created_at: row.get(12)?,
						updated_at: row.get(13)?,
				})
		});

		match rows {
			Ok(iter) => {
				for note in iter.flatten() {
					notes.push(note);
				}
			}
			Err(e) => eprintln!("读取笔记失败: {:?}", e),
	}

		notes
}

///保存笔记
pub fn save_note(category: &str, note: &Notedb) {
	let notes_dir = dirs::data_dir().unwrap().join("bible_reader/notes");
	if let Err(e) = std::fs::create_dir_all(&notes_dir) {
		eprintln!("无法创建 notes 目录 {:?}: {:?}", notes_dir, e);
		return;
	}
	let db_path = notes_dir.join("note.db");
	let conn = rusqlite::Connection::open(&db_path).unwrap();

	let create_sql = format!(
		"CREATE TABLE IF NOT EXISTS {} (
						id TEXT PRIMARY KEY,
						book_num INTEGER,
						book_name TEXT,
						chapter TEXT,
						verse_start INTEGER,
						char_offset INTEGER,
						title TEXT,
						keywords TEXT,
						reference TEXT,
						body TEXT,
						subject TEXT,
						version TEXT,
						created_at TEXT,
						updated_at TEXT
				);",
				category
		);

		if let Err(e) = conn.execute_batch(&create_sql) {
				eprintln!("创建表 {} 失败: {:?}", category, e);
				return;
		}

		let now = Utc::now().format("%Y-%m-%d").to_string();

		let insert_sql = format!(
				"INSERT OR REPLACE INTO {} (
					id, book_num, book_name, chapter, verse_start, char_offset,
						title, keywords, reference, body, subject, version, created_at, updated_at
				) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
				category
		);

		let res = conn.execute(
				&insert_sql,
				rusqlite::params![
						note.id,
						note.book_num,
						note.book_name,
						note.chapter,
						note.verse_start,
						note.char_offset,
						note.title.as_deref().unwrap_or(""),
						note.keywords.as_deref().unwrap_or(""),
						note.reference.as_deref().unwrap_or(""),
						note.body.as_deref().unwrap_or(""),
						note.subject.as_deref().unwrap_or(""),
						note.version.as_deref().unwrap_or(""),
						note.created_at.as_deref().unwrap_or(""),
						now, // updated_at
				],
		);

		match res {
				Ok(_) => println!("已保存笔记 id={}", note.id),
				Err(e) => eprintln!("保存笔记失败: {:?}", e),
		}
}

pub fn delete_note(category: &str, note_id: &str) -> Result<(), rusqlite::Error> {
		let notes_dir = dirs::data_dir().unwrap().join("bible_reader/notes");
		let db_path = notes_dir.join("note.db");
		let conn = rusqlite::Connection::open(&db_path)?;

		let sql = format!("DELETE FROM {} WHERE id = ?1", category);
		conn.execute(&sql, [note_id])?;

		println!("已删除笔记 id={}", note_id);
		Ok(())
}
