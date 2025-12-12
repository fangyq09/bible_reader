use rusqlite::Connection;
use std::path::Path;


/// 从 SQLite 数据库加载书卷
pub fn load_books(db_path: &Path) -> Vec<(i32, String)> {
	let conn = Connection::open(db_path).unwrap();
	let mut stmt = conn
		.prepare("SELECT number, human FROM books ORDER BY number")
		.unwrap();
	let rows = stmt
		.query_map([], |row| {
			let num: i32 = row.get(0)?;
			let human: String = row.get(1)?;
			Ok((num, human))
		})
	.unwrap();
	rows.map(|r| r.unwrap()).collect()
}

/// 从 SQLite 数据库加载章节列表
pub fn load_chapters(db_path: &Path, book_number: i32) -> Vec<String> {
	let conn = Connection::open(db_path).unwrap();

	let osis: String = conn
		.query_row(
			"SELECT osis FROM books WHERE number = ?",
			[book_number],
			|row| row.get(0),
		)
		.unwrap_or_default();

	let mut stmt = conn
		.prepare(
			"SELECT reference_osis FROM chapters WHERE reference_osis LIKE ?1 || '.%' ORDER BY reference_osis"
		)
		.unwrap();

	let rows = stmt
		.query_map([osis], |row| row.get::<_, String>(0))
		.unwrap();

	rows.map(|r| {
		let osis_ref: String = r.unwrap();
		osis_ref.split('.').last().unwrap_or("0").to_string()
	}).collect()
}

/// 从 SQLite 读取章节内容
pub fn load_chapter_content(db_path: &Path, book_number: i32, chapter: i32) -> String {
	let conn = Connection::open(db_path).unwrap();

	let osis: String = conn
		.query_row(
			"SELECT osis FROM books WHERE number = ?",
			[book_number],
			|row| row.get(0),
		)
		.unwrap_or_default();

	let reference = format!("{}.{}", osis, chapter);

	conn.query_row(
		"SELECT content FROM chapters WHERE reference_osis = ?1",
		[reference],
		|row| row.get(0),
	).unwrap_or_else(|_| "（未找到章节内容）".to_string())
}

/// 章节排序辅助
pub fn chapter_number(chap: &str) -> u32 {
	chap.parse::<u32>().unwrap_or(0)
}

/// 章节显示名
pub fn chapter_display_name(chap: &str) -> String {
	if chap == "0" {
		"简介".to_string()
	} else {
		format!("第 {} 章", chap.parse::<u32>().unwrap_or(0))
	}
}

/// 版本显示名
//pub fn version_display_name(version: &str) -> &str {
//	version.trim_end_matches(".sqlite3").trim_end_matches(".db")
//}

pub fn version_display_name(version: &str) -> String {
    version.trim_end_matches(".sqlite3").trim_end_matches(".db").to_string()
}

/// 只读多行文本显示
//pub fn readonly_multiline_text(ui: &mut egui::Ui, text: &str) -> egui::Response {
//	let response = ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
//		ui.set_width(ui.available_width()); // 确保整个 layout 区域占满宽度
//		// 使用 with_layout 包装，以应用所需的右侧边距
//		ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
//			// 确保 Label 在这个宽度内换行
//			ui.set_width(ui.available_width() - 12.0);
//			let label = egui::RichText::new(text);
//			ui.add(
//				egui::Label::new(label)
//				.sense(egui::Sense::click_and_drag())
//				.selectable(true)
//			)
//		}).inner
//	}).response;
//	// 返回响应
//	response
//}
pub fn readonly_multiline_text(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let body_font_id = ui.style().text_styles[&egui::TextStyle::Body].clone();
    let mut mutable_content = text.to_owned();

    let text_edit = egui::TextEdit::multiline(&mut mutable_content)
        .desired_width(ui.available_width() - 12.0)
        .frame(false)
        .interactive(true) //interactive(true) 以确保 TextEdit 捕获点击和选择
        .clip_text(false)
				.font(body_font_id);

    let response = ui.add(text_edit);
    
    response
}
