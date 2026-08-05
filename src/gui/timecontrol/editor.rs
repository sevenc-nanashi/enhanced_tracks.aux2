use super::*;

impl KeyframesGui {
    pub fn show_timecontrol_bezier_editor(
        ui: &mut egui::Ui,
        timecontrol: &mut crate::keyframe::TimeControl,
        selected_point: &mut usize,
        context_menu_position: &mut Option<[f64; 2]>,
        clipboard: &mut Option<crate::keyframe::TimeControl>,
        visible_y_bounds: &mut Option<TimeControlVerticalBounds>,
        drag_scroll_y_bounds: &mut Option<TimeControlVerticalBounds>,
    ) -> (bool, bool) {
        let mut changed = false;
        let mut commit_requested = false;
        *selected_point = (*selected_point).min(timecontrol.points.len().saturating_sub(1));

        let available_size = ui.available_size();
        if available_size.x <= f32::EPSILON || available_size.y <= f32::EPSILON {
            return (false, false);
        }
        let (response, painter) =
            ui.allocate_painter(available_size, egui::Sense::click_and_drag());
        let mut rect = response.rect.shrink(response.rect.height() * 0.1);
        rect.set_left(rect.left() + rect.height() * 0.1);
        let (content_min_y, content_max_y) = Self::timecontrol_editor_vertical_bounds(timecontrol);
        let actual_vertical_bounds = TimeControlVerticalBounds {
            min_y: content_min_y,
            max_y: content_max_y,
        };
        let vertical_bounds = if ui.ctx().dragged_id().is_some() {
            let drag_bounds = drag_scroll_y_bounds.get_or_insert(actual_vertical_bounds);
            *drag_bounds = drag_bounds.union(actual_vertical_bounds);
            *drag_bounds
        } else {
            *drag_scroll_y_bounds = None;
            actual_vertical_bounds
        };
        let mut current_visible_y_bounds = visible_y_bounds
            .unwrap_or(vertical_bounds)
            .clamp_to_content(vertical_bounds);
        let pointer_pos = ui.input(|i| i.pointer.hover_pos());
        if pointer_pos.is_some_and(|pos| response.rect.contains(pos)) {
            let (scroll_delta, zoom_delta, ctrl, pointer_pos) = ui.input(|i| {
                (
                    i.smooth_scroll_delta().y as f64,
                    i.zoom_delta() as f64,
                    i.modifiers.ctrl,
                    i.pointer.hover_pos(),
                )
            });
            if ctrl {
                let zoom_factor = if (zoom_delta - 1.0).abs() > f64::EPSILON {
                    zoom_delta
                } else if scroll_delta.abs() > f64::EPSILON {
                    (scroll_delta * 0.01).exp()
                } else {
                    1.0
                };
                if (zoom_factor - 1.0).abs() > f64::EPSILON {
                    let range = current_visible_y_bounds.y_range() / zoom_factor;
                    current_visible_y_bounds = if let Some(pointer_pos) = pointer_pos {
                        let anchor_ratio = ((rect.bottom() - pointer_pos.y) / rect.height())
                            .clamp(0.0, 1.0) as f64;
                        let anchor_y = current_visible_y_bounds.min_y
                            + current_visible_y_bounds.y_range() * anchor_ratio;
                        TimeControlVerticalBounds::with_anchor_and_range(
                            anchor_y,
                            anchor_ratio,
                            range,
                        )
                    } else {
                        TimeControlVerticalBounds::with_center_and_range(
                            current_visible_y_bounds.center(),
                            range,
                        )
                    }
                    .clamp_to_content(vertical_bounds);
                }
            } else if scroll_delta.abs() > f64::EPSILON && rect.height() > f32::EPSILON {
                let scroll_y =
                    scroll_delta / rect.height() as f64 * current_visible_y_bounds.y_range();
                current_visible_y_bounds = current_visible_y_bounds
                    .translate(scroll_y)
                    .clamp_to_content(vertical_bounds);
            }
        }
        *visible_y_bounds = Some(current_visible_y_bounds);
        let viewport = TimeControlViewport {
            rect,
            min_y: current_visible_y_bounds.min_y,
            max_y: current_visible_y_bounds.max_y,
        };

        if response.secondary_clicked() {
            *context_menu_position = response
                .interact_pointer_pos()
                .map(|pos| viewport.screen_to_graph(pos));
        }
        let (shortcut_changed, shortcut_commit_requested) =
            Self::handle_timecontrol_editor_shortcuts(
                ui,
                &response,
                timecontrol,
                selected_point,
                context_menu_position,
                clipboard,
                visible_y_bounds,
                drag_scroll_y_bounds,
                viewport,
            );
        changed |= shortcut_changed;
        commit_requested |= shortcut_commit_requested;
        response.context_menu(|ui| {
            if Self::show_timecontrol_segment_mode_menu(ui, timecontrol, *selected_point) {
                changed = true;
                commit_requested = true;
            }
            if Self::show_timecontrol_segment_reverse_menu(ui, timecontrol, *selected_point) {
                changed = true;
                commit_requested = true;
            }
            if ui
                .add(
                    egui::Button::new(aviutl2::config::translate("コピー"))
                        .shortcut_text(ui.format_shortcut(&COPY_SHORTCUT)),
                )
                .clicked()
            {
                *clipboard = Some(timecontrol.clone());
                ui.close();
            }
            let can_paste = clipboard.is_some();
            if ui
                .add_enabled(
                    can_paste,
                    egui::Button::new(aviutl2::config::translate("貼り付け"))
                        .shortcut_text(ui.format_shortcut(&PASTE_SHORTCUT)),
                )
                .clicked()
            {
                let Some(copied) = clipboard.clone() else {
                    unreachable!("貼り付け可能な場合はクリップボードに値があるはず");
                };
                *timecontrol = copied;
                *selected_point = 0;
                *context_menu_position = None;
                *visible_y_bounds = None;
                *drag_scroll_y_bounds = None;
                changed = true;
                commit_requested = true;
                ui.close();
            }
            if ui
                .add(
                    egui::Button::new(aviutl2::config::translate("中継点追加"))
                        .shortcut_text(ui.format_shortcut(&ADD_POINT_SHORTCUT)),
                )
                .clicked()
            {
                let new_point = Self::insert_timecontrol_point(
                    timecontrol,
                    context_menu_position.unwrap_or([0.5, 0.5]),
                );
                if let Some(new_point) = new_point {
                    *selected_point = new_point;
                    changed = true;
                    commit_requested = true;
                }
                ui.close();
            }
        });
        if response.double_clicked()
            && let Some(pointer_pos) = response.interact_pointer_pos()
            && !Self::is_timecontrol_anchor_near_pointer(timecontrol, viewport, pointer_pos)
            && let Some(position) =
                Self::timecontrol_curve_position_near_pointer(timecontrol, viewport, pointer_pos)
        {
            if let Some(new_point) = Self::insert_timecontrol_point(timecontrol, position) {
                *selected_point = new_point;
                changed = true;
                commit_requested = true;
            }
        }

        Self::draw_timecontrol_grid(&painter, response.rect, viewport);
        Self::draw_timecontrol_curve(&painter, timecontrol, viewport, false);
        Self::draw_timecontrol_control_lines(&painter, timecontrol, viewport);

        let (anchor_changed, anchor_commit_requested, structure_changed) =
            Self::show_timecontrol_anchors(
                ui,
                &painter,
                timecontrol,
                selected_point,
                context_menu_position,
                viewport,
                visible_y_bounds,
                vertical_bounds,
            );
        changed |= anchor_changed;
        commit_requested |= anchor_commit_requested;
        if structure_changed {
            return (changed, commit_requested);
        }

        for segment_index in 0..timecontrol.points.len().saturating_sub(1) {
            if matches!(
                timecontrol.segment_mode(segment_index),
                Some(crate::keyframe::TimeControlMode::Elastic)
            ) {
                let (elastic_changed, elastic_commit_requested) =
                    Self::show_timecontrol_elastic_handles(
                        ui,
                        &painter,
                        timecontrol,
                        segment_index,
                        selected_point,
                        viewport,
                        visible_y_bounds,
                        vertical_bounds,
                    );
                changed |= elastic_changed;
                commit_requested |= elastic_commit_requested;
            } else if matches!(
                timecontrol.segment_mode(segment_index),
                Some(crate::keyframe::TimeControlMode::Bounce)
            ) {
                let (vertex_changed, vertex_commit_requested) = Self::show_timecontrol_vertex(
                    ui,
                    &painter,
                    timecontrol,
                    segment_index,
                    selected_point,
                    viewport,
                    visible_y_bounds,
                    vertical_bounds,
                );
                changed |= vertex_changed;
                commit_requested |= vertex_commit_requested;
            }
        }

        let (handle_changed, handle_commit_requested, structure_changed) =
            Self::show_timecontrol_handles(
                ui,
                &painter,
                timecontrol,
                selected_point,
                context_menu_position,
                viewport,
                visible_y_bounds,
                vertical_bounds,
            );
        changed |= handle_changed;
        commit_requested |= handle_commit_requested;
        if structure_changed {
            return (changed, commit_requested);
        }

        (changed, commit_requested)
    }

    fn handle_timecontrol_editor_shortcuts(
        ui: &mut egui::Ui,
        response: &egui::Response,
        timecontrol: &mut crate::keyframe::TimeControl,
        selected_point: &mut usize,
        context_menu_position: &mut Option<[f64; 2]>,
        clipboard: &mut Option<crate::keyframe::TimeControl>,
        visible_y_bounds: &mut Option<TimeControlVerticalBounds>,
        drag_scroll_y_bounds: &mut Option<TimeControlVerticalBounds>,
        viewport: TimeControlViewport,
    ) -> (bool, bool) {
        if ui.input_mut(|input| input.consume_shortcut(&COPY_SHORTCUT)) {
            *clipboard = Some(timecontrol.clone());
            egui::Popup::close_all(ui.ctx());
        }

        let can_paste = clipboard.is_some();
        if can_paste && ui.input_mut(|input| input.consume_shortcut(&PASTE_SHORTCUT)) {
            let Some(copied) = clipboard.clone() else {
                unreachable!("貼り付け可能な場合はクリップボードに値があるはず");
            };
            *timecontrol = copied;
            *selected_point = 0;
            *context_menu_position = None;
            *visible_y_bounds = None;
            *drag_scroll_y_bounds = None;
            egui::Popup::close_all(ui.ctx());
            return (true, true);
        }

        let can_remove = *selected_point != 0 && *selected_point + 1 < timecontrol.points.len();
        if can_remove && ui.input_mut(|input| input.consume_shortcut(&REMOVE_POINT_SHORTCUT)) {
            Self::remove_timecontrol_point(timecontrol, selected_point);
            egui::Popup::close_all(ui.ctx());
            return (true, true);
        }

        let has_both_handles = timecontrol.points[*selected_point].in_handle.is_some()
            && timecontrol.points[*selected_point].out_handle.is_some();
        if has_both_handles
            && ui.input_mut(|input| input.consume_shortcut(&SEPARATE_HANDLES_SHORTCUT))
        {
            let changed = Self::toggle_timecontrol_handle_separation(timecontrol, *selected_point);
            if changed {
                egui::Popup::close_all(ui.ctx());
            }
            return (changed, changed);
        }

        let pointer_position = ui
            .input(|input| input.pointer.hover_pos())
            .filter(|pointer_pos| response.rect.contains(*pointer_pos))
            .map(|pointer_pos| viewport.screen_to_graph(pointer_pos));
        let add_point_position = if egui::Popup::is_any_open(ui.ctx()) {
            *context_menu_position
        } else {
            pointer_position
        };
        if let Some(add_point_position) = add_point_position
            && ui.input_mut(|input| {
                let is_initial_press = input.events.iter().any(|event| {
                    matches!(
                        event,
                        egui::Event::Key {
                            key,
                            pressed: true,
                            repeat: false,
                            modifiers,
                            ..
                        } if *key == ADD_POINT_SHORTCUT.logical_key
                            && modifiers.matches_logically(ADD_POINT_SHORTCUT.modifiers)
                    )
                });
                input.consume_shortcut(&ADD_POINT_SHORTCUT) && is_initial_press
            })
        {
            if let Some(new_point) = Self::insert_timecontrol_point(timecontrol, add_point_position)
            {
                *selected_point = new_point;
                egui::Popup::close_all(ui.ctx());
                return (true, true);
            }
        }

        let can_change_segment = *selected_point + 1 < timecontrol.points.len();
        if !can_change_segment {
            return (false, false);
        }

        let mode = [
            (
                &BEZIER_SEGMENT_MODE_MENU_SHORTCUT,
                crate::keyframe::TimeControlMode::Bezier,
            ),
            (
                &ELASTIC_SEGMENT_MODE_MENU_SHORTCUT,
                crate::keyframe::TimeControlMode::Elastic,
            ),
            (
                &BOUNCE_SEGMENT_MODE_MENU_SHORTCUT,
                crate::keyframe::TimeControlMode::Bounce,
            ),
        ]
        .into_iter()
        .find_map(|(shortcut, mode)| {
            ui.input_mut(|input| input.consume_shortcut(shortcut))
                .then_some(mode)
        });
        if let Some(mode) = mode {
            let changed = timecontrol.segment_mode(*selected_point) != Some(mode);
            if changed {
                timecontrol.set_segment_mode(*selected_point, mode);
            }
            egui::Popup::close_all(ui.ctx());
            return (changed, changed);
        }

        if ui.input_mut(|input| input.consume_shortcut(&REVERSE_SHORTCUT)) {
            let changed = Self::reverse_timecontrol_segment(timecontrol, *selected_point);
            if changed {
                egui::Popup::close_all(ui.ctx());
            }
            return (changed, changed);
        }

        (false, false)
    }

    pub fn timecontrol_editor_vertical_bounds(
        timecontrol: &crate::keyframe::TimeControl,
    ) -> (f64, f64) {
        let mut min_y = 0.0_f64;
        let mut max_y = 1.0_f64;
        for segment_index in 0..timecontrol.points.len().saturating_sub(1) {
            let start = timecontrol.points[segment_index].position;
            let end = timecontrol.points[segment_index + 1].position;
            let (segment_min_y, segment_max_y) =
                match timecontrol.points[segment_index].outgoing.as_ref() {
                    Some(crate::keyframe::TimeControlSegment::Elastic(elastic)) => {
                        let local_min_y = if elastic.reversed { -1.0 } else { 0.0 };
                        let local_max_y = if elastic.reversed { 1.0 } else { 2.0 };
                        let elastic_min_y = start[1] + (end[1] - start[1]) * local_min_y;
                        let elastic_max_y = start[1] + (end[1] - start[1]) * local_max_y;
                        (
                            elastic_min_y.min(elastic_max_y),
                            elastic_min_y.max(elastic_max_y),
                        )
                    }
                    Some(crate::keyframe::TimeControlSegment::Bounce(_)) => {
                        (start[1].min(end[1]), start[1].max(end[1]))
                    }
                    _ => {
                        let mut segment_min_y = start[1].min(end[1]);
                        let mut segment_max_y = start[1].max(end[1]);
                        for position in [
                            timecontrol.points[segment_index].out_handle,
                            timecontrol.points[segment_index + 1].in_handle,
                        ]
                        .into_iter()
                        .flatten()
                        {
                            segment_min_y = segment_min_y.min(position[1]);
                            segment_max_y = segment_max_y.max(position[1]);
                        }
                        (segment_min_y, segment_max_y)
                    }
                };
            min_y = min_y.min(segment_min_y);
            max_y = max_y.max(segment_max_y);
        }
        (min_y, max_y)
    }

    pub fn timecontrol_curve_position_near_pointer(
        timecontrol: &crate::keyframe::TimeControl,
        viewport: TimeControlViewport,
        pointer_pos: egui::Pos2,
    ) -> Option<[f64; 2]> {
        const HIT_DISTANCE: f32 = 8.0;

        let points = timecontrol.curve_sampled_points(192);
        let mut nearest = None;
        for points in points.windows(2) {
            let start = viewport.graph_to_screen(points[0]);
            let end = viewport.graph_to_screen(points[1]);
            let nearest_pos = Self::nearest_pos_on_screen_segment(pointer_pos, start, end);
            let distance = pointer_pos.distance(nearest_pos);
            if distance <= HIT_DISTANCE
                && nearest.is_none_or(|(_, nearest_distance): (egui::Pos2, f32)| {
                    distance < nearest_distance
                })
            {
                nearest = Some((nearest_pos, distance));
            }
        }

        nearest.map(|(pos, _)| viewport.screen_to_graph(pos))
    }

    pub fn is_timecontrol_anchor_near_pointer(
        timecontrol: &crate::keyframe::TimeControl,
        viewport: TimeControlViewport,
        pointer_pos: egui::Pos2,
    ) -> bool {
        timecontrol
            .points
            .iter()
            .any(|point| pointer_pos.distance(viewport.graph_to_screen(point.position)) <= 9.0)
    }

    pub fn nearest_pos_on_screen_segment(
        point: egui::Pos2,
        start: egui::Pos2,
        end: egui::Pos2,
    ) -> egui::Pos2 {
        let segment = end - start;
        let segment_length_sq = segment.length_sq();
        if segment_length_sq <= f32::EPSILON {
            return start;
        }

        let t = ((point - start).dot(segment) / segment_length_sq).clamp(0.0, 1.0);
        start + segment * t
    }

    pub fn timecontrol_vertical_bounds(timecontrol: &crate::keyframe::TimeControl) -> (f64, f64) {
        let mut min_y = 0.0_f64;
        let mut max_y = 1.0_f64;
        for position in timecontrol.sampled_points(96) {
            min_y = min_y.min(position[1]);
            max_y = max_y.max(position[1]);
        }
        (min_y, max_y)
    }
}
