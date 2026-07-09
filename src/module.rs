use anyhow::Context;
use aviutl2::module::ScriptModuleFunctions;

pub static DEBUG_MODE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub static CACHE_CLEARED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn vec_to_quat(values: Vec<f64>) -> [f64; 4] {
    assert!(values.len() == 4, "quat must have 4 components");
    [values[0], values[1], values[2], values[3]]
}

#[aviutl2::plugin(ScriptModule)]
pub struct KeyframesMod2 {}

impl aviutl2::module::ScriptModule for KeyframesMod2 {
    fn new(_info: aviutl2::common::AviUtl2Info) -> aviutl2::common::AnyResult<Self> {
        Ok(Self {})
    }

    fn plugin_info(&self) -> aviutl2::module::ScriptModuleTable {
        aviutl2::module::ScriptModuleTable {
            information: "enhanced_tracks.aux2: internal module".into(),
            functions: Self::functions(),
        }
    }
}

#[aviutl2::module::functions]
impl KeyframesMod2 {
    #[expect(clippy::type_complexity)]
    fn get_keyframe(
        bank_id: i32,
        track_id: i32,
        scene_id: i32,
        project_session_nonce: i32,
        index: usize,
    ) -> aviutl2::common::AnyResult<(
        Vec<i32>,
        String,
        *const u8,
        i32,
        String,
        bool,
        bool,
        Vec<f64>,
    )> {
        let param = crate::KeyframeTrackParams {
            bank_id: bank_id as _,
            keyframes_id: track_id as _,
            scene_id: scene_id as _,
            project_session_nonce: project_session_nonce as _,
        };
        let mut keyframes = crate::KEYFRAMES.get_mut(&param).with_context(|| {
            format!(
                "keyframes not found for bank_id: {bank_id}, track_id: {track_id}, scene_id: {scene_id}, project_session_nonce: {project_session_nonce}"
            )
        })?;
        {
            let Some(last_keyframe) = keyframes.keyframes.last_mut() else {
                tracing::error!(
                    "unreachable: keyframes is empty for bank_id: {bank_id}, track_id: {track_id}, scene_id: {scene_id}, project_session_nonce: {project_session_nonce}"
                );
                return Err(anyhow::anyhow!("keyframes is empty"));
            };
            if !matches!(last_keyframe, crate::keyframe::Keyframe::Midpoint) {
                tracing::warn!(
                    "unreachable: last keyframe is not Midpoint for bank_id: {bank_id}, track_id: {track_id}, scene_id: {scene_id}, project_session_nonce: {project_session_nonce}"
                );
                *last_keyframe = crate::keyframe::Keyframe::Midpoint;
            }
        }
        let (index, keyframe) = keyframes
            .keyframes
            .iter()
            .enumerate()
            .take(index + 1)
            .rfind(|(_, k)| matches!(k, crate::keyframe::Keyframe::Easing(_)))
            .expect("first keyframe must be easing");
        let mut indices = vec![index as i32];
        let crate::keyframe::Keyframe::Easing(keyframe) = keyframe else {
            unreachable!()
        };
        for i in (index + 1)..keyframes.keyframes.len() {
            match &keyframes.keyframes[i] {
                _ if i == keyframes.keyframes.len() - 1 => {
                    indices.push(i as i32);
                    break;
                }
                crate::keyframe::Keyframe::Easing(_) => {
                    indices.push(i as i32);
                    break;
                }
                crate::keyframe::Keyframe::Midpoint => indices.push(i as i32),
                crate::keyframe::Keyframe::Ignored => (),
            }
        }
        let easings = crate::EASINGS.read().unwrap();
        let easing = easings.get(&keyframe.easing).context("easing not found")?;
        Ok((
            indices,
            easing.name.clone(),
            easing.script_bytes.as_ptr(),
            easing.script_bytes.len().try_into()?,
            easing
                .path
                .clone()
                .unwrap_or_default()
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default(),
            keyframe.acceleration,
            keyframe.deceleration,
            keyframe.params.clone(),
        ))
    }

    fn get_timecontrol_value(
        bank_id: i32,
        track_id: i32,
        scene_id: i32,
        project_session_nonce: i32,
        index: usize,
        x: f64,
    ) -> aviutl2::common::AnyResult<f64> {
        let param = crate::KeyframeTrackParams {
            bank_id: bank_id as _,
            keyframes_id: track_id as _,
            scene_id: scene_id as _,
            project_session_nonce: project_session_nonce as _,
        };
        let keyframes = crate::KEYFRAMES
            .get(&param)
            .context("keyframes not found")?;
        let keyframe = keyframes
            .keyframes
            .iter()
            .take(index + 1)
            .rfind(|k| matches!(k, crate::keyframe::Keyframe::Easing(_)))
            .expect("first keyframe must be easing");
        let crate::keyframe::Keyframe::Easing(keyframe) = keyframe else {
            unreachable!()
        };

        Ok(keyframe.timecontrol.y_at_x(x))
    }

    fn resolve_segment(point_count: i32, segment: f64, local_t: f64) -> (i32, f64) {
        assert!(point_count >= 0, "point_count must be non-negative");
        crate::std_common::resolve_segment(point_count as usize, segment, local_t)
    }

    fn linear_value(
        values: Vec<f64>,
        segment: f64,
        local_t: f64,
        double_first: bool,
        double_last: bool,
    ) -> f64 {
        crate::std_common::linear_value(&values, segment, local_t, double_first, double_last)
    }

    fn segment_lengths(
        axis1: Vec<f64>,
        axis2: Vec<f64>,
        axis3: Vec<f64>,
        double_first: bool,
        double_last: bool,
    ) -> Vec<f64> {
        crate::std_common::segment_lengths(&axis1, &axis2, &axis3, double_first, double_last)
    }

    fn weighted_segment(
        axis1: Vec<f64>,
        axis2: Vec<f64>,
        axis3: Vec<f64>,
        t: f64,
        double_first: bool,
        double_last: bool,
    ) -> (i32, f64, Vec<f64>) {
        crate::std_common::weighted_segment(&axis1, &axis2, &axis3, t, double_first, double_last)
    }

    #[expect(clippy::too_many_arguments)]
    fn catmull_rom(
        start_prev: f64,
        start_value: f64,
        end_value: f64,
        end_next: f64,
        len_prev: f64,
        len_cur: f64,
        len_next: f64,
        t: f64,
    ) -> f64 {
        crate::std_common::catmull_rom(
            start_prev,
            start_value,
            end_value,
            end_next,
            len_prev,
            len_cur,
            len_next,
            t,
        )
    }

    fn interpolation_value(
        values: Vec<f64>,
        lengths: Vec<f64>,
        segment: f64,
        local_t: f64,
        double_first: bool,
        double_last: bool,
    ) -> f64 {
        crate::std_common::interpolation_value(
            &values,
            &lengths,
            segment,
            local_t,
            double_first,
            double_last,
        )
    }

    fn build_rotation_series(values: Vec<f64>, period: f64) -> Vec<f64> {
        crate::std_common::build_rotation_series(&values, period)
    }

    fn euler_quat_at(
        axis1: Vec<f64>,
        axis2: Vec<f64>,
        axis3: Vec<f64>,
        index: f64,
        order: String,
    ) -> Vec<f64> {
        crate::std_common::euler_quat_at(&axis1, &axis2, &axis3, index, &order).to_vec()
    }

    fn quat_slerp(a: Vec<f64>, b: Vec<f64>, t: f64) -> Vec<f64> {
        crate::std_common::quat_slerp(vec_to_quat(a), vec_to_quat(b), t).to_vec()
    }

    fn rotation_component_from_quat(
        axis_index: i32,
        rotation_order: String,
        quat: Vec<f64>,
    ) -> f64 {
        assert!(
            (1..=3).contains(&axis_index),
            "axis_index must be between 1 and 3"
        );
        let _ = rotation_order;
        crate::std_common::rotation_component_from_quat(axis_index, vec_to_quat(quat))
    }

    fn debug_mode(&self) -> bool {
        DEBUG_MODE.load(std::sync::atomic::Ordering::Relaxed)
    }

    fn is_cache_cleared(&self) -> bool {
        CACHE_CLEARED.load(std::sync::atomic::Ordering::Relaxed)
    }
    fn reset_cache_cleared(&self) {
        CACHE_CLEARED.store(false, std::sync::atomic::Ordering::Relaxed);
    }

    fn current_time(&self) -> f64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64()
    }
}
