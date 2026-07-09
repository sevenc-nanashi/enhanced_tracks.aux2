// このファイルの内容はmimaraka/aviutl-plugin-curve_editorのコードを参考にしています。
//
// https://github.com/mimaraka/aviutl-plugin-curve_editor/blob/main/LICENSE.txt
// ```
// MIT License
//
// Copyright (c) 2022 mimaraka
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.
// ```

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Keyframes {
    pub keyframes: Vec<Keyframe>,
}
impl Keyframes {
    pub fn new(num_keyframes: usize) -> Self {
        let mut keyframes = vec![Keyframe::default(); num_keyframes];
        keyframes[0] = Keyframe::Easing(EasingKeyframeInfo::default());
        Self { keyframes }
    }
    pub fn resize(&mut self, num_keyframes: usize) {
        if num_keyframes <= 1 {
            panic!("num_keyframes must be greater than 1");
        }
        if self.keyframes.len() < num_keyframes {
            self.keyframes.extend(vec![
                Keyframe::default();
                num_keyframes - self.keyframes.len()
            ]);
        } else {
            self.keyframes.truncate(num_keyframes);
        }
    }

    pub fn remap_to_frames(&self, old_frames: &[usize], new_frames: &[usize]) -> Self {
        if old_frames.len() != self.keyframes.len() {
            panic!("old_frames length must match keyframes length");
        }
        if new_frames.len() <= 1 {
            panic!("new_frames length must be greater than 1");
        }

        let old_frame_indices = old_frames
            .iter()
            .enumerate()
            .map(|(index, frame)| (*frame, index))
            .collect::<std::collections::HashMap<_, _>>();
        let mut keyframes = Vec::with_capacity(new_frames.len());

        for (new_index, frame) in new_frames.iter().enumerate() {
            if new_index == 0 {
                keyframes.push(
                    self.easing_at_or_before_frame(old_frames, *frame)
                        .map_or_else(Keyframe::default, |keyframe| {
                            Keyframe::Easing(keyframe.clone())
                        }),
                );
                continue;
            }

            let Some(old_index) = old_frame_indices.get(frame) else {
                keyframes.push(Keyframe::Midpoint);
                continue;
            };
            keyframes.push(self.keyframes[*old_index].clone());
        }

        if !matches!(keyframes[0], Keyframe::Easing(_)) {
            keyframes[0] = self
                .easing_at_or_before_frame(old_frames, new_frames[0])
                .map_or_else(Keyframe::default, |keyframe| {
                    Keyframe::Easing(keyframe.clone())
                });
        }

        Self { keyframes }
    }

    fn easing_at_or_before_frame(
        &self,
        frames: &[usize],
        target_frame: usize,
    ) -> Option<&EasingKeyframeInfo> {
        frames
            .iter()
            .zip(&self.keyframes)
            .take_while(|(frame, _)| **frame <= target_frame)
            .filter_map(|(_, keyframe)| match keyframe {
                Keyframe::Easing(keyframe) => Some(keyframe),
                Keyframe::Ignored | Keyframe::Midpoint => None,
            })
            .last()
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub enum Keyframe {
    Easing(EasingKeyframeInfo),
    Ignored,
    #[default]
    Midpoint,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EasingKeyframeInfo {
    pub easing: String,
    pub acceleration: bool,
    pub deceleration: bool,
    pub params: Vec<f64>,
    pub timecontrol: TimeControl,
}
impl Default for EasingKeyframeInfo {
    fn default() -> Self {
        Self {
            easing: "直線移動".to_string(),
            acceleration: false,
            deceleration: false,
            params: Vec::new(),
            timecontrol: TimeControl::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TimeControlMode {
    Bezier,
    Elastic,
    Bounce,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TimeControl {
    pub points: Vec<TimeControlPoint>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TimeControlSegment {
    Bezier,
    Elastic(TimeControlElastic),
    Bounce(TimeControlBounce),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TimeControlPoint {
    pub position: [f64; 2],
    pub in_handle: Option<[f64; 2]>,
    pub out_handle: Option<[f64; 2]>,
    pub handles_separated: bool,
    pub outgoing: Option<TimeControlSegment>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TimeControlElastic {
    pub reversed: bool,
    #[serde(default = "TimeControlElastic::default_amplitude")]
    pub amplitude: f64,
    #[serde(default = "TimeControlElastic::default_frequency")]
    pub frequency: f64,
    #[serde(default = "TimeControlElastic::default_decay")]
    pub decay: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TimeControlBounce {
    pub reversed: bool,
    pub vertex: [f64; 2],
}

impl Default for TimeControl {
    fn default() -> Self {
        Self::from_cubic_bezier([1.0 / 3.0, 1.0 / 3.0, 2.0 / 3.0, 2.0 / 3.0])
    }
}

impl Default for TimeControlElastic {
    fn default() -> Self {
        Self {
            reversed: false,
            amplitude: Self::default_amplitude(),
            frequency: Self::default_frequency(),
            decay: Self::default_decay(),
        }
    }
}

impl Default for TimeControlBounce {
    fn default() -> Self {
        Self {
            reversed: false,
            vertex: [0.55, 0.75],
        }
    }
}

impl TimeControl {
    pub fn default_for_mode(mode: TimeControlMode) -> Self {
        let mut timecontrol = Self::default();
        timecontrol.points[0].outgoing = Some(TimeControlSegment::default_for_mode(mode));
        timecontrol
    }

    pub fn mode(&self) -> TimeControlMode {
        self.segment_mode(0).unwrap_or(TimeControlMode::Bezier)
    }

    pub fn y_at_x(&self, x: f64) -> f64 {
        let x = x.clamp(0.0, 1.0);
        self.curve_y_at_x(x)
    }

    pub fn curve_y_at_x(&self, x: f64) -> f64 {
        let x = x.clamp(0.0, 1.0);
        let segment_index = self.segment_index_at_x(x);
        self.segment_y_at_x(segment_index, x)
    }

    pub fn curve_sampled_points(&self, steps: usize) -> Vec<[f64; 2]> {
        let steps = steps.max(1);
        let mut points = Vec::new();
        for segment_index in 0..self.points.len().saturating_sub(1) {
            if points.is_empty() {
                points.push(self.points[segment_index].position);
            }
            for i in 1..=steps {
                let t = i as f64 / steps as f64;
                points.push(self.segment_point_at(segment_index, t));
            }
        }
        points
    }

    pub fn point_at(&self, t: f64) -> [f64; 2] {
        self.y_point_at_x(t)
    }

    pub fn y_point_at_x(&self, x: f64) -> [f64; 2] {
        let x = x.clamp(0.0, 1.0);
        [x, self.y_at_x(x)]
    }

    pub fn sampled_points(&self, steps: usize) -> Vec<[f64; 2]> {
        let steps = steps.max(1);
        (0..=steps)
            .map(|i| {
                let x = i as f64 / steps as f64;
                [x, self.y_at_x(x)]
            })
            .collect()
    }

    pub fn editable_vertex(&self) -> Option<[f64; 2]> {
        self.segment_vertex(0)
    }

    pub fn set_editable_vertex(&mut self, vertex: [f64; 2]) {
        self.set_segment_vertex(0, vertex);
    }

    pub fn from_cubic_bezier(cubic_bezier: [f64; 4]) -> Self {
        Self {
            points: vec![
                TimeControlPoint {
                    position: [0.0, 0.0],
                    in_handle: None,
                    out_handle: Some([cubic_bezier[0], cubic_bezier[1]]),
                    handles_separated: false,
                    outgoing: Some(TimeControlSegment::Bezier),
                },
                TimeControlPoint {
                    position: [1.0, 1.0],
                    in_handle: Some([cubic_bezier[2], cubic_bezier[3]]),
                    out_handle: None,
                    handles_separated: false,
                    outgoing: None,
                },
            ],
        }
    }

    pub fn segment_mode(&self, segment_index: usize) -> Option<TimeControlMode> {
        self.points
            .get(segment_index)
            .and_then(|point| point.outgoing.as_ref())
            .map(TimeControlSegment::mode)
    }

    pub fn set_segment_mode(&mut self, segment_index: usize, mode: TimeControlMode) {
        if segment_index + 1 >= self.points.len() {
            return;
        }
        self.points[segment_index].outgoing = Some(TimeControlSegment::default_for_mode(mode));
        if mode == TimeControlMode::Bezier {
            self.reset_segment_handles(segment_index);
        }
    }

    pub fn segment_vertex(&self, segment_index: usize) -> Option<[f64; 2]> {
        match self
            .points
            .get(segment_index)
            .and_then(|point| point.outgoing.as_ref())
        {
            Some(TimeControlSegment::Bounce(bounce)) => {
                let start = self.points[segment_index].position;
                let end = self.points[segment_index + 1].position;
                let vertex = if bounce.reversed {
                    [1.0 - bounce.vertex[0], 1.0 - bounce.vertex[1]]
                } else {
                    bounce.vertex
                };
                Some([
                    start[0] + (end[0] - start[0]) * vertex[0],
                    start[1] + (end[1] - start[1]) * vertex[1],
                ])
            }
            _ => None,
        }
    }

    pub fn set_segment_vertex(&mut self, segment_index: usize, vertex: [f64; 2]) {
        if segment_index + 1 >= self.points.len() {
            return;
        }
        let start = self.points[segment_index].position;
        let end = self.points[segment_index + 1].position;
        let dx = end[0] - start[0];
        let dy = end[1] - start[1];
        let local_vertex = [
            if dx.abs() < f64::EPSILON {
                0.5
            } else {
                (vertex[0] - start[0]) / dx
            },
            if dy.abs() < f64::EPSILON {
                1.0
            } else {
                (vertex[1] - start[1]) / dy
            },
        ];
        if let Some(TimeControlSegment::Bounce(bounce)) = self
            .points
            .get_mut(segment_index)
            .and_then(|point| point.outgoing.as_mut())
        {
            let local_vertex = if bounce.reversed {
                [1.0 - local_vertex[0], 1.0 - local_vertex[1]]
            } else {
                local_vertex
            };
            bounce.vertex = [
                local_vertex[0].clamp(0.001, 0.999),
                local_vertex[1].clamp(0.0, 1.0),
            ];
        }
    }

    pub fn segment_elastic(&self, segment_index: usize) -> Option<&TimeControlElastic> {
        match self
            .points
            .get(segment_index)
            .and_then(|point| point.outgoing.as_ref())
        {
            Some(TimeControlSegment::Elastic(elastic)) => Some(elastic),
            _ => None,
        }
    }

    pub fn segment_elastic_mut(&mut self, segment_index: usize) -> Option<&mut TimeControlElastic> {
        match self
            .points
            .get_mut(segment_index)
            .and_then(|point| point.outgoing.as_mut())
        {
            Some(TimeControlSegment::Elastic(elastic)) => Some(elastic),
            _ => None,
        }
    }

    pub fn segment_reversed(&self, segment_index: usize) -> Option<bool> {
        match self
            .points
            .get(segment_index)
            .and_then(|point| point.outgoing.as_ref())
        {
            Some(TimeControlSegment::Elastic(elastic)) => Some(elastic.reversed),
            Some(TimeControlSegment::Bounce(bounce)) => Some(bounce.reversed),
            _ => None,
        }
    }

    pub fn set_segment_reversed(&mut self, segment_index: usize, reversed: bool) {
        match self
            .points
            .get_mut(segment_index)
            .and_then(|point| point.outgoing.as_mut())
        {
            Some(TimeControlSegment::Elastic(elastic)) => elastic.reversed = reversed,
            Some(TimeControlSegment::Bounce(bounce)) => bounce.reversed = reversed,
            _ => {}
        }
    }

    fn segment_y_at_x(&self, segment_index: usize, x: f64) -> f64 {
        if segment_index + 1 >= self.points.len() {
            return self
                .points
                .last()
                .map(|point| point.position[1])
                .unwrap_or(x);
        }
        let start = self.points[segment_index].position;
        let end = self.points[segment_index + 1].position;
        let local_x = Self::local_x(start, end, x);
        match self.points[segment_index]
            .outgoing
            .as_ref()
            .unwrap_or(&TimeControlSegment::Bezier)
        {
            TimeControlSegment::Bezier => self.bezier_segment_y_at_x(segment_index, x),
            TimeControlSegment::Elastic(elastic) => {
                start[1] + (end[1] - start[1]) * elastic.eased_y_at_x(local_x)
            }
            TimeControlSegment::Bounce(bounce) => {
                start[1] + (end[1] - start[1]) * bounce.eased_y_at_x(local_x)
            }
        }
    }

    fn bezier_segment_y_at_x(&self, segment_index: usize, x: f64) -> f64 {
        let x = x.clamp(0.0, 1.0);
        let mut min_t = 0.0;
        let mut max_t = 1.0;

        for _ in 0..32 {
            let t = (min_t + max_t) / 2.0;
            if self.segment_point_at(segment_index, t)[0] < x {
                min_t = t;
            } else {
                max_t = t;
            }
        }

        self.segment_point_at(segment_index, (min_t + max_t) / 2.0)[1]
    }

    pub fn segment_point_at(&self, segment_index: usize, t: f64) -> [f64; 2] {
        match self
            .points
            .get(segment_index)
            .and_then(|point| point.outgoing.as_ref())
        {
            Some(TimeControlSegment::Elastic(elastic)) => {
                self.function_segment_point_at(segment_index, t, |local_x| {
                    elastic.eased_y_at_x(local_x)
                })
            }
            Some(TimeControlSegment::Bounce(bounce)) => {
                self.function_segment_point_at(segment_index, t, |local_x| {
                    bounce.eased_y_at_x(local_x)
                })
            }
            _ => self.bezier_segment_point_at(segment_index, t),
        }
    }

    fn bezier_segment_point_at(&self, segment_index: usize, t: f64) -> [f64; 2] {
        let t = t.clamp(0.0, 1.0);
        let mt = 1.0 - t;
        let segment_index = segment_index.min(self.points.len().saturating_sub(2));
        let start = &self.points[segment_index];
        let end = &self.points[segment_index + 1];
        let p0 = start.position;
        let p1 = start.out_handle.unwrap_or(start.position);
        let p2 = end.in_handle.unwrap_or(end.position);
        let p3 = end.position;

        [
            mt.powi(3) * p0[0]
                + 3.0 * mt.powi(2) * t * p1[0]
                + 3.0 * mt * t.powi(2) * p2[0]
                + t.powi(3) * p3[0],
            mt.powi(3) * p0[1]
                + 3.0 * mt.powi(2) * t * p1[1]
                + 3.0 * mt * t.powi(2) * p2[1]
                + t.powi(3) * p3[1],
        ]
    }

    fn function_segment_point_at(
        &self,
        segment_index: usize,
        t: f64,
        y_at_x: impl Fn(f64) -> f64,
    ) -> [f64; 2] {
        let t = t.clamp(0.0, 1.0);
        let segment_index = segment_index.min(self.points.len().saturating_sub(2));
        let start = self.points[segment_index].position;
        let end = self.points[segment_index + 1].position;
        [
            start[0] + (end[0] - start[0]) * t,
            start[1] + (end[1] - start[1]) * y_at_x(t),
        ]
    }

    pub fn insert_midpoint(&mut self, after_index: usize) -> usize {
        let after_index = after_index.min(self.points.len().saturating_sub(2));
        let x =
            (self.points[after_index].position[0] + self.points[after_index + 1].position[0]) / 2.0;
        let y = self.y_at_x(x);
        let prev = self.points[after_index].position;
        let next = self.points[after_index + 1].position;
        let handle_delta = [(next[0] - prev[0]) / 6.0, (next[1] - prev[1]) / 6.0];
        let new_point = TimeControlPoint {
            position: [x, y],
            in_handle: Some([
                (x - handle_delta[0]).clamp(0.0, 1.0),
                (y - handle_delta[1]).clamp(0.0, 1.0),
            ]),
            out_handle: Some([
                (x + handle_delta[0]).clamp(0.0, 1.0),
                (y + handle_delta[1]).clamp(0.0, 1.0),
            ]),
            handles_separated: false,
            outgoing: Some(TimeControlSegment::Bezier),
        };
        let new_index = after_index + 1;
        self.points.insert(new_index, new_point);
        new_index
    }

    pub fn remove_midpoint(&mut self, index: usize) {
        if index == 0 || index + 1 >= self.points.len() {
            return;
        }
        self.points.remove(index);
    }

    fn segment_index_at_x(&self, x: f64) -> usize {
        if self.points.len() <= 1 {
            return 0;
        }
        self.points
            .windows(2)
            .position(|points| x <= points[1].position[0])
            .unwrap_or(self.points.len() - 2)
    }

    fn reset_segment_handles(&mut self, segment_index: usize) {
        if segment_index + 1 >= self.points.len() {
            return;
        }
        let start = self.points[segment_index].position;
        let end = self.points[segment_index + 1].position;
        self.points[segment_index].out_handle = Some([
            start[0] + (end[0] - start[0]) / 3.0,
            start[1] + (end[1] - start[1]) / 3.0,
        ]);
        self.points[segment_index + 1].in_handle = Some([
            end[0] - (end[0] - start[0]) / 3.0,
            end[1] - (end[1] - start[1]) / 3.0,
        ]);
    }

    fn local_x(start: [f64; 2], end: [f64; 2], x: f64) -> f64 {
        let dx = end[0] - start[0];
        if dx.abs() < f64::EPSILON {
            0.0
        } else {
            ((x - start[0]) / dx).clamp(0.0, 1.0)
        }
    }
}

impl TimeControlSegment {
    pub fn default_for_mode(mode: TimeControlMode) -> Self {
        match mode {
            TimeControlMode::Bezier => Self::Bezier,
            TimeControlMode::Elastic => Self::Elastic(TimeControlElastic::default()),
            TimeControlMode::Bounce => Self::Bounce(TimeControlBounce::default()),
        }
    }

    pub fn mode(&self) -> TimeControlMode {
        match self {
            Self::Bezier => TimeControlMode::Bezier,
            Self::Elastic(_) => TimeControlMode::Elastic,
            Self::Bounce(_) => TimeControlMode::Bounce,
        }
    }
}

impl TimeControlElastic {
    fn default_amplitude() -> f64 {
        1.0
    }

    fn default_frequency() -> f64 {
        5.0
    }

    fn default_decay() -> f64 {
        6.0
    }

    pub fn amp_handle(&self) -> [f64; 2] {
        [0.0, 1.0 + self.amplitude.clamp(0.0, 1.0)]
    }

    pub fn freq_decay_handle(&self) -> [f64; 2] {
        let frequency = self.frequency.max(0.5);
        let decay = self.decay.max(1.0);
        [
            (0.5 / frequency).clamp(0.0001, 1.0),
            (1.0 - (-(decay - 1.0) * 0.1).exp()).clamp(0.0, 0.9999),
        ]
    }

    pub fn set_amp_handle_y(&mut self, y: f64) {
        self.amplitude = (y - 1.0).clamp(0.0, 1.0);
    }

    pub fn set_freq_decay_handle(&mut self, point: [f64; 2]) {
        let x = point[0].clamp(0.0001, 1.0);
        let y = point[1].clamp(0.0, 0.9999);
        self.frequency = (0.5 / x).max(0.5);
        self.decay = -10.0 * (1.0 - y).ln() + 1.0;
    }

    fn eased_y_at_x(&self, x: f64) -> f64 {
        if self.reversed {
            1.0 - self.y_at_x(1.0 - x)
        } else {
            self.y_at_x(x)
        }
    }

    fn y_at_x(&self, x: f64) -> f64 {
        // https://github.com/mimaraka/aviutl-plugin-curve_editor/blob/main/curve_editor/curve_elastic.cpp
        let progress = x.clamp(0.0, 1.0);
        let amplitude = self.amplitude.clamp(0.0, 1.0);
        let frequency = self.frequency.max(0.5);
        let decay = self.decay.max(1.0);
        let omega = 2.0 * std::f64::consts::PI * frequency;
        let exp_k = (-decay).exp();

        let func_elastic = |progress: f64| {
            let coef = if decay == 0.0 {
                1.0 - progress
            } else {
                (exp_k.powf(progress) - exp_k) / (1.0 - exp_k)
            };
            1.0 - coef * (omega * progress).cos()
        };
        let func_elastic_derivative = |progress: f64| {
            let angle = omega * progress;
            let value_cos = angle.cos();
            let value_sin = angle.sin();
            if decay == 0.0 {
                value_cos + omega * (1.0 - progress) * value_sin
            } else {
                let exp_kt = exp_k.powf(progress);
                decay * exp_kt * value_cos + omega * (exp_kt - exp_k) * value_sin
            }
        };
        let func_elastic_derivative_2 = |progress: f64| {
            let angle = omega * progress;
            let value_cos = angle.cos();
            let value_sin = angle.sin();
            if decay == 0.0 {
                omega * omega * (1.0 - progress) * value_cos - 2.0 * omega * value_sin
            } else {
                let exp_kt = exp_k.powf(progress);
                let omega_sq = omega * omega;
                ((omega_sq - decay * decay) * exp_kt - omega_sq * exp_k) * value_cos
                    - 2.0 * omega * decay * exp_kt * value_sin
            }
        };

        let mut extremum_t = (0.5 - (decay / frequency).sqrt() * 0.05) / frequency;
        for _ in 0..3 {
            extremum_t -=
                func_elastic_derivative(extremum_t) / func_elastic_derivative_2(extremum_t);
        }
        let extremum_x = func_elastic(extremum_t);
        let value = func_elastic(progress);
        let ret = if progress < extremum_t {
            (amplitude * (extremum_x - 1.0) + 1.0) / extremum_x * value
        } else {
            amplitude * (value - 1.0) + 1.0
        };

        ret.clamp(0.0, 2.0)
    }
}

impl TimeControlBounce {
    fn eased_y_at_x(&self, x: f64) -> f64 {
        if self.reversed {
            1.0 - self.y_at_x(1.0 - x)
        } else {
            self.y_at_x(x)
        }
    }

    fn y_at_x(&self, x: f64) -> f64 {
        // https://github.com/mimaraka/aviutl-plugin-curve_editor/blob/main/curve_editor/curve_bounce.cpp
        let progress = x.clamp(0.0, 1.0);
        let handle_x = self.vertex[0].clamp(0.001, 0.999);
        let handle_y = self.vertex[1].clamp(0.001, 1.0);
        let cor = (1.0 - handle_y).sqrt().clamp(0.001, 0.999);
        let period = 2.0 * handle_x / (cor + 1.0);
        let limit_value = period * (1.0 / (1.0 - cor) - 0.5);

        let active = if limit_value > 1.0 {
            let n = ((1.0 + (cor - 1.0) * (1.0 / period + 0.5)).ln() / cor.ln()).floor();
            progress < period * ((cor.powf(n) - 1.0) / (cor - 1.0) - 0.5)
        } else {
            progress < limit_value
        };
        if !active {
            return 1.0;
        }

        let progress = progress / period;
        let bounce_index = |progress: f64| (((cor - 1.0) * progress + 1.0).ln() / cor.ln()).floor();
        let bounce_offset = |progress: f64| {
            progress + 0.5 + 1.0 / (cor - 1.0)
                - (cor + 1.0) * cor.powf(bounce_index(progress + 0.5)) / (2.0 * cor - 2.0)
        };
        let offset = bounce_offset(progress);
        let ret = 4.0 * offset * offset - cor.powf(2.0 * bounce_index(progress + 0.5));

        (1.0 + ret).clamp(0.0, 1.0)
    }
}

#[derive(Debug, Clone)]
pub struct TimeControlPreset {
    pub name: &'static str,
    pub timecontrol: TimeControl,
}

pub fn timecontrol_presets() -> Vec<TimeControlPreset> {
    #[derive(Clone, Copy)]
    enum PresetShape {
        Normal,
        Reverse,
        NormalReverse,
        ReverseNormal,
    }

    fn reverse_cubic_bezier(cubic_bezier: [f64; 4]) -> [f64; 4] {
        [
            1.0 - cubic_bezier[2],
            1.0 - cubic_bezier[3],
            1.0 - cubic_bezier[0],
            1.0 - cubic_bezier[1],
        ]
    }

    fn scaled_cubic_handles(
        cubic_bezier: [f64; 4],
        start: [f64; 2],
        end: [f64; 2],
    ) -> ([f64; 2], [f64; 2]) {
        let dx = end[0] - start[0];
        let dy = end[1] - start[1];
        (
            [
                start[0] + dx * cubic_bezier[0],
                start[1] + dy * cubic_bezier[1],
            ],
            [
                start[0] + dx * cubic_bezier[2],
                start[1] + dy * cubic_bezier[3],
            ],
        )
    }

    fn split_bezier(cubic_bezier: [f64; 4], first_reversed: bool) -> TimeControl {
        let first_cubic = if first_reversed {
            reverse_cubic_bezier(cubic_bezier)
        } else {
            cubic_bezier
        };
        let second_cubic = if first_reversed {
            cubic_bezier
        } else {
            reverse_cubic_bezier(cubic_bezier)
        };
        let (first_out, first_in) = scaled_cubic_handles(first_cubic, [0.0, 0.0], [0.5, 0.5]);
        let (second_out, second_in) = scaled_cubic_handles(second_cubic, [0.5, 0.5], [1.0, 1.0]);

        TimeControl {
            points: vec![
                TimeControlPoint {
                    position: [0.0, 0.0],
                    in_handle: None,
                    out_handle: Some(first_out),
                    handles_separated: false,
                    outgoing: Some(TimeControlSegment::Bezier),
                },
                TimeControlPoint {
                    position: [0.5, 0.5],
                    in_handle: Some(first_in),
                    out_handle: Some(second_out),
                    handles_separated: false,
                    outgoing: Some(TimeControlSegment::Bezier),
                },
                TimeControlPoint {
                    position: [1.0, 1.0],
                    in_handle: Some(second_in),
                    out_handle: None,
                    handles_separated: false,
                    outgoing: None,
                },
            ],
        }
    }

    fn bezier(name: &'static str, shape: PresetShape, cubic_bezier: [f64; 4]) -> TimeControlPreset {
        let timecontrol = match shape {
            PresetShape::Normal => TimeControl::from_cubic_bezier(cubic_bezier),
            PresetShape::Reverse => {
                TimeControl::from_cubic_bezier(reverse_cubic_bezier(cubic_bezier))
            }
            PresetShape::NormalReverse => split_bezier(cubic_bezier, false),
            PresetShape::ReverseNormal => split_bezier(cubic_bezier, true),
        };
        TimeControlPreset { name, timecontrol }
    }

    fn function_segment(mode: TimeControlMode, reversed: bool) -> TimeControlSegment {
        let mut segment = TimeControlSegment::default_for_mode(mode);
        match &mut segment {
            TimeControlSegment::Elastic(elastic) => elastic.reversed = reversed,
            TimeControlSegment::Bounce(bounce) => bounce.reversed = reversed,
            TimeControlSegment::Bezier => {}
        }
        segment
    }

    fn function_timecontrol(mode: TimeControlMode, shape: PresetShape) -> TimeControl {
        match shape {
            PresetShape::Normal | PresetShape::Reverse => {
                let mut timecontrol = TimeControl::default();
                timecontrol.points[0].outgoing = Some(function_segment(
                    mode,
                    matches!(shape, PresetShape::Reverse),
                ));
                timecontrol
            }
            PresetShape::NormalReverse | PresetShape::ReverseNormal => {
                let first_reversed = matches!(shape, PresetShape::ReverseNormal);
                TimeControl {
                    points: vec![
                        TimeControlPoint {
                            position: [0.0, 0.0],
                            in_handle: None,
                            out_handle: Some([1.0 / 6.0, 1.0 / 6.0]),
                            handles_separated: false,
                            outgoing: Some(function_segment(mode, first_reversed)),
                        },
                        TimeControlPoint {
                            position: [0.5, 0.5],
                            in_handle: Some([1.0 / 3.0, 1.0 / 3.0]),
                            out_handle: Some([2.0 / 3.0, 2.0 / 3.0]),
                            handles_separated: false,
                            outgoing: Some(function_segment(mode, !first_reversed)),
                        },
                        TimeControlPoint {
                            position: [1.0, 1.0],
                            in_handle: Some([5.0 / 6.0, 5.0 / 6.0]),
                            out_handle: None,
                            handles_separated: false,
                            outgoing: None,
                        },
                    ],
                }
            }
        }
    }

    fn elastic(name: &'static str, shape: PresetShape) -> TimeControlPreset {
        let timecontrol = function_timecontrol(TimeControlMode::Elastic, shape);
        TimeControlPreset { name, timecontrol }
    }

    fn bounce(name: &'static str, shape: PresetShape) -> TimeControlPreset {
        let timecontrol = function_timecontrol(TimeControlMode::Bounce, shape);
        TimeControlPreset { name, timecontrol }
    }

    vec![
        bezier("[1] linear", PresetShape::Normal, [0.25, 0.25, 0.75, 0.75]),
        bezier(
            "[2] easeInSine",
            PresetShape::Normal,
            [0.12, 0.0, 0.39, 0.0],
        ),
        bezier(
            "[3] easeOutSine",
            PresetShape::Reverse,
            [0.12, 0.0, 0.39, 0.0],
        ),
        bezier(
            "[4] easeInOutSine",
            PresetShape::NormalReverse,
            [0.12, 0.0, 0.39, 0.0],
        ),
        bezier(
            "[5] easeOutInSine",
            PresetShape::ReverseNormal,
            [0.12, 0.0, 0.39, 0.0],
        ),
        bezier("[6] easeInQuad", PresetShape::Normal, [0.11, 0.0, 0.5, 0.0]),
        bezier(
            "[7] easeOutQuad",
            PresetShape::Reverse,
            [0.11, 0.0, 0.5, 0.0],
        ),
        bezier(
            "[8] easeInOutQuad",
            PresetShape::NormalReverse,
            [0.11, 0.0, 0.5, 0.0],
        ),
        bezier(
            "[9] easeOutInQuad",
            PresetShape::ReverseNormal,
            [0.11, 0.0, 0.5, 0.0],
        ),
        bezier(
            "[10] easeInCubic",
            PresetShape::Normal,
            [0.32, 0.0, 0.67, 0.0],
        ),
        bezier(
            "[11] easeOutCubic",
            PresetShape::Reverse,
            [0.32, 0.0, 0.67, 0.0],
        ),
        bezier(
            "[12] easeInOutCubic",
            PresetShape::NormalReverse,
            [0.32, 0.0, 0.67, 0.0],
        ),
        bezier(
            "[13] easeOutInCubic",
            PresetShape::ReverseNormal,
            [0.32, 0.0, 0.67, 0.0],
        ),
        bezier(
            "[14] easeInQuart",
            PresetShape::Normal,
            [0.5, 0.0, 0.75, 0.0],
        ),
        bezier(
            "[15] easeOutQuart",
            PresetShape::Reverse,
            [0.5, 0.0, 0.75, 0.0],
        ),
        bezier(
            "[16] easeInOutQuart",
            PresetShape::NormalReverse,
            [0.5, 0.0, 0.75, 0.0],
        ),
        bezier(
            "[17] easeOutInQuart",
            PresetShape::ReverseNormal,
            [0.5, 0.0, 0.75, 0.0],
        ),
        bezier(
            "[18] easeInQuint",
            PresetShape::Normal,
            [0.64, 0.0, 0.78, 0.0],
        ),
        bezier(
            "[19] easeOutQuint",
            PresetShape::Reverse,
            [0.64, 0.0, 0.78, 0.0],
        ),
        bezier(
            "[20] easeInOutQuint",
            PresetShape::NormalReverse,
            [0.64, 0.0, 0.78, 0.0],
        ),
        bezier(
            "[21] easeOutInQuint",
            PresetShape::ReverseNormal,
            [0.64, 0.0, 0.78, 0.0],
        ),
        bezier(
            "[22] easeInExpo",
            PresetShape::Normal,
            [0.7, 0.0, 0.84, 0.0],
        ),
        bezier(
            "[23] easeOutExpo",
            PresetShape::Reverse,
            [0.7, 0.0, 0.84, 0.0],
        ),
        bezier(
            "[24] easeInOutExpo",
            PresetShape::NormalReverse,
            [0.7, 0.0, 0.84, 0.0],
        ),
        bezier(
            "[25] easeOutInExpo",
            PresetShape::ReverseNormal,
            [0.7, 0.0, 0.84, 0.0],
        ),
        bezier(
            "[26] easeInCirc",
            PresetShape::Normal,
            [0.55, 0.0, 1.0, 0.45],
        ),
        bezier(
            "[27] easeOutCirc",
            PresetShape::Reverse,
            [0.55, 0.0, 1.0, 0.45],
        ),
        bezier(
            "[28] easeInOutCirc",
            PresetShape::NormalReverse,
            [0.55, 0.0, 1.0, 0.45],
        ),
        bezier(
            "[29] easeOutInCirc",
            PresetShape::ReverseNormal,
            [0.55, 0.0, 1.0, 0.45],
        ),
        elastic("[30] easeInElastic", PresetShape::Reverse),
        elastic("[31] easeOutElastic", PresetShape::Normal),
        elastic("[32] easeInOutElastic", PresetShape::ReverseNormal),
        elastic("[33] easeOutInElastic", PresetShape::NormalReverse),
        bezier(
            "[34] easeInBack",
            PresetShape::Normal,
            [0.36, 0.0, 0.66, -0.56],
        ),
        bezier(
            "[35] easeOutBack",
            PresetShape::Reverse,
            [0.36, 0.0, 0.66, -0.56],
        ),
        bezier(
            "[36] easeInOutBack",
            PresetShape::NormalReverse,
            [0.36, 0.0, 0.66, -0.56],
        ),
        bezier(
            "[37] easeOutInBack",
            PresetShape::ReverseNormal,
            [0.36, 0.0, 0.66, -0.56],
        ),
        bounce("[38] easeInBounce", PresetShape::Reverse),
        bounce("[39] easeOutBounce", PresetShape::Normal),
        bounce("[40] easeInOutBounce", PresetShape::ReverseNormal),
        bounce("[41] easeOutInBounce", PresetShape::NormalReverse),
    ]
}

#[derive(Debug, Clone)]
pub struct Easing {
    pub name: String,
    pub script: String,
    pub script_bytes: Vec<u8>,
    pub label: Option<String>,
    pub path: Option<std::path::PathBuf>,
    pub has_speed: bool,
    pub default_acceleration: bool,
    pub default_deceleration: bool,
    pub has_timecontrol: bool,
    pub ignore_midpoints: bool,
    pub params: indexmap::IndexMap<String, f64>,
}

impl Easing {
    pub fn from_script(path: Option<std::path::PathBuf>, name: &str, script: &str) -> Easing {
        Self::from_script_with_bytes(path, name, script, script.as_bytes())
    }

    pub fn from_script_with_bytes(
        path: Option<std::path::PathBuf>,
        name: &str,
        script: &str,
        script_bytes: &[u8],
    ) -> Easing {
        let mut easing = Easing {
            name: name.to_string(),
            script: script.to_string(),
            script_bytes: script_bytes.to_vec(),
            label: None,
            path,
            has_speed: false,
            default_acceleration: false,
            default_deceleration: false,
            has_timecontrol: false,
            ignore_midpoints: false,
            params: indexmap::IndexMap::new(),
        };

        for line in script.lines() {
            if let Some((_, default_acceleration, default_deceleration)) =
                lazy_regex::regex_captures!(r"--speed:([01]),([01])", line.trim())
            {
                easing.has_speed = true;
                easing.default_acceleration = default_acceleration == "1";
                easing.default_deceleration = default_deceleration == "1";
            }
            if line.trim() == "--timecontrol" {
                easing.has_timecontrol = true;
            }
            if let Some((_, param_name, param_value)) =
                lazy_regex::regex_captures!(r"--param:([^,]+),(-?\d*\.?\d+)", line.trim())
            {
                let param_value: f64 = param_value.parse().unwrap_or(0.0);
                easing.params.insert(param_name.to_string(), param_value);
            }
            if let Some((_, param_captures)) =
                lazy_regex::regex_captures!(r"--param:(-?\d*\.?\d+)", line.trim())
            {
                let param_value: f64 = param_captures.parse().unwrap_or(0.0);
                easing.params.insert("設定値".to_string(), param_value);
            }
            if line.trim() == "--twopoint" {
                easing.ignore_midpoints = true;
            }
            if let Some(label) = line.strip_prefix("--label:") {
                easing.label = Some(label.trim().to_string());
            }
        }

        easing
    }

    pub fn from_multi_script(
        mut path: Option<std::path::PathBuf>,
        multi_script: &str,
    ) -> Vec<Easing> {
        Self::from_multi_script_with_bytes(path.take(), multi_script, multi_script.as_bytes())
    }

    pub fn from_multi_script_with_bytes(
        mut path: Option<std::path::PathBuf>,
        multi_script: &str,
        multi_script_bytes: &[u8],
    ) -> Vec<Easing> {
        let mut current_script: Option<(String, String)> = None;
        let mut current_script_bytes: Vec<u8> = Vec::new();
        let mut easings = Vec::new();
        for (line, line_bytes) in multi_script
            .lines()
            .zip(Self::script_byte_lines(multi_script_bytes))
        {
            if let Some(script_name) = line.strip_prefix("@") {
                if let Some((name, script)) = current_script.take() {
                    easings.push(Self::from_script_with_bytes(
                        path.clone(),
                        &name,
                        &script,
                        &current_script_bytes,
                    ));
                    current_script_bytes.clear();
                }
                let script_name = script_name.trim();
                if !script_name.is_empty() {
                    current_script = Some((script_name.to_string(), String::new()));
                }
            } else if let Some((_, script)) = &mut current_script {
                script.push_str(line);
                script.push('\n');
                current_script_bytes.extend_from_slice(line_bytes);
                current_script_bytes.push(b'\n');
            }
        }

        if let Some((name, script)) = current_script.take() {
            easings.push(Self::from_script_with_bytes(
                path.take(),
                &name,
                &script,
                &current_script_bytes,
            ));
        }

        easings
    }

    fn script_byte_lines(bytes: &[u8]) -> impl Iterator<Item = &[u8]> {
        bytes.split(|b| *b == b'\n').map(|line| {
            if let Some(line) = line.strip_suffix(b"\r") {
                line
            } else {
                line
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_timecontrol_bezier_maps_x_to_same_y() {
        let timecontrol = TimeControl::default();

        for x in [0.0, 0.25, 0.5, 0.75, 1.0] {
            assert!((timecontrol.y_at_x(x) - x).abs() < 0.000001);
        }
    }

    #[test]
    fn elastic_timecontrol_starts_and_ends_at_anchors() {
        let timecontrol = TimeControl::default_for_mode(TimeControlMode::Elastic);

        assert!((timecontrol.y_at_x(0.0) - 0.0).abs() < 0.000001);
        assert!((timecontrol.y_at_x(1.0) - 1.0).abs() < 0.000001);
    }

    #[test]
    fn elastic_handles_update_parameters() {
        let mut timecontrol = TimeControl::default_for_mode(TimeControlMode::Elastic);
        let elastic = timecontrol.segment_elastic_mut(0).unwrap();

        elastic.set_amp_handle_y(1.5);
        elastic.set_freq_decay_handle([0.25, 0.5]);

        assert!((elastic.amplitude - 0.5).abs() < 0.000001);
        assert!((elastic.frequency - 2.0).abs() < 0.000001);
        assert!((elastic.freq_decay_handle()[1] - 0.5).abs() < 0.000001);
    }

    #[test]
    fn bounce_timecontrol_starts_ends_and_passes_vertex() {
        let timecontrol = TimeControl::default_for_mode(TimeControlMode::Bounce);
        let vertex = timecontrol.editable_vertex().unwrap();

        assert!((timecontrol.y_at_x(0.0) - 0.0).abs() < 0.000001);
        assert!((timecontrol.y_at_x(1.0) - 1.0).abs() < 0.000001);
        assert!((timecontrol.y_at_x(vertex[0]) - vertex[1]).abs() < 0.000001);
    }

    #[test]
    fn mixed_timecontrol_segments_are_independent() {
        let mut timecontrol = TimeControl::default();
        timecontrol.insert_midpoint(0);
        timecontrol.points[1].position = [0.5, 0.5];
        timecontrol.set_segment_mode(0, TimeControlMode::Bounce);
        timecontrol.set_segment_mode(1, TimeControlMode::Elastic);
        timecontrol.set_segment_vertex(0, [0.25, 0.25]);
        timecontrol
            .segment_elastic_mut(1)
            .unwrap()
            .set_amp_handle_y(1.5);

        assert_eq!(timecontrol.segment_mode(0), Some(TimeControlMode::Bounce));
        assert_eq!(timecontrol.segment_mode(1), Some(TimeControlMode::Elastic));
        assert_eq!(timecontrol.editable_vertex().unwrap(), [0.25, 0.25]);
        assert!((timecontrol.y_at_x(0.25) - 0.25).abs() < 0.000001);
        assert!((timecontrol.y_at_x(0.5) - 0.5).abs() < 0.000001);
        assert!((timecontrol.y_at_x(1.0) - 1.0).abs() < 0.000001);
        assert!((timecontrol.segment_elastic(1).unwrap().amplitude - 0.5).abs() < 0.000001);
    }

    #[test]
    fn inserting_midpoint_keeps_existing_curve_on_left_and_makes_right_bezier() {
        let mut timecontrol = TimeControl::default_for_mode(TimeControlMode::Elastic);
        timecontrol.segment_elastic_mut(0).unwrap().reversed = true;
        timecontrol
            .segment_elastic_mut(0)
            .unwrap()
            .set_amp_handle_y(1.5);

        let new_index = timecontrol.insert_midpoint(0);

        assert_eq!(new_index, 1);
        assert_eq!(timecontrol.segment_mode(0), Some(TimeControlMode::Elastic));
        assert_eq!(timecontrol.segment_mode(1), Some(TimeControlMode::Bezier));
        assert!(timecontrol.segment_elastic(0).unwrap().reversed);
        assert!((timecontrol.segment_elastic(0).unwrap().amplitude - 0.5).abs() < 0.000001);
    }

    #[test]
    fn bounce_vertex_y_is_clamped_to_unit_range() {
        let mut timecontrol = TimeControl::default_for_mode(TimeControlMode::Bounce);

        timecontrol.set_editable_vertex([0.5, 1.5]);
        assert_eq!(timecontrol.editable_vertex().unwrap(), [0.5, 1.0]);

        timecontrol.set_editable_vertex([0.5, -0.5]);
        assert_eq!(timecontrol.editable_vertex().unwrap(), [0.5, 0.0]);
    }

    #[test]
    fn elastic_and_bounce_segments_can_be_reversed() {
        let normal_elastic = TimeControl::default_for_mode(TimeControlMode::Elastic);
        let mut reversed_elastic = normal_elastic.clone();
        reversed_elastic.set_segment_reversed(0, true);
        let normal_bounce = TimeControl::default_for_mode(TimeControlMode::Bounce);
        let mut reversed_bounce = normal_bounce.clone();
        reversed_bounce.set_segment_reversed(0, true);

        for x in [0.125, 0.25, 0.75] {
            assert!(
                (reversed_elastic.y_at_x(x) - (1.0 - normal_elastic.y_at_x(1.0 - x))).abs()
                    < 0.000001
            );
            assert!(
                (reversed_bounce.y_at_x(x) - (1.0 - normal_bounce.y_at_x(1.0 - x))).abs()
                    < 0.000001
            );
        }
    }

    #[test]
    fn split_timecontrol_presets_pass_through_midpoint() {
        let presets = timecontrol_presets();
        for name in [
            "[4] easeInOutSine",
            "[5] easeOutInSine",
            "[32] easeInOutElastic",
            "[33] easeOutInElastic",
            "[40] easeInOutBounce",
            "[41] easeOutInBounce",
        ] {
            let preset = presets
                .iter()
                .find(|preset| preset.name == name)
                .unwrap_or_else(|| panic!("preset not found: {name}"));
            assert!(
                (preset.timecontrol.y_at_x(0.0) - 0.0).abs() < 0.000001,
                "{name} at 0.0 = {}",
                preset.timecontrol.y_at_x(0.0)
            );
            assert!(
                (preset.timecontrol.y_at_x(0.5) - 0.5).abs() < 0.000001,
                "{name} at 0.5 = {}",
                preset.timecontrol.y_at_x(0.5)
            );
            assert!(
                (preset.timecontrol.y_at_x(1.0) - 1.0).abs() < 0.000001,
                "{name} at 1.0 = {}",
                preset.timecontrol.y_at_x(1.0)
            );
        }
    }

    #[test]
    fn remap_to_frames_clones_previous_easing_for_split_start() {
        let mut keyframes = Keyframes::new(3);
        keyframes.keyframes[0] = Keyframe::Easing(EasingKeyframeInfo {
            easing: "first".to_string(),
            ..Default::default()
        });
        keyframes.keyframes[2] = Keyframe::Easing(EasingKeyframeInfo {
            easing: "second".to_string(),
            ..Default::default()
        });

        let remapped = keyframes.remap_to_frames(&[10, 20, 30], &[15, 20]);

        let Keyframe::Easing(first) = &remapped.keyframes[0] else {
            panic!("first keyframe must be easing");
        };
        assert_eq!(first.easing, "first");
        assert!(matches!(remapped.keyframes[1], Keyframe::Midpoint));
    }

    #[test]
    fn remap_to_frames_preserves_exact_frame_keyframes() {
        let mut keyframes = Keyframes::new(4);
        keyframes.keyframes[1] = Keyframe::Ignored;
        keyframes.keyframes[2] = Keyframe::Easing(EasingKeyframeInfo {
            easing: "middle".to_string(),
            ..Default::default()
        });

        let remapped = keyframes.remap_to_frames(&[10, 20, 30, 40], &[10, 20, 30]);

        assert!(matches!(remapped.keyframes[1], Keyframe::Ignored));
        let Keyframe::Easing(third) = &remapped.keyframes[2] else {
            panic!("third keyframe must be easing");
        };
        assert_eq!(third.easing, "middle");
    }

    #[test]
    fn remap_to_frames_uses_midpoint_for_new_non_initial_frames() {
        let keyframes = Keyframes::new(2);

        let remapped = keyframes.remap_to_frames(&[10, 40], &[10, 20, 40]);

        assert!(matches!(remapped.keyframes[1], Keyframe::Midpoint));
    }
}
