use eframe::egui;

#[derive(PartialEq)]
pub enum Theme {
	Dark,
	Light,
}

pub struct ThemeColors {
	pub book_selected_bg: egui::Color32,     
	pub chapter_selected_bg: egui::Color32,  
	pub book_unselected_bg: egui::Color32,   
	pub chapter_unselected_bg: egui::Color32,
	pub text_color: egui::Color32,           
	pub menu_button_bg: egui::Color32,
	pub menu_button_hover: egui::Color32,
	pub menu_button_active: egui::Color32,
	pub menu_stroke: egui::Color32,         
	pub comment_text_color: egui::Color32,         
	pub item_bg: egui::Color32,         
	pub item_text: egui::Color32,         
	pub selected_text_color: egui::Color32,         
	pub link_color: egui::Color32,         
	pub search_hl_bg: egui::Color32,         
	pub search_hl_fg: egui::Color32,         
}

pub fn apply_theme(ctx: &egui::Context, theme: &Theme) -> ThemeColors {
	match theme {
		Theme::Dark => {
			let mut visuals = egui::Visuals::dark();
			visuals.override_text_color = Some(egui::Color32::from_rgb(220, 220, 220));

			visuals.window_stroke = egui::Stroke {
				width: 2.0,               
				color: egui::Color32::from_rgb(200, 100, 0), 
			};

			ctx.set_visuals(visuals.clone());



			ThemeColors {
				book_selected_bg: egui::Color32::from_rgb(50, 100, 160),
				chapter_selected_bg: egui::Color32::from_rgb(60, 140, 80),
				book_unselected_bg: visuals.widgets.inactive.bg_fill,
				chapter_unselected_bg: visuals.widgets.inactive.bg_fill,
				text_color: egui::Color32::from_rgb(220, 220, 220),
				menu_button_bg: visuals.widgets.inactive.bg_fill,    
				menu_button_hover: egui::Color32::from_rgb(24, 7, 46),
				menu_button_active: visuals.widgets.active.bg_fill,
				menu_stroke: egui::Color32::from_rgb(58,140,255),
				comment_text_color: egui::Color32::from_rgb(150, 150, 150),
				item_bg: visuals.selection.bg_fill,
				item_text: egui::Color32::from_rgb(220, 220, 220),
				selected_text_color: egui::Color32::from_rgb(220, 220, 220),
				link_color: egui::Color32::from_rgb(0, 191, 255),
				search_hl_bg: egui::Color32::from_rgb(255, 215, 0),
				search_hl_fg: egui::Color32::BLACK,
			}
		}
		Theme::Light => {
			let mut visuals = egui::Visuals::light();
			visuals.panel_fill = egui::Color32::from_rgb(242, 235, 217);
			visuals.override_text_color = Some(egui::Color32::BLACK);
			//visuals.selection.bg_fill = egui::Color32::from_rgb(71, 78, 86);
			visuals.selection.bg_fill = egui::Color32::from_rgb(41, 134, 204);

			visuals.hyperlink_color = egui::Color32::from_rgb(0, 128, 128); 

			// 按钮背景色
			visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(230, 220, 200);
			//visuals.widgets.hovered.bg_fill  = egui::Color32::from_rgb(240, 230, 210);
			visuals.widgets.hovered.bg_fill  = egui::Color32::from_rgb(255, 215, 0);
			visuals.widgets.active.bg_fill   = egui::Color32::from_rgb(210, 200, 180);

			//弹窗背景色
			//visuals.window_fill = egui::Color32::from_rgb(255, 240, 186);
			visuals.window_stroke = egui::Stroke {
				width: 2.0,               
				color: egui::Color32::from_rgb(107, 79, 63), 
			};


			ctx.set_visuals(visuals.clone());


			ThemeColors {
				book_selected_bg: egui::Color32::from_rgb(50, 100, 160),
				chapter_selected_bg: egui::Color32::from_rgb(60, 140, 80),
				book_unselected_bg: egui::Color32::from_rgb(229,215,179),
				chapter_unselected_bg: egui::Color32::from_rgb(229,215,179),
				text_color: egui::Color32::BLACK,
				menu_button_bg: egui::Color32::from_rgb(230, 220, 200),  
				menu_button_hover: egui::Color32::from_rgb(191, 140, 36),
				menu_button_active: egui::Color32::from_rgb(210, 200, 180),
				menu_stroke: egui::Color32::from_rgb(107, 79, 63),
				comment_text_color: egui::Color32::from_rgb(142, 131, 113),
				item_bg: egui::Color32::from_rgb(180, 200, 220),
				item_text: egui::Color32::BLACK,
				selected_text_color: egui::Color32::from_rgb(220, 220, 220),
				link_color: egui::Color32::from_rgb(0, 128, 128),
				search_hl_bg: egui::Color32::from_rgb(255, 215, 0),
				search_hl_fg: egui::Color32::BLACK,
			}
		}
	}
}
