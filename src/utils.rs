use rusqlite::Connection;
use std::path::Path;
use std::cmp::Ordering;
use crate::theme::ThemeColors;


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
pub fn version_display_name(version: &str) -> String {
    version.trim_end_matches(".sqlite3").trim_end_matches(".db").to_string()
}

fn has_chinese(s: &str) -> bool {
    s.chars().any(|c| {
        ('\u{4E00}'..='\u{9FFF}').contains(&c)
    })
}

/// 中文优先，其余字典序排序
pub fn sort_versions_chinese_first(versions: &mut Vec<String>) {
    versions.sort_by(|a, b| {
        let a_cn = has_chinese(a);
        let b_cn = has_chinese(b);

        match (a_cn, b_cn) {
            (true, false) => Ordering::Less,    // a 在前
            (false, true) => Ordering::Greater, // b 在前
            _ => a.cmp(b),                      // 同类：字典序
        }
    });
}

/// 只读多行文本显示
//pub fn readonly_content_text(ui: &mut egui::Ui, text: &str) -> egui::Response {
//	let response = ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
//		ui.set_width(ui.available_width()); 
//		ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
//			ui.set_width(ui.available_width() - 12.0);
//			let label = egui::RichText::new(text);
//			ui.add(
//				egui::Label::new(label)
//				.sense(egui::Sense::click())
//				.selectable(true)
//			)
//		}).inner
//	}).response;
//	response
//}

pub fn readonly_content_text_highlighted(
	ui: &mut egui::Ui,
	text: &str,
	colors: &ThemeColors,
	highlight: Option<&str>,
) -> egui::Response {
	let response = ui
		.with_layout(
			egui::Layout::top_down_justified(egui::Align::LEFT),
			|ui| {
				ui.set_width(ui.available_width());

				ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
					ui.set_width(ui.available_width() - 12.0);

					let mut job = egui::text::LayoutJob::default();
					let body_font_id = ui.style().text_styles[&egui::TextStyle::Body].clone();

					match highlight {
						Some(query) if !query.is_empty() => {
							highlight_search_terms(
								text,
								query,
								colors,
								&mut job,
								&body_font_id,
							);
						}
						_ => {
							job.append(
								text,
								0.0,
								egui::TextFormat {
									font_id: body_font_id,
									color: colors.text_color,
									..Default::default()
								},
							);
						}
					}

					ui.add(
						egui::Label::new(job)
						.sense(egui::Sense::click())
						.selectable(true),
					)
				})
				.inner
			},
			)
				.inner;

	response
}

pub fn highlight_search_terms(
    text: &str,
    search_terms: &str,
		colors: &ThemeColors,
    job: &mut egui::text::LayoutJob, 
		font_id: &egui::FontId,
) {
    let mut last_index = 0;
    let lower_text = text.to_lowercase();
    let lower_query = search_terms.to_lowercase();

    let mut start = 0;
    while let Some(pos) = lower_text[start..].find(&lower_query) {
        let match_start = start + pos;
        let match_end = match_start + search_terms.len();

        // 普通文本
        if match_start > last_index {
            job.append(
                &text[last_index..match_start],
                0.0,
                egui::TextFormat {
									font_id: font_id.clone(),
                    color: colors.text_color,
                    ..Default::default()
                },
            );
        }

        // 高亮文本
        job.append(
            &text[match_start..match_end],
            0.0,
            egui::TextFormat {
									font_id: font_id.clone(),
								color: colors.search_hl_fg,            
								background: colors.search_hl_bg,
                ..Default::default()
            },
        );

        last_index = match_end;
        start = match_end;
    }

    // 剩余普通文本
    if last_index < text.len() {
        job.append(
            &text[last_index..],
            0.0,
            egui::TextFormat {
							font_id: font_id.clone(),
                color: colors.text_color,
                ..Default::default()
            },
        );
    }
}
//pub fn readonly_multiline_text(ui: &mut egui::Ui, text: &str) -> egui::Response {
//    let body_font_id = ui.style().text_styles[&egui::TextStyle::Body].clone();
//    let mut mutable_content = text.to_owned();
//
//    let text_edit = egui::TextEdit::multiline(&mut mutable_content)
//        .desired_width(ui.available_width() - 12.0)
//        .frame(false)
//        .interactive(true) 
//        .clip_text(false)
//				.font(body_font_id);
//
//    let response = ui.add(text_edit);
//    
//    response
//}

pub fn book_number_to_abbr(number: i32) -> &'static str {
    // 圣经 66 卷的缩写，按常见顺序排列
    const ABBRS: [&str; 66] = [
        "创", "出", "利", "民", "申", "书", "士", "得", "撒上", "撒下",
        "王上", "王下", "代上", "代下", "拉", "尼", "斯", "伯", "诗", "箴",
        "传", "歌", "赛", "耶", "哀", "结", "但", "何", "珥", "摩",
        "俄", "拿", "弥", "鸿", "哈", "番", "该", "亚", "玛",
        "太", "可", "路", "约", "徒", "罗", "林前", "林后", "加", "弗",
				"腓", "西", "帖前", "帖后", "提前", "提后", "多",
        "门", "来", "雅", "彼前", "彼后", "约一", "约二", "约三", "犹", "启"
    ];

    if number >= 1 && number <= 66 {
        ABBRS[(number - 1) as usize]
    } else {
        "未知"
    }
}


pub fn draw_hover_button(
    ui: &mut egui::Ui,
    text: &str,
    size: egui::Vec2,
		colors: &ThemeColors,
) -> egui::Response {
    // 分配按钮矩形，处理点击、悬停
	let (id, rect) = ui.allocate_space(size); // 返回 (Id, Rect)
	let response = ui.interact(rect, id, egui::Sense::click()); // Response

    // 绘制背景
    let fill = if response.hovered() { colors.menu_button_hover } else { colors.item_bg };
    ui.painter().rect_filled(rect, egui::Rounding::same(4.0), fill);

    // 绘制文字（居中）
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        egui::TextStyle::Button.resolve(&ui.style()),
        colors.text_color,
    );

    // 返回 Response，方便判断点击
    response
}




