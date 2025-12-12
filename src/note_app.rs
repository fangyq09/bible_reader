use eframe::egui;
use egui::{RichText,ScrollArea};
use crate::notes::{Notedb,save_note,delete_note};

pub struct NoteApp {
		pub note: Notedb,
}

impl eframe::App for NoteApp {
	fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

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
					if ui.add_sized([btn_w, btn_h], egui::Button::new("🗑删除")).clicked() {
						if let Err(e) = delete_note("notes", &note.id) {
							eprintln!("删除笔记失败 id={}: {:?}", note.id, e);
						} else {
							println!("删除笔记 id={}", note.id);
						}
						ctx.send_viewport_cmd(egui::ViewportCommand::Close);
					}
				});
				ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
					if ui.add_sized([btn_w, btn_h], egui::Button::new("保存")).clicked() {
						save_note("notes", note);
						ctx.send_viewport_cmd(egui::ViewportCommand::Close);
					}
				});
			});
			ui.add_space(2.0);
		});
	}
}
