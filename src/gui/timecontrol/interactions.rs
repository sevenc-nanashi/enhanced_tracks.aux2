use super::*;

impl KeyframesGui {
    pub fn show_timecontrol_handles(
        ui: &mut egui::Ui,
        painter: &egui::Painter,
        timecontrol: &mut crate::keyframe::TimeControl,
        selected_point: &mut usize,
        context_menu_position: &mut Option<[f64; 2]>,
        viewport: TimeControlViewport,
        visible_y_bounds: &mut Option<TimeControlVerticalBounds>,
        vertical_bounds: TimeControlVerticalBounds,
    ) -> (bool, bool, bool) {
        let mut changed = false;
        let mut commit_requested = false;
        let mut structure_changed = false;

        for point_index in 0..timecontrol.points.len() {
            for handle_kind in [TimeControlHandleKind::In, TimeControlHandleKind::Out] {
                let handle_segment_index = match handle_kind {
                    TimeControlHandleKind::In => point_index.checked_sub(1),
                    TimeControlHandleKind::Out => Some(point_index),
                };
                if handle_segment_index.and_then(|index| timecontrol.segment_mode(index))
                    != Some(crate::keyframe::TimeControlMode::Bezier)
                {
                    continue;
                }
                let Some(handle) = (match handle_kind {
                    TimeControlHandleKind::In => timecontrol.points[point_index].in_handle,
                    TimeControlHandleKind::Out => timecontrol.points[point_index].out_handle,
                }) else {
                    continue;
                };

                let handle_pos = viewport.graph_to_screen(handle);
                let handle_rect = egui::Rect::from_center_size(handle_pos, egui::Vec2::splat(18.0));
                let handle_response = ui.interact(
                    handle_rect,
                    ui.id()
                        .with(("timecontrol_handle", point_index, handle_kind.id())),
                    egui::Sense::click_and_drag(),
                );
                if handle_response.clicked() || handle_response.dragged() {
                    *selected_point = point_index;
                }
                if handle_response.secondary_clicked() {
                    *selected_point = point_index;
                    *context_menu_position = handle_response
                        .interact_pointer_pos()
                        .map(|pos| viewport.screen_to_graph(pos));
                }
                if handle_response.dragged()
                    && let Some(pointer_pos) = handle_response.interact_pointer_pos()
                {
                    let position = Self::timecontrol_drag_position(
                        viewport.screen_to_graph(pointer_pos),
                        Self::timecontrol_drag_modifiers(ui),
                        Some(timecontrol.points[point_index].position[1]),
                    );
                    let new_point = Self::constrain_timecontrol_handle_position(
                        timecontrol,
                        point_index,
                        handle_kind,
                        position,
                    );
                    let changed_handle = match handle_kind {
                        TimeControlHandleKind::In => {
                            timecontrol.points[point_index].in_handle != Some(new_point)
                        }
                        TimeControlHandleKind::Out => {
                            timecontrol.points[point_index].out_handle != Some(new_point)
                        }
                    };
                    if changed_handle {
                        Self::set_timecontrol_handle(
                            timecontrol,
                            point_index,
                            handle_kind,
                            new_point,
                        );
                        changed = true;
                    }
                    Self::scroll_timecontrol_y_for_drag(
                        ui,
                        visible_y_bounds,
                        viewport,
                        vertical_bounds,
                        pointer_pos,
                    );
                }
                commit_requested |= handle_response.drag_stopped();
                let color = if handle_response.hovered() || handle_response.dragged() {
                    GUI_COLORS.anchor_hover
                } else {
                    GUI_COLORS.anchor
                };
                painter.circle_filled(handle_pos, 4.0, color.linear_multiply(0.85));
                handle_response.context_menu(|ui| {
                    if ui
                        .button(aviutl2::config::translate("中継点追加"))
                        .clicked()
                    {
                        *selected_point = Self::insert_timecontrol_point(
                            timecontrol,
                            context_menu_position
                                .unwrap_or_else(|| viewport.screen_to_graph(handle_pos)),
                        );
                        changed = true;
                        commit_requested = true;
                        structure_changed = true;
                        ui.close();
                    }
                    ui.separator();
                    if Self::show_timecontrol_handle_menu(ui, timecontrol, selected_point) {
                        changed = true;
                        commit_requested = true;
                    }
                });
                if structure_changed {
                    return (changed, commit_requested, true);
                }
            }
        }

        (changed, commit_requested, false)
    }

    pub fn show_timecontrol_anchors(
        ui: &mut egui::Ui,
        painter: &egui::Painter,
        timecontrol: &mut crate::keyframe::TimeControl,
        selected_point: &mut usize,
        context_menu_position: &mut Option<[f64; 2]>,
        viewport: TimeControlViewport,
        visible_y_bounds: &mut Option<TimeControlVerticalBounds>,
        vertical_bounds: TimeControlVerticalBounds,
    ) -> (bool, bool, bool) {
        let mut changed = false;
        let mut commit_requested = false;

        for point_index in 0..timecontrol.points.len() {
            let point = viewport.graph_to_screen(timecontrol.points[point_index].position);
            let handle_rect = egui::Rect::from_center_size(point, egui::Vec2::splat(18.0));
            let handle_response = ui.interact(
                handle_rect,
                ui.id().with(("timecontrol_anchor", point_index)),
                egui::Sense::click_and_drag(),
            );
            if handle_response.clicked() || handle_response.dragged() {
                *selected_point = point_index;
            }
            if handle_response.secondary_clicked() {
                *selected_point = point_index;
                *context_menu_position = handle_response
                    .interact_pointer_pos()
                    .map(|pos| viewport.screen_to_graph(pos));
            }
            if handle_response.dragged()
                && let Some(pointer_pos) = handle_response.interact_pointer_pos()
            {
                let new_position = Self::timecontrol_drag_position(
                    viewport.screen_to_graph(pointer_pos),
                    Self::timecontrol_drag_modifiers(ui),
                    None,
                );
                if Self::move_timecontrol_anchor(timecontrol, point_index, new_position) {
                    changed = true;
                }
                Self::scroll_timecontrol_y_for_drag(
                    ui,
                    visible_y_bounds,
                    viewport,
                    vertical_bounds,
                    pointer_pos,
                );
            }
            commit_requested |= handle_response.drag_stopped();
            let color = if handle_response.hovered()
                || handle_response.dragged()
                || point_index == *selected_point
            {
                GUI_COLORS.anchor_select
            } else {
                GUI_COLORS.anchor
            };
            Self::draw_timecontrol_anchor(
                painter,
                point,
                timecontrol.points[point_index].handles_separated,
                color,
            );
            let before_len = timecontrol.points.len();
            handle_response.context_menu(|ui| {
                if Self::show_timecontrol_anchor_menu(
                    ui,
                    timecontrol,
                    selected_point,
                    context_menu_position.unwrap_or_else(|| viewport.screen_to_graph(point)),
                ) {
                    changed = true;
                    commit_requested = true;
                }
            });
            if timecontrol.points.len() != before_len {
                return (changed, commit_requested, true);
            }
        }

        (changed, commit_requested, false)
    }

    pub fn draw_timecontrol_anchor(
        painter: &egui::Painter,
        center: egui::Pos2,
        separated_handles: bool,
        color: egui::Color32,
    ) {
        if separated_handles {
            let radius = 6.0;
            painter.add(egui::Shape::convex_polygon(
                vec![
                    egui::pos2(center.x, center.y - radius),
                    egui::pos2(center.x + radius, center.y),
                    egui::pos2(center.x, center.y + radius),
                    egui::pos2(center.x - radius, center.y),
                ],
                color,
                egui::Stroke::NONE,
            ));
        } else {
            painter.rect_filled(
                egui::Rect::from_center_size(center, egui::Vec2::splat(10.0)),
                2.0,
                color,
            );
        }
    }

    pub fn format_timecontrol_grid_label(value: f64) -> String {
        if value.abs() < 0.000_001 {
            "0".to_string()
        } else if (value - value.round()).abs() < 0.000_001 {
            format!("{value:.0}")
        } else {
            format!("{value:.2}")
        }
    }

    pub fn show_timecontrol_segment_mode_menu(
        ui: &mut egui::Ui,
        timecontrol: &mut crate::keyframe::TimeControl,
        segment_index: usize,
    ) -> bool {
        let mut changed = false;
        let can_change = segment_index + 1 < timecontrol.points.len();
        ui.add_enabled_ui(can_change, |ui| {
            ui.menu_button(aviutl2::config::translate("区間方式"), |ui| {
                for (mode, label) in [
                    (
                        crate::keyframe::TimeControlMode::Bezier,
                        aviutl2::config::translate("ベジェ"),
                    ),
                    (
                        crate::keyframe::TimeControlMode::Elastic,
                        aviutl2::config::translate("Elastic"),
                    ),
                    (
                        crate::keyframe::TimeControlMode::Bounce,
                        aviutl2::config::translate("Bounce"),
                    ),
                ] {
                    if ui
                        .selectable_label(
                            timecontrol.segment_mode(segment_index) == Some(mode),
                            label,
                        )
                        .clicked()
                    {
                        if timecontrol.segment_mode(segment_index) != Some(mode) {
                            timecontrol.set_segment_mode(segment_index, mode);
                            changed = true;
                        }
                        ui.close();
                    }
                }
            });
        });
        changed
    }

    pub fn show_timecontrol_segment_reverse_menu(
        ui: &mut egui::Ui,
        timecontrol: &mut crate::keyframe::TimeControl,
        segment_index: usize,
    ) -> bool {
        let mut changed = false;
        if timecontrol.segment_mode(segment_index) == Some(crate::keyframe::TimeControlMode::Bezier)
        {
            if ui.button(aviutl2::config::translate("反転")).clicked() {
                Self::reverse_timecontrol_bezier_segment(timecontrol, segment_index);
                changed = true;
                ui.close();
            }
            return changed;
        }

        let Some(mut reversed) = timecontrol.segment_reversed(segment_index) else {
            return false;
        };
        if ui
            .checkbox(&mut reversed, aviutl2::config::translate("反転"))
            .changed()
        {
            if timecontrol.segment_reversed(segment_index) != Some(reversed) {
                timecontrol.set_segment_reversed(segment_index, reversed);
                changed = true;
            }
            ui.close();
        }
        changed
    }

    pub fn reverse_timecontrol_bezier_segment(
        timecontrol: &mut crate::keyframe::TimeControl,
        segment_index: usize,
    ) {
        if timecontrol.segment_mode(segment_index) != Some(crate::keyframe::TimeControlMode::Bezier)
            || segment_index + 1 >= timecontrol.points.len()
        {
            return;
        }

        let end_index = segment_index + 1;
        let start = timecontrol.points[segment_index].position;
        let end = timecontrol.points[end_index].position;
        let start_out = timecontrol.points[segment_index]
            .out_handle
            .unwrap_or(start);
        let end_in = timecontrol.points[end_index].in_handle.unwrap_or(end);
        let new_start_out = Self::constrain_timecontrol_handle_position(
            timecontrol,
            segment_index,
            TimeControlHandleKind::Out,
            [start[0] + end[0] - end_in[0], start[1] + end[1] - end_in[1]],
        );
        let new_end_in = Self::constrain_timecontrol_handle_position(
            timecontrol,
            end_index,
            TimeControlHandleKind::In,
            [
                start[0] + end[0] - start_out[0],
                start[1] + end[1] - start_out[1],
            ],
        );

        if !timecontrol.points[segment_index].handles_separated
            && timecontrol.points[segment_index].in_handle.is_some()
        {
            let mirrored_in = Self::constrain_timecontrol_handle_position(
                timecontrol,
                segment_index,
                TimeControlHandleKind::In,
                [
                    start[0] * 2.0 - new_start_out[0],
                    start[1] * 2.0 - new_start_out[1],
                ],
            );
            if timecontrol.points[segment_index].in_handle != Some(mirrored_in) {
                timecontrol.points[segment_index].handles_separated = true;
            }
        }
        if !timecontrol.points[end_index].handles_separated
            && timecontrol.points[end_index].out_handle.is_some()
        {
            let mirrored_out = Self::constrain_timecontrol_handle_position(
                timecontrol,
                end_index,
                TimeControlHandleKind::Out,
                [end[0] * 2.0 - new_end_in[0], end[1] * 2.0 - new_end_in[1]],
            );
            if timecontrol.points[end_index].out_handle != Some(mirrored_out) {
                timecontrol.points[end_index].handles_separated = true;
            }
        }

        timecontrol.points[segment_index].out_handle = Some(new_start_out);
        timecontrol.points[end_index].in_handle = Some(new_end_in);
        Self::constrain_all_timecontrol_handles(timecontrol);
    }

    pub fn show_timecontrol_vertex(
        ui: &mut egui::Ui,
        painter: &egui::Painter,
        timecontrol: &mut crate::keyframe::TimeControl,
        segment_index: usize,
        selected_point: &mut usize,
        viewport: TimeControlViewport,
        visible_y_bounds: &mut Option<TimeControlVerticalBounds>,
        vertical_bounds: TimeControlVerticalBounds,
    ) -> (bool, bool) {
        let Some(vertex) = timecontrol.segment_vertex(segment_index) else {
            return (false, false);
        };
        let vertex_pos = viewport.graph_to_screen(vertex);
        let vertex_rect = egui::Rect::from_center_size(vertex_pos, egui::Vec2::splat(18.0));
        let response = ui.interact(
            vertex_rect,
            ui.id().with(("timecontrol_mode_vertex", segment_index)),
            egui::Sense::click_and_drag(),
        );
        if response.clicked() || response.dragged() {
            *selected_point = segment_index;
        }
        let mut changed = false;
        if response.dragged()
            && let Some(pointer_pos) = response.interact_pointer_pos()
        {
            let new_vertex = Self::timecontrol_drag_position(
                viewport.screen_to_graph(pointer_pos),
                Self::timecontrol_drag_modifiers(ui),
                Some(vertex[1]),
            );
            if Some(new_vertex) != timecontrol.segment_vertex(segment_index) {
                timecontrol.set_segment_vertex(segment_index, new_vertex);
                changed = true;
            }
            Self::scroll_timecontrol_y_for_drag(
                ui,
                visible_y_bounds,
                viewport,
                vertical_bounds,
                pointer_pos,
            );
        }
        let color = if response.hovered() || response.dragged() {
            GUI_COLORS.anchor_select
        } else {
            GUI_COLORS.anchor
        };
        Self::draw_timecontrol_anchor(painter, vertex_pos, false, color);
        (changed, response.drag_stopped())
    }

    pub fn show_timecontrol_elastic_handles(
        ui: &mut egui::Ui,
        painter: &egui::Painter,
        timecontrol: &mut crate::keyframe::TimeControl,
        segment_index: usize,
        selected_point: &mut usize,
        viewport: TimeControlViewport,
        visible_y_bounds: &mut Option<TimeControlVerticalBounds>,
        vertical_bounds: TimeControlVerticalBounds,
    ) -> (bool, bool) {
        if segment_index + 1 >= timecontrol.points.len() {
            return (false, false);
        }
        let start = timecontrol.points[segment_index].position;
        let end = timecontrol.points[segment_index + 1].position;
        let Some(elastic) = timecontrol.segment_elastic(segment_index) else {
            return (false, false);
        };
        let display_local = |point: [f64; 2]| {
            if elastic.reversed {
                [1.0 - point[0], 1.0 - point[1]]
            } else {
                point
            }
        };

        let amp_y = elastic.amp_handle()[1];
        let amp_handles = [
            Self::timecontrol_local_to_graph(start, end, display_local([0.0, amp_y])),
            Self::timecontrol_local_to_graph(start, end, display_local([1.0, amp_y])),
        ];
        let freq_decay_handle = Self::timecontrol_local_to_graph(
            start,
            end,
            display_local(elastic.freq_decay_handle()),
        );
        let control_stroke = egui::Stroke::new(1.0, GUI_COLORS.anchor_line);
        painter.line_segment(
            [
                viewport.graph_to_screen(amp_handles[0]),
                viewport.graph_to_screen(amp_handles[1]),
            ],
            control_stroke,
        );
        painter.line_segment(
            [
                viewport.graph_to_screen(Self::timecontrol_local_to_graph(
                    start,
                    end,
                    display_local([elastic.freq_decay_handle()[0], 1.0]),
                )),
                viewport.graph_to_screen(freq_decay_handle),
            ],
            control_stroke,
        );

        let mut changed = false;
        let mut commit_requested = false;
        for (index, handle) in amp_handles.into_iter().enumerate() {
            let handle_pos = viewport.graph_to_screen(handle);
            let handle_rect = egui::Rect::from_center_size(handle_pos, egui::Vec2::splat(18.0));
            let response = ui.interact(
                handle_rect,
                ui.id()
                    .with(("timecontrol_elastic_amp", segment_index, index)),
                egui::Sense::click_and_drag(),
            );
            if response.clicked() || response.dragged() {
                *selected_point = segment_index;
            }
            if response.dragged()
                && let Some(pointer_pos) = response.interact_pointer_pos()
            {
                let position = Self::timecontrol_drag_position(
                    viewport.screen_to_graph(pointer_pos),
                    Self::timecontrol_drag_modifiers(ui),
                    None,
                );
                let mut position = Self::timecontrol_graph_to_local(start, end, position);
                if timecontrol
                    .segment_elastic(segment_index)
                    .is_some_and(|elastic| elastic.reversed)
                {
                    position = [1.0 - position[0], 1.0 - position[1]];
                }
                let elastic = timecontrol.segment_elastic_mut(segment_index).unwrap();
                let old_amplitude = elastic.amplitude;
                elastic.set_amp_handle_y(position[1]);
                changed |= (elastic.amplitude - old_amplitude).abs() > f64::EPSILON;
                Self::scroll_timecontrol_y_for_drag(
                    ui,
                    visible_y_bounds,
                    viewport,
                    vertical_bounds,
                    pointer_pos,
                );
            }
            commit_requested |= response.drag_stopped();
            let color = if response.hovered() || response.dragged() {
                GUI_COLORS.anchor_select
            } else {
                GUI_COLORS.anchor
            };
            Self::draw_timecontrol_anchor(painter, handle_pos, false, color);
        }

        let handle_pos = viewport.graph_to_screen(freq_decay_handle);
        let handle_rect = egui::Rect::from_center_size(handle_pos, egui::Vec2::splat(18.0));
        let response = ui.interact(
            handle_rect,
            ui.id()
                .with(("timecontrol_elastic_freq_decay", segment_index)),
            egui::Sense::click_and_drag(),
        );
        if response.clicked() || response.dragged() {
            *selected_point = segment_index;
        }
        if response.dragged()
            && let Some(pointer_pos) = response.interact_pointer_pos()
        {
            let position = Self::timecontrol_drag_position(
                viewport.screen_to_graph(pointer_pos),
                Self::timecontrol_drag_modifiers(ui),
                None,
            );
            let mut position = Self::timecontrol_graph_to_local(start, end, position);
            if timecontrol
                .segment_elastic(segment_index)
                .is_some_and(|elastic| elastic.reversed)
            {
                position = [1.0 - position[0], 1.0 - position[1]];
            }
            let elastic = timecontrol.segment_elastic_mut(segment_index).unwrap();
            let old_frequency = elastic.frequency;
            let old_decay = elastic.decay;
            elastic.set_freq_decay_handle(position);
            changed |= (elastic.frequency - old_frequency).abs() > f64::EPSILON
                || (elastic.decay - old_decay).abs() > f64::EPSILON;
            Self::scroll_timecontrol_y_for_drag(
                ui,
                visible_y_bounds,
                viewport,
                vertical_bounds,
                pointer_pos,
            );
        }
        commit_requested |= response.drag_stopped();
        let color = if response.hovered() || response.dragged() {
            GUI_COLORS.anchor_select
        } else {
            GUI_COLORS.anchor
        };
        Self::draw_timecontrol_anchor(painter, handle_pos, false, color);

        (changed, commit_requested)
    }

    pub fn show_timecontrol_anchor_menu(
        ui: &mut egui::Ui,
        timecontrol: &mut crate::keyframe::TimeControl,
        selected_point: &mut usize,
        add_point_position: [f64; 2],
    ) -> bool {
        let mut changed = false;
        *selected_point = (*selected_point).min(timecontrol.points.len().saturating_sub(1));

        if ui
            .button(aviutl2::config::translate("中継点追加"))
            .clicked()
        {
            *selected_point = Self::insert_timecontrol_point(timecontrol, add_point_position);
            changed = true;
            ui.close();
        }

        let can_remove = *selected_point != 0 && *selected_point + 1 < timecontrol.points.len();
        if ui
            .add_enabled(
                can_remove,
                egui::Button::new(aviutl2::config::translate("中継点削除")),
            )
            .clicked()
        {
            Self::remove_timecontrol_point(timecontrol, selected_point);
            changed = true;
            ui.close();
        }

        ui.separator();
        changed |= Self::show_timecontrol_segment_mode_menu(ui, timecontrol, *selected_point);
        changed |= Self::show_timecontrol_segment_reverse_menu(ui, timecontrol, *selected_point);
        ui.separator();
        changed |= Self::show_timecontrol_handle_menu(ui, timecontrol, selected_point);

        changed
    }

    pub fn insert_timecontrol_point(
        timecontrol: &mut crate::keyframe::TimeControl,
        position: [f64; 2],
    ) -> usize {
        let x = position[0].clamp(0.0, 1.0);
        let after_index = timecontrol
            .points
            .windows(2)
            .position(|points| x <= points[1].position[0])
            .unwrap_or(timecontrol.points.len().saturating_sub(2));
        let new_index = timecontrol.insert_midpoint(after_index);
        timecontrol.points[new_index].position = [
            x.clamp(
                timecontrol.points[new_index - 1].position[0]
                    + Self::TIMECONTROL_MIN_ANCHOR_DISTANCE,
                timecontrol.points[new_index + 1].position[0]
                    - Self::TIMECONTROL_MIN_ANCHOR_DISTANCE,
            ),
            position[1],
        ];
        Self::reset_timecontrol_handles(timecontrol, new_index);
        Self::constrain_all_timecontrol_handles(timecontrol);
        new_index
    }

    pub fn remove_timecontrol_point(
        timecontrol: &mut crate::keyframe::TimeControl,
        selected_point: &mut usize,
    ) {
        let remove_index = *selected_point;
        timecontrol.remove_midpoint(remove_index);
        *selected_point = remove_index
            .saturating_sub(1)
            .min(timecontrol.points.len().saturating_sub(1));
        Self::constrain_all_timecontrol_handles(timecontrol);
    }

    pub fn show_timecontrol_handle_menu(
        ui: &mut egui::Ui,
        timecontrol: &mut crate::keyframe::TimeControl,
        selected_point: &mut usize,
    ) -> bool {
        let mut changed = false;
        *selected_point = (*selected_point).min(timecontrol.points.len().saturating_sub(1));

        let has_both_handles = timecontrol.points[*selected_point].in_handle.is_some()
            && timecontrol.points[*selected_point].out_handle.is_some();
        let label = if timecontrol.points[*selected_point].handles_separated {
            aviutl2::config::translate("ハンドル連動")
        } else {
            aviutl2::config::translate("ハンドル分離")
        };
        if ui
            .add_enabled(has_both_handles, egui::Button::new(label))
            .clicked()
        {
            timecontrol.points[*selected_point].handles_separated =
                !timecontrol.points[*selected_point].handles_separated;
            if !timecontrol.points[*selected_point].handles_separated {
                Self::mirror_timecontrol_handle(timecontrol, *selected_point, true);
            }
            changed = true;
            ui.close();
        }

        let has_any_handle = timecontrol.points[*selected_point].in_handle.is_some()
            || timecontrol.points[*selected_point].out_handle.is_some();
        if ui
            .add_enabled(
                has_any_handle,
                egui::Button::new(aviutl2::config::translate("ハンドルリセット")),
            )
            .clicked()
        {
            Self::reset_timecontrol_handles(timecontrol, *selected_point);
            changed = true;
            ui.close();
        }

        changed
    }

    pub fn clamped_timecontrol_anchor_position(
        timecontrol: &crate::keyframe::TimeControl,
        point_index: usize,
        position: [f64; 2],
    ) -> [f64; 2] {
        if point_index == 0 {
            return [0.0, 0.0];
        }
        if point_index + 1 == timecontrol.points.len() {
            return [1.0, 1.0];
        }

        let min_x = point_index as f64 * Self::TIMECONTROL_MIN_ANCHOR_DISTANCE;
        let max_x = 1.0
            - (timecontrol.points.len() - 1 - point_index) as f64
                * Self::TIMECONTROL_MIN_ANCHOR_DISTANCE;
        [position[0].clamp(min_x, max_x), position[1]]
    }

    pub const TIMECONTROL_MIN_ANCHOR_DISTANCE: f64 = 0.001;

    pub fn snap_timecontrol_position(position: [f64; 2], step: f64) -> [f64; 2] {
        [
            (position[0] / step).round() * step,
            (position[1] / step).round() * step,
        ]
    }

    pub fn timecontrol_drag_modifiers(ui: &egui::Ui) -> TimeControlDragModifiers {
        ui.input(|input| TimeControlDragModifiers {
            shift: input.modifiers.shift,
            alt: input.modifiers.alt,
        })
    }

    pub fn timecontrol_drag_position(
        mut position: [f64; 2],
        modifiers: TimeControlDragModifiers,
        horizontal_snap_y: Option<f64>,
    ) -> [f64; 2] {
        if modifiers.alt {
            position = Self::snap_timecontrol_position(position, 0.125);
        }
        if let Some(y) = horizontal_snap_y
            && modifiers.shift
        {
            position[1] = y;
        }
        position
    }

    pub fn timecontrol_local_to_graph(
        start: [f64; 2],
        end: [f64; 2],
        position: [f64; 2],
    ) -> [f64; 2] {
        [
            start[0] + (end[0] - start[0]) * position[0],
            start[1] + (end[1] - start[1]) * position[1],
        ]
    }

    pub fn timecontrol_graph_to_local(
        start: [f64; 2],
        end: [f64; 2],
        position: [f64; 2],
    ) -> [f64; 2] {
        let dx = end[0] - start[0];
        let dy = end[1] - start[1];
        [
            if dx.abs() < f64::EPSILON {
                0.0
            } else {
                (position[0] - start[0]) / dx
            },
            if dy.abs() < f64::EPSILON {
                position[1] - start[1] + 1.0
            } else {
                (position[1] - start[1]) / dy
            },
        ]
    }

    pub fn scroll_timecontrol_y_for_drag(
        ui: &egui::Ui,
        visible_y_bounds: &mut Option<TimeControlVerticalBounds>,
        viewport: TimeControlViewport,
        vertical_bounds: TimeControlVerticalBounds,
        pointer_pos: egui::Pos2,
    ) {
        if viewport.rect.height() <= f32::EPSILON {
            return;
        }

        let overflow = if pointer_pos.y < viewport.rect.top() {
            viewport.rect.top() - pointer_pos.y
        } else if pointer_pos.y > viewport.rect.bottom() {
            viewport.rect.bottom() - pointer_pos.y
        } else {
            return;
        };

        let visible_y_range = viewport.max_y - viewport.min_y;
        let scroll_y = overflow as f64 / viewport.rect.height() as f64 * visible_y_range;
        let max_scroll_y = visible_y_range * 0.1;
        let scroll_y = scroll_y.clamp(-max_scroll_y, max_scroll_y);
        *visible_y_bounds = Some(
            TimeControlVerticalBounds {
                min_y: viewport.min_y,
                max_y: viewport.max_y,
            }
            .translate(scroll_y)
            .clamp_to_content(vertical_bounds),
        );
        ui.ctx().request_repaint();
    }

    pub fn move_timecontrol_anchor(
        timecontrol: &mut crate::keyframe::TimeControl,
        point_index: usize,
        position: [f64; 2],
    ) -> bool {
        let new_position =
            Self::clamped_timecontrol_anchor_position(timecontrol, point_index, position);
        if timecontrol.points[point_index].position == new_position {
            return false;
        }

        let old_position = timecontrol.points[point_index].position;
        let delta = [
            new_position[0] - old_position[0],
            new_position[1] - old_position[1],
        ];

        timecontrol.points[point_index].position = new_position;

        for index in (1..point_index).rev() {
            let max_x =
                timecontrol.points[index + 1].position[0] - Self::TIMECONTROL_MIN_ANCHOR_DISTANCE;
            if timecontrol.points[index].position[0] <= max_x {
                break;
            }
            timecontrol.points[index].position[0] = max_x;
        }

        for index in point_index + 1..timecontrol.points.len().saturating_sub(1) {
            let min_x =
                timecontrol.points[index - 1].position[0] + Self::TIMECONTROL_MIN_ANCHOR_DISTANCE;
            if timecontrol.points[index].position[0] >= min_x {
                break;
            }
            timecontrol.points[index].position[0] = min_x;
        }

        if let Some(in_handle) = timecontrol.points[point_index].in_handle {
            timecontrol.points[point_index].in_handle =
                Some(Self::constrain_timecontrol_handle_position(
                    timecontrol,
                    point_index,
                    TimeControlHandleKind::In,
                    [in_handle[0] + delta[0], in_handle[1] + delta[1]],
                ));
        }
        if let Some(out_handle) = timecontrol.points[point_index].out_handle {
            timecontrol.points[point_index].out_handle =
                Some(Self::constrain_timecontrol_handle_position(
                    timecontrol,
                    point_index,
                    TimeControlHandleKind::Out,
                    [out_handle[0] + delta[0], out_handle[1] + delta[1]],
                ));
        }
        Self::constrain_all_timecontrol_handles(timecontrol);
        true
    }

    pub fn set_timecontrol_handle(
        timecontrol: &mut crate::keyframe::TimeControl,
        point_index: usize,
        handle_kind: TimeControlHandleKind,
        point: [f64; 2],
    ) {
        match handle_kind {
            TimeControlHandleKind::In => {
                timecontrol.points[point_index].in_handle = Some(point);
                if !timecontrol.points[point_index].handles_separated {
                    Self::mirror_timecontrol_handle(timecontrol, point_index, false);
                }
            }
            TimeControlHandleKind::Out => {
                timecontrol.points[point_index].out_handle = Some(point);
                if !timecontrol.points[point_index].handles_separated {
                    Self::mirror_timecontrol_handle(timecontrol, point_index, true);
                }
            }
        }
    }

    pub fn constrain_all_timecontrol_handles(timecontrol: &mut crate::keyframe::TimeControl) {
        for point_index in 0..timecontrol.points.len() {
            if let Some(in_handle) = timecontrol.points[point_index].in_handle {
                timecontrol.points[point_index].in_handle =
                    Some(Self::constrain_timecontrol_handle_position(
                        timecontrol,
                        point_index,
                        TimeControlHandleKind::In,
                        in_handle,
                    ));
            }
            if let Some(out_handle) = timecontrol.points[point_index].out_handle {
                timecontrol.points[point_index].out_handle =
                    Some(Self::constrain_timecontrol_handle_position(
                        timecontrol,
                        point_index,
                        TimeControlHandleKind::Out,
                        out_handle,
                    ));
            }
        }
    }

    pub fn constrain_timecontrol_handle_position(
        timecontrol: &crate::keyframe::TimeControl,
        point_index: usize,
        handle_kind: TimeControlHandleKind,
        handle: [f64; 2],
    ) -> [f64; 2] {
        let anchor = timecontrol.points[point_index].position;
        let x =
            Self::clamped_timecontrol_handle_x(timecontrol, point_index, handle_kind, handle[0]);
        if (x - handle[0]).abs() < f64::EPSILON {
            return handle;
        }

        let delta = [handle[0] - anchor[0], handle[1] - anchor[1]];
        if (x - anchor[0]).abs() < f64::EPSILON {
            return [x, handle[1]];
        }
        if delta[0].abs() < f64::EPSILON {
            return [x, handle[1]];
        }

        let scale = (x - anchor[0]) / delta[0];
        if !scale.is_finite() {
            return [x, handle[1]];
        }

        [x, anchor[1] + delta[1] * scale]
    }

    pub fn clamped_timecontrol_handle_x(
        timecontrol: &crate::keyframe::TimeControl,
        point_index: usize,
        handle_kind: TimeControlHandleKind,
        x: f64,
    ) -> f64 {
        match handle_kind {
            TimeControlHandleKind::In => {
                let min_x = point_index
                    .checked_sub(1)
                    .map(|index| timecontrol.points[index].position[0])
                    .unwrap_or(timecontrol.points[point_index].position[0]);
                Self::clamp_ordered(x, min_x, timecontrol.points[point_index].position[0])
            }
            TimeControlHandleKind::Out => {
                let max_x = timecontrol
                    .points
                    .get(point_index + 1)
                    .map(|point| point.position[0])
                    .unwrap_or(timecontrol.points[point_index].position[0]);
                Self::clamp_ordered(x, timecontrol.points[point_index].position[0], max_x)
            }
        }
    }

    pub fn clamp_ordered(value: f64, min: f64, max: f64) -> f64 {
        if min <= max {
            value.clamp(min, max)
        } else {
            value.clamp(max, min)
        }
    }

    pub fn mirror_timecontrol_handle(
        timecontrol: &mut crate::keyframe::TimeControl,
        point_index: usize,
        moved_out_handle: bool,
    ) {
        if timecontrol.points[point_index].handles_separated {
            return;
        }
        let position = timecontrol.points[point_index].position;
        if moved_out_handle {
            let Some(out_handle) = timecontrol.points[point_index].out_handle else {
                return;
            };
            let new_in_handle = Self::constrain_timecontrol_handle_position(
                timecontrol,
                point_index,
                TimeControlHandleKind::In,
                [
                    position[0] * 2.0 - out_handle[0],
                    position[1] * 2.0 - out_handle[1],
                ],
            );
            if let Some(in_handle) = &mut timecontrol.points[point_index].in_handle {
                *in_handle = new_in_handle;
            }
        } else {
            let Some(in_handle) = timecontrol.points[point_index].in_handle else {
                return;
            };
            let new_out_handle = Self::constrain_timecontrol_handle_position(
                timecontrol,
                point_index,
                TimeControlHandleKind::Out,
                [
                    position[0] * 2.0 - in_handle[0],
                    position[1] * 2.0 - in_handle[1],
                ],
            );
            if let Some(out_handle) = &mut timecontrol.points[point_index].out_handle {
                *out_handle = new_out_handle;
            }
        }
    }

    pub fn reset_timecontrol_handles(
        timecontrol: &mut crate::keyframe::TimeControl,
        point_index: usize,
    ) {
        let position = timecontrol.points[point_index].position;
        let prev = point_index
            .checked_sub(1)
            .map(|prev_index| timecontrol.points[prev_index].position);
        let next = timecontrol
            .points
            .get(point_index + 1)
            .map(|next_point| next_point.position);
        let tangent = match (prev, next) {
            (Some(prev), Some(next)) => [next[0] - prev[0], next[1] - prev[1]],
            (Some(prev), None) => [position[0] - prev[0], position[1] - prev[1]],
            (None, Some(next)) => [next[0] - position[0], next[1] - position[1]],
            (None, None) => [1.0, 0.0],
        };
        timecontrol.points[point_index].in_handle = prev.map(|prev| {
            let x = position[0] + (prev[0] - position[0]) / 3.0;
            Self::timecontrol_point_on_tangent(position, tangent, x)
        });
        timecontrol.points[point_index].out_handle = next.map(|next| {
            let x = position[0] + (next[0] - position[0]) / 3.0;
            Self::timecontrol_point_on_tangent(position, tangent, x)
        });
        timecontrol.points[point_index].handles_separated = false;
    }

    pub fn timecontrol_point_on_tangent(origin: [f64; 2], tangent: [f64; 2], x: f64) -> [f64; 2] {
        if tangent[0].abs() < f64::EPSILON {
            return [x, origin[1]];
        }
        let scale = (x - origin[0]) / tangent[0];
        [x, origin[1] + tangent[1] * scale]
    }

    pub fn update_timecontrol_editor_target(
        &mut self,
        read: &aviutl2::generic::ReadSection,
    ) -> aviutl2::common::AnyResult<()> {
        let Some(target) = &self.timecontrol_editor else {
            return Ok(());
        };
        if target.dirty {
            return Ok(());
        }
        if !read.get_focused_object()?.is_some_and(|focused| {
            read.get_effects(focused)
                .is_ok_and(|effects| effects.contains(&target.effect))
        }) {
            tracing::info!("Time control editor target effect is not focused, closing editor");
            self.timecontrol_editor = None;
            return Ok(());
        }
        let params = match crate::KeyframeTrackParams::parse(
            read,
            target.effect,
            &target.track_names[0],
        ) {
            Some(params) => params,
            None => {
                tracing::error!(
                    "Failed to parse keyframe track params for time control editor, closing editor"
                );
                self.timecontrol_editor = None;
                return Ok(());
            }
        };
        let keyframes = crate::KEYFRAMES
            .get(&params)
            .context("Failed to get keyframes for time control editor")?
            .clone();
        self.timecontrol_editor.as_mut().unwrap().timecontrol =
            match keyframes.keyframes[target.keyframe_index] {
                crate::keyframe::Keyframe::Easing(ref easing) => easing.timecontrol.clone(),
                _ => anyhow::bail!("Target keyframe is not easing"),
            };
        Ok(())
    }
}
