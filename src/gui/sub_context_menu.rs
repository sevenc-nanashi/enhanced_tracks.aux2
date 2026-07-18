use aviutl2_eframe::egui;

pub(super) fn show<R>(
    response: &egui::Response,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> Option<egui::InnerResponse<R>> {
    let context_menu_id = response.id.with("sub_context_menu");
    let context_menu_position_id = context_menu_id.with("position");
    let context_menu_requested = response.secondary_clicked();
    if context_menu_requested {
        let position = response
            .interact_pointer_pos()
            .expect("A secondary-clicked response must have a pointer position");
        response.ctx.data_mut(|data| {
            data.insert_temp(context_menu_id, true);
            data.insert_temp(context_menu_position_id, position);
        });
        response.ctx.request_repaint();
    }

    let mut context_menu_open = response
        .ctx
        .data(|data| data.get_temp::<bool>(context_menu_id) == Some(true));
    let popup_response = if context_menu_open && !context_menu_requested {
        let position = response.ctx.data(|data| {
            data.get_temp::<egui::Pos2>(context_menu_position_id)
                .expect("An open sub-context menu must have a position")
        });
        egui::containers::Popup::new(
            context_menu_id,
            response.ctx.clone(),
            egui::containers::PopupAnchor::Position(position),
            response.layer_id,
        )
        .open_bool(&mut context_menu_open)
        .kind(egui::containers::PopupKind::Menu)
        .layout(egui::Layout::top_down_justified(egui::Align::Min))
        .show(|ui| {
            egui::menu::menu_style(ui.style_mut());
            add_contents(ui)
        })
    } else {
        None
    };

    if !context_menu_open {
        response.ctx.data_mut(|data| {
            data.remove::<bool>(context_menu_id);
            data.remove::<egui::Pos2>(context_menu_position_id);
        });
    }
    popup_response
}
