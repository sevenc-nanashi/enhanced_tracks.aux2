use super::*;

impl KeyframesGui {
    pub fn show_timecontrol_presets(ui: &mut egui::Ui) -> Option<crate::keyframe::TimeControl> {
        let mut selected = None;
        ui.label(aviutl2::config::translate("プリセット"));
        ui.add_space(4.0);
        let row_height = 58.0;
        let available_width = ui.available_width();
        let spacing = 8.0;
        let target_width = 200.0;
        let columns = (((available_width + spacing) / (target_width + spacing))
            .floor()
            .max(1.0)) as usize;
        let preset_width = ((available_width - spacing * ((columns - 1) as f32))
            / (columns as f32))
            .max(target_width);
        let presets = crate::keyframe::timecontrol_presets();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show_rows(
                ui,
                row_height,
                presets.len().div_ceil(columns),
                |ui, rows| {
                    for row_index in rows {
                        let row = presets.iter().skip(row_index * columns).take(columns);
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = spacing;
                            for preset in row {
                                let (rect, response) = ui.allocate_exact_size(
                                    egui::vec2(preset_width, row_height),
                                    egui::Sense::click(),
                                );
                                let hovered = response.hovered();
                                if hovered {
                                    ui.painter().rect_filled(
                                        rect,
                                        4.0,
                                        GUI_COLORS.object_section.linear_multiply(1.15),
                                    );
                                }

                                let text_rect = egui::Rect::from_min_max(
                                    rect.left_top() + egui::vec2(6.0, 4.0),
                                    egui::pos2(
                                        rect.left() + target_width / 3.0,
                                        rect.bottom() - 4.0,
                                    ),
                                );
                                let layout = egui::text::LayoutJob::simple(
                                    preset.name.to_string(),
                                    egui::FontId::proportional(13.0),
                                    GUI_COLORS.text,
                                    text_rect.width(),
                                );
                                let galley = ui.painter().layout_job(layout);
                                ui.painter().galley(
                                    text_rect
                                        .left_center()
                                        .tap_mut(|pos| pos.y -= galley.size().y / 2.0),
                                    galley,
                                    GUI_COLORS.text,
                                );
                                //
                                // ui.painter().text(
                                //     text_rect.left_top(),
                                //     egui::Align2::LEFT_TOP,
                                //     preset.name,
                                //     egui::FontId::proportional(13.0),
                                //     GUI_COLORS.text,
                                // );
                                // TODO: 作者とか出したいかも
                                // ui.painter().text(
                                //     text_rect.left_bottom(),
                                //     egui::Align2::LEFT_BOTTOM,
                                //     preset.category,
                                //     egui::FontId::proportional(11.0),
                                //     GUI_COLORS.text.linear_multiply(0.75),
                                // );

                                let preview_rect = egui::Rect::from_min_max(
                                    egui::pos2(text_rect.right() + 6.0, rect.top() + 6.0),
                                    egui::pos2(rect.right() - 6.0, rect.bottom() - 6.0),
                                );
                                if preview_rect.width() > 8.0 && preview_rect.height() > 8.0 {
                                    Self::draw_timecontrol_preset_preview(
                                        ui.painter(),
                                        &preset.timecontrol,
                                        preview_rect,
                                    );
                                }

                                if response.double_clicked() {
                                    selected = Some(preset.timecontrol.clone());
                                }
                            }
                        });
                        ui.add_space(spacing);
                    }
                },
            );
        selected
    }

    pub fn draw_timecontrol_preset_preview(
        painter: &egui::Painter,
        timecontrol: &crate::keyframe::TimeControl,
        rect: egui::Rect,
    ) {
        let (min_y, max_y) = Self::timecontrol_vertical_bounds(timecontrol);
        let viewport = TimeControlViewport { rect, min_y, max_y };
        painter.rect_stroke(
            rect,
            2.0,
            egui::Stroke::new(1.0, GUI_COLORS.grid_line),
            egui::StrokeKind::Inside,
        );
        for y in [0.0, 1.0] {
            if min_y <= y && y <= max_y {
                let y_pos = viewport.graph_to_screen([0.0, y]).y;
                painter.line_segment(
                    [
                        egui::pos2(rect.left(), y_pos),
                        egui::pos2(rect.right(), y_pos),
                    ],
                    egui::Stroke::new(1.0, GUI_COLORS.grid_line.linear_multiply(0.7)),
                );
            }
        }
        Self::draw_timecontrol_curve(painter, timecontrol, viewport, true);
    }
}
