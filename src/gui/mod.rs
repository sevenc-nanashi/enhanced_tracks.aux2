use anyhow::Context;
use aviutl2::generic::GenericPlugin;
use aviutl2_eframe::{eframe, egui};
use tap::prelude::*;

mod debug_view;
mod edit_section_thread;
mod keyframes;
mod timecontrol;

use edit_section_thread::EditSectionThread;

pub struct KeyframesGui {
    pub selected_object_info: Option<SelectedObjectInfo>,
    pub timecontrol_editor: Option<TimeControlEditorTarget>,
    pub timecontrol_clipboard: Option<crate::keyframe::TimeControl>,
    pub easing_search_text: String,
    pub keyframe_timeline_view: KeyframeTimelineView,
    pub debug_counter: usize,
    pub debug_view: bool,
    edit_info: Option<aviutl2::generic::EditInfo>,
    edit_section_thread: EditSectionThread,
}

#[derive(Debug, Clone, Copy)]
pub struct KeyframeTimelineView {
    pub left: f32,
    pub right: f32,
}

impl Default for KeyframeTimelineView {
    fn default() -> Self {
        Self {
            left: 0.0,
            right: 1.0,
        }
    }
}

impl KeyframeTimelineView {
    fn width(self) -> f32 {
        self.right - self.left
    }

    fn translate(self, delta: f32) -> Self {
        Self {
            left: self.left + delta,
            right: self.right + delta,
        }
        .clamp()
    }

    fn zoom_at(self, anchor: f32, factor: f32) -> Self {
        assert!(factor.is_finite());
        let new_width = (self.width() / factor).clamp(0.01, 1.0);
        let left = anchor - (anchor - self.left) / self.width() * new_width;
        Self {
            left,
            right: left + new_width,
        }
        .clamp()
    }

    fn clamp(self) -> Self {
        assert!(self.left.is_finite());
        assert!(self.right.is_finite());
        let width = self.width().clamp(0.01, 1.0);
        let left = self.left.clamp(0.0, 1.0 - width);
        Self {
            left,
            right: left + width,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TimeControlEditorTarget {
    pub params: crate::KeyframeTrackParams,
    pub keyframe_index: usize,
    pub object: aviutl2::generic::ObjectHandle,
    pub effect: aviutl2::generic::EffectHandle,
    pub effect_name: String,
    pub track_names: Vec<String>,
    pub timecontrol: crate::keyframe::TimeControl,
    pub selected_point: usize,
    pub context_menu_position: Option<[f64; 2]>,
    pub preset_panel_width: f32,
    pub visible_y_bounds: Option<TimeControlVerticalBounds>,
    pub drag_scroll_y_bounds: Option<TimeControlVerticalBounds>,
    pub dirty: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct TimeControlVerticalBounds {
    pub min_y: f64,
    pub max_y: f64,
}

impl TimeControlVerticalBounds {
    fn y_range(self) -> f64 {
        self.max_y - self.min_y
    }

    fn center(self) -> f64 {
        (self.min_y + self.max_y) / 2.0
    }

    fn translate(self, delta: f64) -> Self {
        Self {
            min_y: self.min_y + delta,
            max_y: self.max_y + delta,
        }
    }

    fn union(self, other: Self) -> Self {
        Self {
            min_y: self.min_y.min(other.min_y),
            max_y: self.max_y.max(other.max_y),
        }
    }

    fn with_center_and_range(center: f64, range: f64) -> Self {
        let half_range = range / 2.0;
        Self {
            min_y: center - half_range,
            max_y: center + half_range,
        }
    }

    fn with_anchor_and_range(anchor_y: f64, anchor_ratio: f64, range: f64) -> Self {
        let min_y = anchor_y - anchor_ratio * range;
        Self {
            min_y,
            max_y: min_y + range,
        }
    }

    fn clamp_to_content(self, content: Self) -> Self {
        let content_range = content.y_range().max(0.000_001);
        let range = self.y_range().clamp(content_range / 8.0, content_range);
        if range >= content_range {
            return content;
        }

        let mut bounds = Self::with_center_and_range(self.center(), range);
        if bounds.min_y < content.min_y {
            bounds = bounds.translate(content.min_y - bounds.min_y);
        }
        if bounds.max_y > content.max_y {
            bounds = bounds.translate(content.max_y - bounds.max_y);
        }
        bounds
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TimeControlHandleKind {
    In,
    Out,
}
impl TimeControlHandleKind {
    fn id(self) -> &'static str {
        match self {
            TimeControlHandleKind::In => "in",
            TimeControlHandleKind::Out => "out",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectType {
    Control,
    VideoInput,
    VideoEffect,
    VideoFilter,
    AudioInput,
    AudioEffect,
    AudioFilter,
}

#[derive(Debug, Clone)]
pub struct SelectedObjectInfo {
    pub handle: aviutl2::generic::ObjectHandle,
    pub name: String,
    pub frames: Vec<usize>,
    pub effects: Vec<EffectInfo>,
}

#[derive(Debug, Clone)]
pub struct EffectInfo {
    pub handle: aviutl2::generic::EffectHandle,
    pub name: String,
    pub effect_type: EffectType,
    pub is_output: bool,
    pub keyframe_tracks: indexmap::IndexMap<String, KeyframeTrackInfo>,
}

#[derive(Debug, Clone)]
pub struct KeyframeTrackInfo {
    pub params: crate::KeyframeTrackParams,
    pub names: Vec<String>,
}

pub struct GuiColors {
    text: egui::Color32,
    log_warn: egui::Color32,
    frame_cursor: egui::Color32,
    grid_line: egui::Color32,
    zoom_gauge: egui::Color32,
    anchor: egui::Color32,
    anchor_line: egui::Color32,
    anchor_hover: egui::Color32,
    anchor_select: egui::Color32,
    object_section: egui::Color32,
    object_section_ignored: egui::Color32,
    object_control: ObjectColors,
    object_video: ObjectColors,
    object_video_effect: ObjectColors,
    object_video_filter: ObjectColors,
    object_audio: ObjectColors,
    object_audio_effect: ObjectColors,
    object_audio_filter: ObjectColors,
}

#[derive(Clone)]
pub struct ObjectColors {
    normal: Vec<egui::Color32>,
    selected: egui::Color32,
}

pub static GUI_COLORS: std::sync::LazyLock<GuiColors> = std::sync::LazyLock::new(GuiColors::load);

impl GuiColors {
    fn load() -> Self {
        Self {
            text: color_code("Text"),
            log_warn: color_code("LogWarn"),
            frame_cursor: color_code("FrameCursor"),
            grid_line: color_code("GridLine"),
            zoom_gauge: color_code("ZoomGauge"),
            anchor: color_code("Anchor"),
            anchor_line: color_code("AnchorLine"),
            anchor_hover: color_code("AnchorHover"),
            anchor_select: color_code("AnchorSelect"),
            object_section: color_code("ObjectSection"),
            object_section_ignored: color_code("Background"),
            object_control: object_colors("ObjectControl", "ObjectControlSelect"),
            object_video: object_colors("ObjectVideo", "ObjectVideoSelect"),
            object_video_effect: object_colors("ObjectVideoEffect", "ObjectVideoEffectSelect"),
            object_video_filter: object_colors("ObjectVideoFilter", "ObjectVideoFilterSelect"),
            object_audio: object_colors("ObjectAudio", "ObjectAudioSelect"),
            object_audio_effect: object_colors("ObjectAudioEffect", "ObjectAudioEffectSelect"),
            object_audio_filter: object_colors("ObjectAudioFilter", "ObjectAudioFilterSelect"),
        }
    }
}

fn color_code(key: &str) -> egui::Color32 {
    aviutl2::config::get_color_code(key)
        .expect("Null文字はない")
        .unwrap_or_else(|| panic!("{key} が style.conf に存在しない"))
        .pipe(|(r, g, b)| egui::Color32::from_rgb(r, g, b))
}

fn color_codes(key: &str) -> Vec<egui::Color32> {
    aviutl2::config::get_all_color_codes(key)
        .unwrap_or_else(|_| panic!("{key} が style.conf に存在しない"))
        .into_iter()
        .map(|(r, g, b)| egui::Color32::from_rgb(r, g, b))
        .collect()
}

fn object_colors(normal: &str, selected: &str) -> ObjectColors {
    let normal = color_codes(normal);
    let normal = if normal.len() == 1 {
        vec![normal[0], normal[0]]
    } else {
        normal
    };
    ObjectColors {
        normal,
        selected: color_code(selected),
    }
}

pub fn get_colors(object_type: &EffectType) -> (Vec<egui::Color32>, egui::Color32) {
    let colors = match object_type {
        EffectType::Control => &GUI_COLORS.object_control,
        EffectType::VideoInput => &GUI_COLORS.object_video,
        EffectType::VideoEffect => &GUI_COLORS.object_video_effect,
        EffectType::VideoFilter => &GUI_COLORS.object_video_filter,
        EffectType::AudioInput => &GUI_COLORS.object_audio,
        EffectType::AudioEffect => &GUI_COLORS.object_audio_effect,
        EffectType::AudioFilter => &GUI_COLORS.object_audio_filter,
    };
    (colors.normal.clone(), colors.selected)
}

pub fn create_gui(
    cc: &aviutl2_eframe::eframe::CreationContext,
    _handle: aviutl2_eframe::AviUtl2EframeHandle,
) -> Result<Box<dyn aviutl2_eframe::eframe::App>, Box<dyn std::error::Error + Send + Sync>> {
    cc.egui_ctx.all_styles_mut(|style| {
        style.visuals = aviutl2_eframe::aviutl2_visuals();
    });
    cc.egui_ctx.set_fonts(aviutl2_eframe::aviutl2_fonts());
    let edit_section_thread = EditSectionThread::start(cc.egui_ctx.clone())?;
    Ok(Box::new(KeyframesGui {
        selected_object_info: None,
        timecontrol_editor: None,
        timecontrol_clipboard: None,
        easing_search_text: String::new(),
        keyframe_timeline_view: KeyframeTimelineView::default(),
        debug_counter: 0,
        debug_view: false,
        edit_info: None,
        edit_section_thread,
    }))
}

impl aviutl2_eframe::eframe::App for KeyframesGui {
    fn logic(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if crate::EDIT_HANDLE.is_ready() {
            if crate::EDIT_HANDLE
                .get_edit_state()
                .is_ok_and(|state| state == aviutl2::generic::EditState::Save)
            {
                return;
            }

            if let Some(result) = self.edit_section_thread.try_recv() {
                match result {
                    Ok((edit_info, selected_object_info, timecontrol_editor, requested_effect)) => {
                        self.edit_info = Some(edit_info);
                        self.selected_object_info = selected_object_info;
                        let current_effect =
                            self.timecontrol_editor.as_ref().map(|target| target.effect);
                        if current_effect == requested_effect
                            && !self
                                .timecontrol_editor
                                .as_ref()
                                .is_some_and(|target| target.dirty)
                        {
                            match (&mut self.timecontrol_editor, timecontrol_editor) {
                                (Some(current), Some(updated)) => {
                                    current.timecontrol = updated.timecontrol;
                                }
                                (current @ Some(_), None) => {
                                    *current = None;
                                }
                                (None, None) => {}
                                (None, Some(_)) => {
                                    unreachable!("The edit section thread must not open an editor");
                                }
                            }
                        }
                    }
                    Err(error) => {
                        tracing::error!("Failed to update selected object info: {:?}", error);
                    }
                }
            }

            self.edit_section_thread
                .request(self.timecontrol_editor.clone());
        }
    }
    fn ui(
        &mut self,
        ui: &mut aviutl2_eframe::egui::Ui,
        _frame: &mut aviutl2_eframe::eframe::Frame,
    ) {
        egui::CentralPanel::default().show(ui, |ui| {
            if crate::EDIT_HANDLE.is_ready() {
                if self.debug_view {
                    self.render_debug_view(ui);
                }
                if self.is_undo_mode() {
                    self.render_undo_mode_warning(ui);
                } else if self.timecontrol_editor.is_some() {
                    self.render_timecontrol_editor(ui);
                } else {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        self.render_selected_object_info(ui);
                    });
                }
            } else {
                ui.label(aviutl2::config::translate("初期化中..."));
            }
        });
    }
}

impl KeyframesGui {
    pub fn update_keyframes_for_tracks(
        effect: aviutl2::generic::EffectHandle,
        track_names: &[String],
        new_keyframes: crate::keyframe::Keyframes,
    ) -> anyhow::Result<crate::KeyframeTrackParams> {
        crate::EDIT_HANDLE
            .call_edit_section(|edit| {
                let new_params = crate::KeyframeTrackParams::new(edit.info.scene_id);
                crate::KEYFRAMES.insert(new_params, new_keyframes);
                for name in track_names {
                    new_params.set_params(edit, effect, name)?;
                }
                anyhow::Ok(new_params)
            })
            .map_err(anyhow::Error::from)
            .flatten()
    }

    fn is_undo_mode(&self) -> bool {
        let Some(selected_object_info) = &self.selected_object_info else {
            return false;
        };

        selected_object_info.effects.iter().any(|effect| {
            effect.keyframe_tracks.values().any(|params| {
                crate::KEYFRAMES
                    .get(&params.params)
                    .is_some_and(|keyframes| {
                        keyframes.keyframes.len() != selected_object_info.frames.len()
                    })
            })
        })
    }

    fn render_undo_mode_warning(&self, ui: &mut egui::Ui) {
        let (response, painter) =
            ui.allocate_painter(ui.available_size(), aviutl2_eframe::egui::Sense::click());
        let rect = response.rect;

        if response.clicked() {
            crate::KeyframesAux2::with_instance(|aux| {
                aux.watcher.flush_resolved_migrations();
            });
        }

        let color = GUI_COLORS.log_warn;

        let mut layout = egui::text::LayoutJob::default();
        layout.append(
            &format!("{}\n", aviutl2::config::translate("一時停止中")),
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::proportional(18.0),
                color,
                ..Default::default()
            },
        );
        layout.append(
            &aviutl2::config::translate(
                "Undoを妨げないために同期を中断しています。クリックで再同期します。",
            ),
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::default(),
                color: GUI_COLORS.text,
                ..Default::default()
            },
        );
        layout.wrap = egui::text::TextWrapping::wrap_at_width(rect.width());
        let galley = painter.layout_job(layout);
        painter.galley(
            rect.center().tap_mut(|pos| {
                pos.x -= galley.size().x / 2.0;
                pos.y -= galley.size().y / 2.0;
            }),
            galley,
            color,
        );
    }

    fn read_selected_object_info(
        read: &aviutl2::generic::ReadSection,
    ) -> aviutl2::common::AnyResult<Option<SelectedObjectInfo>> {
        let selected_object = read.get_focused_object()?;
        let Some(selected_object) = selected_object else {
            return Ok(None);
        };
        let first_effect_name = read
            .get_effect_name(
                read.object(selected_object)
                    .get_first_effect()
                    .context("Failed to get first effect")?,
            )
            .context("Failed to get first effect name")?;
        let first_effect_info = crate::EFFECTS
            .get(&first_effect_name)
            .context("Failed to get effect info")?;
        let first_effect_type = Self::determine_effect_type(&first_effect_info, None);
        let mut effects = Vec::new();
        for effect in read
            .object(selected_object)
            .get_effects()
            .context("Failed to get effects")?
        {
            let effect = read.effect(effect);
            let effect_name = effect.get_name().context("Failed to get effect name")?;

            let effect_info = crate::EFFECTS
                .get(&effect_name)
                .context("Failed to get effect info")?;
            let effect_type = Self::determine_effect_type(&effect_info, Some(first_effect_type));

            let mut effect_info = EffectInfo {
                handle: effect.handle,
                name: effect_name.to_string(),
                is_output: matches!(
                    effect_info.effect_type,
                    aviutl2::generic::EffectType::Output
                ),
                effect_type,
                keyframe_tracks: indexmap::IndexMap::new(),
            };
            crate::EDIT_HANDLE.enumerate_effect_items(&effect_name, |item| {
                if item.item_type != aviutl2::generic::EffectItemType::Number {
                    return;
                }
                if let Some(params) =
                    crate::KeyframeTrackParams::parse(read, effect.handle, &item.name)
                {
                    // let keyframe_info = KeyframeTrackInfo {
                    //     name: key.to_string(),
                    //     params,
                    // };
                    // effect_info.keyframe_tracks.push(keyframe_info);
                    match read.get_effect_track_info(effect.handle, &item.name) {
                        Ok(Some(track_info)) => {
                            effect_info
                                .keyframe_tracks
                                .entry(
                                    track_info
                                        .group_name
                                        .unwrap_or_else(|| item.name.to_owned()),
                                )
                                .or_insert_with(|| KeyframeTrackInfo {
                                    params,
                                    names: Vec::new(),
                                })
                                .names
                                .push(item.name.to_string());
                        }
                        Ok(None) => {
                            tracing::warn!(
                                "Failed to get track info for effect {} item {}: track info is None",
                                effect_name,
                                item.name
                            );
                        }
                        Err(e) => {
                            tracing::error!(
                                "Failed to get track info for effect {} item {}: {:?}",
                                effect_name,
                                item.name,
                                e
                            );
                        }
                    }
                }
            })?;
            effects.push(effect_info);
        }

        let raw_object_name = read.get_object_name(selected_object)?.unwrap_or_else(|| {
            effects
                .iter()
                .find(|e| e.effect_type != EffectType::Control)
                .map(|e| e.name.clone())
                .or_else(|| effects.first().map(|e| e.name.clone()))
                .unwrap_or_else(|| "Unknown Object".to_string())
        });
        let output = effects.iter().find(|e| {
            crate::EFFECTS
                .get(&e.name)
                .is_some_and(|e| e.effect_type == aviutl2::generic::EffectType::Output)
        });
        let object_name = if let Some(output_effect) = output {
            format!(
                "{} [{}]",
                crate::utils::get_translated_effect_name(&raw_object_name),
                crate::utils::get_translated_effect_name(&output_effect.name)
            )
        } else {
            crate::utils::get_translated_effect_name(&raw_object_name)
        };

        // 出力制御は上に持っていく
        effects.sort_by_key(|e| {
            if crate::EFFECTS
                .get(&e.name)
                .is_some_and(|e| e.effect_type == aviutl2::generic::EffectType::Output)
            {
                -1
            } else {
                1
            }
        });

        // let frames = read
        //     .get_object_section_frames(selected_object)
        //     .context("Failed to get object section frames")?;
        let frames = {
            let section_num = read.get_object_section_num(selected_object)?;
            let mut frames = Vec::new();
            for section in 0..section_num {
                frames.push(
                    read.get_object_section_frame(selected_object, section)?
                        .ok_or_else(|| {
                            anyhow::anyhow!("Failed to get frame for section {section}")
                        })?,
                );
            }
            let last_frame = read.get_object_layer_frame(selected_object)?;
            frames.push(last_frame.end + 1);
            frames
        };
        let selected_object_info = SelectedObjectInfo {
            handle: selected_object,
            name: object_name,
            frames,
            effects,
        };
        Ok(Some(selected_object_info))
    }

    fn determine_effect_type(
        effect_info: &aviutl2::generic::Effect,
        first_effect_type: Option<EffectType>,
    ) -> EffectType {
        match effect_info.effect_type {
            aviutl2::generic::EffectType::Filter
                if matches!(first_effect_type, Some(EffectType::Control)) =>
            {
                if effect_info.flag.audio {
                    EffectType::AudioFilter
                } else {
                    EffectType::VideoFilter
                }
            }
            aviutl2::generic::EffectType::Filter if effect_info.flag.audio => {
                EffectType::AudioEffect
            }
            aviutl2::generic::EffectType::Filter => EffectType::VideoEffect,

            aviutl2::generic::EffectType::Input if effect_info.flag.video => EffectType::VideoInput,
            aviutl2::generic::EffectType::Input => EffectType::AudioInput,
            aviutl2::generic::EffectType::SceneChange => EffectType::Control,
            aviutl2::generic::EffectType::Control => EffectType::Control,
            aviutl2::generic::EffectType::Output => {
                first_effect_type.unwrap_or(EffectType::Control)
            }
        }
    }
}
