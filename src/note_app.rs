use eframe::egui;
use egui::{RichText,ScrollArea};
use crate::notes::{Notedb,save_note,delete_note};

pub struct NoteApp {
		pub note: Notedb,
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
	v.widgets.inactive.bg_fill =
		egui::Color32::from_rgb(242, 242, 238);

	v.widgets.hovered.bg_fill =
		egui::Color32::from_rgb(230, 230, 225);

	v.widgets.active.bg_fill =
		egui::Color32::from_rgb(220, 220, 215);

	// ====== 输入框 ======
	v.widgets.inactive.bg_stroke =
		egui::Stroke::new(1.0, egui::Color32::from_rgb(200, 200, 195));

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

		egui::CentralPanel::default().show(ctx, |ui| {
			let label_width = 90.0;
			ui.collapsing("笔记标题", |ui| {
				ui.horizontal(|ui| {
					ui.add_sized([label_width, 0.0],
						egui::Label::new(RichText::new("主题：").size(14.0)));
					let subject_text_edit = egui::TextEdit::singleline(
						note.subject.get_or_insert(String::new()))
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

			ScrollArea::vertical().show(ui, |ui| {
				ui.add_sized(
					[ui.available_width(), ui.available_height()],
					egui::TextEdit::multiline(note.body.get_or_insert(String::new()))
					.hint_text("笔记正文"),
				);
			}); 
			ui.separator();
		});

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
						}
						ctx.send_viewport_cmd(egui::ViewportCommand::Close);
					}
				});
				ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
					if ui.add_sized([btn_w, btn_h], egui::Button::new("保存"))
						.on_hover_cursor(egui::CursorIcon::Default)
						.clicked() {
						save_note("notes", note);
						ctx.send_viewport_cmd(egui::ViewportCommand::Close);
					}
				});
			});
			ui.add_space(2.0);
		});
	}
}
