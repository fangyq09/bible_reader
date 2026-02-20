use chrono::Utc;
use serde::{Serialize, Deserialize};
use crate::theme::{ThemeColors,font_size_tiny};
use crate::BibleApp;
use crate::ParallelVerse;
use crate::utils::{version_display_name,sort_versions_chinese_first};
use std::fs;
use rfd::FileDialog;

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

#[derive(Debug)]
enum SearchMode {
    Default,
    Title,
    Content,
    Keyword,
		Subject,
}

#[derive(Debug)]
struct SearchTerm {
    mode: SearchMode,
    text: String,
}

#[derive(Debug)]
struct SearchQuery {
    terms: Vec<SearchTerm>,
}

//追加笔记样式
impl BibleApp {
pub fn show_appended_notes(
    &mut self,
    ui: &mut eframe::egui::Ui,
) {
    if self.appended_notes_current.is_empty() {
        return;
    }

    ui.add_space(10.0);
    ui.separator();

    for i in 0..self.appended_notes_current.len() {
			let note = &self.appended_notes_current[i];
        ui.horizontal(|ui| {
            ui.label("📝");
						let title = note.title.as_deref().unwrap_or("<无标题>");
						let subject = note.subject.as_deref().unwrap_or("");
						let reference = note.reference.as_deref().unwrap_or("");

						let display_text = if !reference.is_empty() {
							format!("【{}】「{}」 （{}）", subject, title, reference)
						} else {
							format!("【{}】「{}」", subject, title)
						};
            if ui.link(&display_text).clicked(){
                self.current_note = Some(self.appended_notes_current[i].clone());
                self.note_window_open = true;
            }
        });
    }
}
}

impl BibleApp {
	pub fn get_appended_notes(&mut self) {
		let current_state = match self.current_nav_state() {
			Some(s) => s,
			None => return,
		};

		if self.last_appended_notes_state.as_ref() != Some(&current_state) {
			self.appended_notes_current = self.load_notes("notes", "append");
			self.last_appended_notes_state = Some(current_state);
		}
	}
}

//笔记阅读窗口
impl BibleApp {
	pub fn show_note_window(&mut self, ctx: &egui::Context, colors: &ThemeColors) {
		if !self.note_window_open {
			return;
		}
		let note = self.current_note.clone().unwrap();

		egui::Area::new("note_window_area".into())
			.default_pos([300.0, 200.0])
			.show(ctx, |ui| {
				egui::Frame::window(ui.style()).show(ui, |ui| {
					ui.set_max_size(egui::vec2(500.0, 400.0));

					// 笔记内容滚动区域
					egui::containers::ScrollArea::vertical().show(ui, |ui| {
						ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
							ui.heading(note.title.as_deref().unwrap_or("笔记"));
							if let Some(reference) = note.reference.as_deref() {
								if !reference.is_empty() {
									ui.label(
										egui::RichText::new(format!("引用：{}", reference))
										//.size(10.0)
                                        .text_style(font_size_tiny())
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

							// 删除按钮
							if let Some(note_id) = self.current_note.as_ref().map(|n| n.id.clone()) {
								if ui.add_sized([btn_w, btn_h], egui::Button::new("🗑删除")).clicked() {
									if let Err(e) = delete_note("notes", &note_id) {
										eprintln!("删除笔记失败 id={}: {:?}", note_id, e);
									} else {
										self.current_note = None;
										self.note_window_open = false;
										self.last_appended_notes_state = None;
										self.last_state = None;     
										self.content_layout = None;
									}
								}
							}

							ui.add_space(15.0);

							if let Some(created) = &note.created_at {
								ui.label(
									egui::RichText::new(format!("创建: {}", created))
									//.size(10.0)
									.text_style(font_size_tiny())
									.color(colors.comment_text_color)
								);
							}

							ui.add_space(20.0);

							// 编辑按钮
							if ui.add_sized([btn_w, btn_h], egui::Button::new("编辑")).clicked() {
								self.open_noteapp_window(self.current_note.as_ref());
								self.current_note = None;
								self.note_window_open = false;
							}

							ui.add_space(15.0);

							// 修改时间
							if let Some(updated) = &note.updated_at {
								ui.label(
									egui::RichText::new(format!("修改: {}", updated))
									//.size(10.0)
									.text_style(font_size_tiny())
									.color(colors.comment_text_color)
								);
							}

							ui.add_space(15.0);

							// 关闭按钮
							ui.with_layout(egui::Layout::right_to_left(Default::default()), |ui| {
								if ui.add_sized([btn_w, btn_h], egui::Button::new("关闭")).clicked() {
									self.note_window_open = false;
								}
							});
						});
						ui.separator();
					});
				});
			});
	}
}

//笔记列表样式
fn draw_notes_list(
	ui: &mut egui::Ui,
	colors: &ThemeColors,
	notes: &Vec<Notedb>,
	current_note: &mut Option<Notedb>,
	note_window_open: &mut bool,
) -> bool {
	if notes.is_empty() {
		ui.label("暂无笔记");
		return false;
	}

	let mut request_close = false;

    for note in notes {
        let title = note.title.as_deref().unwrap_or("<无标题>");
        let subject = note.subject.as_deref().unwrap_or("");
				let body = note.body.as_deref().unwrap_or("");
				let version = version_display_name(note.version.as_deref().unwrap_or(""));
				let book_name = note.book_name.as_deref().unwrap_or("");
				let chapter = note.chapter.as_deref().unwrap_or("");
				let note_location = format!("（{}:{}:{}）", version, book_name, chapter);

        let title_text = if subject.is_empty() {
            format!("📝「{}」", title)
        } else {
            format!("📝【{}】「{}」", subject, title)
        };

				//===== 无正文预览 ===== 
				//if hover_link(ui, &title_text, &colors) {
				//	*current_note = Some(note.clone());
				//	*note_window_open = true;
				//	request_close = true;
				//}

				let title_response = ui.link(&title_text);

        // ===== 第二行：正文预览（单行） =====
        let _body_response = ui.add(
            egui::Label::new(body)
                .truncate()   // 只显示第一行
        );
        ui.add(
            egui::Label::new(
                egui::RichText::new(note_location)
                //.size(10.0)
                .text_style(font_size_tiny())
                .color(colors.comment_text_color)
                )
            .truncate()  
            );

        // ===== 点击任意一行都打开 =====
        //if title_response || body_response.clicked() {
        if title_response.clicked() {
            *current_note = Some(note.clone());
            *note_window_open = true;
            request_close = true;
        }

				//ui.add_space(6.0);
        ui.separator();
    }

    request_close
}

//笔记列表窗口
impl BibleApp {
    pub fn show_notes_list_window(
        &mut self,
        ctx: &egui::Context,
        colors: &ThemeColors,
    ) {
        if !self.show_notes_list_window {
            return;
        }

        let mut close_window = false;
				let mut open_note = false;
				let mut do_search = false; 

        egui::Window::new(egui::RichText::new("📒 笔记列表").size(14.0))
            .open(&mut self.show_notes_list_window)
            .resizable(true)
            .default_width(500.0)
            .show(ctx, |ui| {
							let response = ui.add(
								egui::TextEdit::singleline(&mut self.notes_search_keyword)
								.hint_text(
									egui::RichText::new("搜索笔记")
									.color(colors.comment_text_color)
                                    .text_style(egui::TextStyle::Small),
									//.size(14.0),
								)
								.desired_width(f32::INFINITY),
							);

							if response.clicked() || response.gained_focus() || response.has_focus(){
								self.active_search_type = "notes".to_string();
							}
							if response.clicked_elsewhere() {
								self.active_search_type = "".to_string();
							}

							if ui.input(|i| i.key_pressed(egui::Key::Enter))&& self.active_search_type == "notes"
							{
								do_search = true;
							}

							ui.separator();

							egui::ScrollArea::vertical()
								.auto_shrink([false; 2])
								.show(ui, |ui| {
									if draw_notes_list(
										ui,
										colors,
										&self.notes_cache,
										&mut self.current_note,
										&mut open_note,
									) {
										close_window = true;
									}
								});
            });

        // 在 closure 结束之后再关窗口
        if close_window {
            self.show_notes_list_window = false;
        }

				if open_note {
					self.note_window_open = true; 
				}

				if do_search {
					let query = parse_search_input(&self.notes_search_keyword);
					self.notes_cache = self.search_notes_from_db("notes", &query);
				}
    }
}

//保存笔记
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

//删除笔记
pub fn delete_note(category: &str, note_id: &str) -> Result<(), rusqlite::Error> {
		let notes_dir = dirs::data_dir().unwrap().join("bible_reader/notes");
		let db_path = notes_dir.join("note.db");
		let conn = rusqlite::Connection::open(&db_path)?;

		let sql = format!("DELETE FROM {} WHERE id = ?1", category);
		conn.execute(&sql, [note_id])?;

		println!("已删除笔记 id={}", note_id);
		Ok(())
}

//读取笔记
impl BibleApp {
    pub fn load_notes(&self, category: &str, mode: &str) -> Vec<Notedb> {
        let mut notes = Vec::new();

        let notes_dir = match dirs::data_dir() {
            Some(d) => d.join("bible_reader/notes"),
            None => return notes,
        };
        let db_path = notes_dir.join("note.db");

        let conn = match rusqlite::Connection::open(&db_path) {
            Ok(c) => c,
            Err(_) => return notes,
        };

				// ===============================
        //  表不存在直接返回空
        // ===============================
        if !table_exists(&conn, category) {
            return notes;
        }

        match mode {
            // ===============================
            // 章节后附加笔记
            // ===============================
            "append" => {
                let book_num = match self.current_book {
                    Some(b) => b,
                    None => return notes,
                };
                let chapter = match &self.current_chapter {
                    Some(c) => c.clone(),
                    None => return notes,
                };

                let mut conditions = vec![
                    "book_num = ?1",
                    "chapter = ?2",
                    "version = ?3",
                ];

                if category != self.current_version {
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
										 ORDER BY COALESCE(updated_at, created_at) DESC;",
                    category,
                    where_clause
                );

                let mut stmt = match conn.prepare(&sql) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("SQL 解析失败: {:?}", e);
                        return notes;
                    }
                };

                let rows = stmt.query_map(
                    rusqlite::params![
                        book_num,
                        chapter,
                        self.current_version
                    ],
                    |row| {
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
                    },
                );

                if let Ok(iter) = rows {
                    for note in iter.flatten() {
                        notes.push(note);
                    }
                }
            }

            // ===============================
            // 加载全部笔记
            // ===============================
            "all" => {
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
										 ORDER BY COALESCE(updated_at, created_at) DESC;",
                    category
                );

                let mut stmt = match conn.prepare(&sql) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("SQL 解析失败: {:?}", e);
                        return notes;
                    }
                };

                let rows = stmt.query_map([], |row| {
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

                if let Ok(iter) = rows {
                    for note in iter.flatten() {
                        notes.push(note);
                    }
                }
            }

            // ===============================
            // 未来扩展
            // ===============================
            _ => {
                eprintln!("未知的笔记加载模式: {}", mode);
            }
        }

        notes
    }
}
fn table_exists(conn: &rusqlite::Connection, table: &str) -> bool {
    let sql = r#"
        SELECT 1
        FROM sqlite_master
        WHERE type = 'table' AND name = ?1
        LIMIT 1;
    "#;

    conn.query_row(sql, [table], |_| Ok(()))
        .is_ok()
}

//搜索笔记
fn parse_search_input(input: &str) -> SearchQuery {
    let mut terms = Vec::new();

    for part in input.split(['；', ';', '，', ',']) {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        if let Some((prefix, rest)) = part.split_once([':', '：']) {
            let text = rest.trim().to_string();
            if text.is_empty() {
                continue;
            }

            let mode = match prefix.trim().to_lowercase().as_str() {
                "title" | "标题" => SearchMode::Title,
                "content" | "内容" | "" => SearchMode::Content,
                "keyword" | "keywords" | "关键词" => SearchMode::Keyword,
								"subject" | "主题" => SearchMode::Subject,
                _ => SearchMode::Default,
            };

            terms.push(SearchTerm { mode, text });
        } else {
            // 没有前缀，走默认搜索
            terms.push(SearchTerm {
                mode: SearchMode::Default,
                text: part.to_string(),
            });
        }
    }

    SearchQuery { terms }
}
impl BibleApp {
 fn search_notes_from_db(
    &self,
    category: &str,
    query: &SearchQuery,
) -> Vec<Notedb> {
    let mut notes = Vec::new();

    if query.terms.is_empty() {
			let notes = self.load_notes("notes", "all");
        return notes;
    }

    let notes_dir = match dirs::data_dir() {
        Some(d) => d.join("bible_reader/notes"),
        None => return notes,
    };
    let db_path = notes_dir.join("note.db");

    let conn = match rusqlite::Connection::open(&db_path) {
        Ok(c) => c,
        Err(_) => return notes,
    };

    if !table_exists(&conn, category) {
        return notes;
    }

    let mut clauses: Vec<String> = Vec::new();
    let mut params: Vec<String> = Vec::new();

    for term in &query.terms {
        let text = term.text.trim();
        if text.is_empty() {
            continue;
        }

        let pat = format!("%{}%", text);

        match term.mode {
            SearchMode::Title => {
                clauses.push("title LIKE ?".to_string());
                params.push(pat);
            }
            SearchMode::Content => {
                clauses.push("body LIKE ?".to_string());
                params.push(pat);
            }
            SearchMode::Keyword => {
                clauses.push("keywords LIKE ?".to_string());
                params.push(pat);
            }
						SearchMode::Subject => {
							clauses.push("subject LIKE ?".to_string());
							params.push(pat);
						}
            SearchMode::Default => {
                clauses.push("(title LIKE ? OR keywords LIKE ?)".to_string());
                params.push(pat.clone());
                params.push(pat);
            }
        }
    }

    if clauses.is_empty() {
        return notes;
    }

    let where_clause = clauses.join(" AND ");

    let sql = format!(
        "SELECT
            id, book_num, book_name, chapter, verse_start, char_offset,
            title, keywords, reference, body, subject, version,
            created_at, updated_at
         FROM {}
         WHERE {}
         ORDER BY COALESCE(updated_at, created_at) DESC;",
        category,
        where_clause
    );

    let mut stmt = match conn.prepare(&sql) {
        Ok(s) => s,
        Err(_) => return notes,
    };

    let rows = stmt.query_map(
        rusqlite::params_from_iter(params.iter()),
        |row| {
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
        },
    );

    if let Ok(iter) = rows {
        for note in iter.flatten() {
            notes.push(note);
        }
    }

    notes
}
}

//导入圣经译本
impl BibleApp {
	pub fn import_bible_logic(&mut self) {
		// 1. 弹出文件选择对话框
		// 我们限定只能选择 .sqlite3 文件，这和你之前的过滤逻辑一致
		let file = FileDialog::new()
			.add_filter("Bible Database", &["sqlite3", "db"])
			.set_title("选择圣经译本数据库")
			.pick_file();

		if let Some(source_path) = file {
			// 2. 准备目标路径
			// self.bible_root 在 new 函数中已经是 ~/.local/share/bible_reader/sqlite
			let file_name = source_path.file_name().unwrap();
			let target_path = self.bible_root.join(file_name);

			// 3. 执行复制操作
			match fs::copy(&source_path, &target_path) {
				Ok(_) => {
					println!("成功导入译本: {:?}", file_name);

					// 4. 关键：导入后立即刷新译本列表，这样用户不用重启就能看到
					self.refresh_versions();
				}
				Err(e) => {
					eprintln!("导入译本失败: {}, 路径: {:?}", e, source_path);
					// 这里以后可以加一个弹窗提示用户失败原因
				}
			}
		}
	}

	// 辅助函数：重新扫描目录以更新 self.versions
	fn refresh_versions(&mut self) {
		if let Ok(entries) = fs::read_dir(&self.bible_root) {
			let mut new_versions: Vec<String> = entries
				.flatten()
				.filter_map(|e| {
					let path = e.path();
					if path.extension().and_then(|s| s.to_str()) == Some("sqlite3") {
						return Some(path.file_name().unwrap().to_string_lossy().to_string());
					}
					None
				})
			.collect();

			// 保持和你 new 函数中一致的排序逻辑
			sort_versions_chinese_first(&mut new_versions);
			self.versions = new_versions;
		}
	}
}

//导入导出笔记
impl BibleApp {
	/// 导出笔记：将 note.db 另存为用户选择的位置
	pub fn export_notes_logic(&mut self) {
		// 1. 准备默认文件名，带上日期，例如: bible_notes_20231027.db
		let date_str = chrono::Local::now().format("%Y%m%d").to_string();
		let default_name = format!("bible_notes_{}.db", date_str);

		// 2. 弹出保存对话框
		let target_file = rfd::FileDialog::new()
			.set_file_name(&default_name)
			.add_filter("SQLite Database", &["db", "sqlite3"])
			.set_title("导出笔记备份")
			.save_file();

		if let Some(dest_path) = target_file {
			let source_path = self.bible_root.parent().unwrap().join("notes").join("note.db");

			if source_path.exists() {
				match fs::copy(&source_path, &dest_path) {
					Ok(_) => println!("笔记已备份至: {:?}", dest_path),
					Err(e) => eprintln!("导出失败: {}", e),
				}
			} else {
				eprintln!("未找到笔记文件: {:?}", source_path);
			}
		}
	}

	/// 导入笔记：用备份文件替换当前的 note.db
	pub fn import_notes_logic(&mut self) {
		// 1. 弹出选择对话框
		let file = rfd::FileDialog::new()
			.add_filter("Bible Notes Backup", &["db", "sqlite3"])
			.set_title("选择要恢复的笔记备份")
			.pick_file();

		if let Some(source_path) = file {
			let notes_dir = self.bible_root.parent().unwrap().join("notes");
			let target_path = notes_dir.join("note.db");

			// 2. 如果当前正在显示笔记列表或编辑笔记，先关闭相关连接/缓存
			// 如果你使用了数据库连接池或长期持有连接，这里可能需要先释放 self.notes_conn = None;

			// 3. 执行覆盖操作
			match fs::copy(&source_path, &target_path) {
				Ok(_) => {
					println!("笔记恢复成功！");
					// 4. 重要：恢复后清空笔记缓存，迫使程序重新从新数据库读取
					self.notes_cache.clear(); 
					// 如果有打开的连接，最好在这里重新初始化
				}
				Err(e) => eprintln!("恢复笔记失败: {}", e),
			}
		}
	}
}

//加载对比经文
impl BibleApp {
	pub fn load_parallel_verses(&mut self, verse_num: i32) {
		self.parallel_verses.clear();

		// 获取当前章节的整数值
		let chapter_num: i32 = match &self.current_chapter {
			Some(s) => match s.parse::<i32>() {
				Ok(v) => v,
				Err(_) => { return; }
			},
			None => { return; }
		};

		// 获取当前书卷的编号 (book_num)
		let book_num = match self.current_book {
			Some(num) => num, // 假设 self.current_book 存储的就是 1-66 的编号
			None => { return; }
		};

		// 遍历所有译本
		for version in &self.versions {
			if version == &self.current_version {
				continue; 
			}

			let db_path = self.bible_root.join(version);

			let conn = match rusqlite::Connection::open(&db_path) {
				Ok(c) => c,
				Err(_) => { continue; }
			};

			// 使用 book_num, chapter_num, verse_num 进行精确匹配
			let result: rusqlite::Result<String> = conn.query_row(
				"SELECT unformatted 
								 FROM verses 
								 WHERE book_num = ?1 AND chapter_num = ?2 AND verse_num = ?3 
								 LIMIT 1",
								(book_num, chapter_num, verse_num),
								|row| row.get(0),
						);

			if let Ok(text) = result {
				self.parallel_verses.push(ParallelVerse {
					version: version.clone(),
					text,
				});
			}
		}
	}
}
