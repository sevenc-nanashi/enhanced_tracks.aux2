use anyhow::Context;
use aviutl2::module::ScriptModuleFunctions;
use std::sync::Arc;

pub static DEBUG_MODE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub static CACHE_CLEARED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static RESOLVED_KEYFRAMES_CACHE: std::sync::LazyLock<
    dashmap::DashMap<crate::KeyframeTrackParams, ResolvedKeyframes>,
> = std::sync::LazyLock::new(dashmap::DashMap::new);
static RESOLVED_SCRIPT_CACHE: std::sync::LazyLock<dashmap::DashMap<String, ResolvedScript>> =
    std::sync::LazyLock::new(dashmap::DashMap::new);

#[derive(Debug, Clone)]
struct ResolvedKeyframes {
    keyframes: Vec<ResolvedKeyframe>,
}

#[derive(Debug, Clone)]
struct ResolvedKeyframe {
    indices: Vec<i32>,
    script_name: String,
    acceleration: bool,
    deceleration: bool,
    params: Vec<f64>,
    timecontrol: crate::keyframe::TimeControl,
}

#[derive(Debug, Clone)]
struct ResolvedScript {
    script_bytes: Arc<[u8]>,
    script_dir: String,
}

pub fn clear_runtime_caches() {
    RESOLVED_KEYFRAMES_CACHE.clear();
    RESOLVED_SCRIPT_CACHE.clear();
}

fn vec_to_quat(values: Vec<f64>) -> [f64; 4] {
    assert!(values.len() == 4, "quat must have 4 components");
    [values[0], values[1], values[2], values[3]]
}

fn resolve_keyframe(
    params: crate::KeyframeTrackParams,
    index: usize,
) -> anyhow::Result<ResolvedKeyframe> {
    if let Some(resolved) = RESOLVED_KEYFRAMES_CACHE.get(&params) {
        return resolved.keyframe(index);
    }

    let resolved = build_resolved_keyframes(params)?;
    let keyframe = resolved.keyframe(index)?;
    RESOLVED_KEYFRAMES_CACHE.insert(params, resolved);
    Ok(keyframe)
}

impl ResolvedKeyframes {
    fn keyframe(&self, index: usize) -> anyhow::Result<ResolvedKeyframe> {
        if self.keyframes.is_empty() {
            anyhow::bail!("resolved keyframes is empty");
        }
        let index = index.min(self.keyframes.len() - 1);
        Ok(self.keyframes[index].clone())
    }
}

fn build_resolved_keyframes(
    params: crate::KeyframeTrackParams,
) -> anyhow::Result<ResolvedKeyframes> {
    let keyframes = crate::KEYFRAMES.get(&params).with_context(|| {
        format!(
            "keyframes not found for bank_id: {}, track_id: {}, scene_id: {}, project_session_nonce: {}",
            params.bank_id, params.keyframes_id, params.scene_id, params.project_session_nonce
        )
    })?;
    if keyframes.keyframes.is_empty() {
        anyhow::bail!("keyframes is empty");
    }

    let mut resolved = Vec::with_capacity(keyframes.keyframes.len());
    for index in 0..keyframes.keyframes.len() {
        resolved.push(resolve_keyframe_at_index(&keyframes.keyframes, index)?);
    }
    Ok(ResolvedKeyframes {
        keyframes: resolved,
    })
}

fn resolve_keyframe_at_index(
    keyframes: &[crate::keyframe::Keyframe],
    index: usize,
) -> anyhow::Result<ResolvedKeyframe> {
    let (easing_index, keyframe) = keyframes
        .iter()
        .enumerate()
        .take(index + 1)
        .rfind(|(_, k)| matches!(k, crate::keyframe::Keyframe::Easing(_)))
        .context("first keyframe must be easing")?;
    let crate::keyframe::Keyframe::Easing(keyframe) = keyframe else {
        unreachable!()
    };

    let mut indices = Vec::with_capacity(keyframes.len() - easing_index);
    indices.push(easing_index as i32);
    for i in (easing_index + 1)..keyframes.len() {
        match &keyframes[i] {
            _ if i == keyframes.len() - 1 => {
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

    Ok(ResolvedKeyframe {
        indices,
        script_name: keyframe.easing.clone(),
        acceleration: keyframe.acceleration,
        deceleration: keyframe.deceleration,
        params: keyframe.params.clone(),
        timecontrol: keyframe.timecontrol.clone(),
    })
}

fn resolve_script(name: &str) -> anyhow::Result<ResolvedScript> {
    if let Some(script) = RESOLVED_SCRIPT_CACHE.get(name) {
        return Ok(script.clone());
    }
    let easings = crate::EASINGS.read().unwrap();
    let easing = easings.get(name).context("easing not found")?;
    let resolved = ResolvedScript {
        script_bytes: Arc::from(easing.script_bytes.clone()),
        script_dir: easing
            .path
            .as_ref()
            .and_then(|path| path.parent())
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default(),
    };
    RESOLVED_SCRIPT_CACHE.insert(name.to_string(), resolved.clone());
    Ok(resolved)
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
    ) -> aviutl2::common::AnyResult<(Vec<i32>, String, bool, bool, Vec<f64>)> {
        let param = crate::KeyframeTrackParams {
            bank_id: bank_id as _,
            keyframes_id: track_id as _,
            scene_id: scene_id as _,
            project_session_nonce: project_session_nonce as _,
        };
        let resolved = resolve_keyframe(param, index).with_context(|| {
            format!(
                "failed to resolve keyframe for bank_id: {bank_id}, track_id: {track_id}, scene_id: {scene_id}, project_session_nonce: {project_session_nonce}, index: {index}"
            )
        })?;
        Ok((
            resolved.indices,
            resolved.script_name,
            resolved.acceleration,
            resolved.deceleration,
            resolved.params,
        ))
    }

    fn get_script(script_name: String) -> aviutl2::common::AnyResult<(*const u8, i32, String)> {
        let script = resolve_script(&script_name)
            .with_context(|| format!("failed to resolve script: {script_name}"))?;
        Ok((
            script.script_bytes.as_ptr(),
            script.script_bytes.len().try_into()?,
            script.script_dir,
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
        let resolved = resolve_keyframe(param, index).context("failed to resolve keyframe")?;

        Ok(resolved.timecontrol.y_at_x(x))
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
