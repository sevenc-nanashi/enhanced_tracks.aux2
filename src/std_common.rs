const EPS: f64 = 1.0e-12;
const HALF: f64 = 0.5;

fn clamp(value: f64, min_value: f64, max_value: f64) -> f64 {
    if value < min_value {
        return min_value;
    }
    if value > max_value {
        return max_value;
    }
    value
}

pub fn resolve_segment(point_count: usize, segment: f64, local_t: f64) -> (i32, f64) {
    if point_count <= 1 {
        return (0, 0.0);
    }

    if segment >= (point_count - 1) as f64 {
        return ((point_count - 2) as i32, 1.0);
    }
    (
        clamp(segment, 0.0, (point_count - 2) as f64) as i32,
        clamp(local_t, 0.0, 1.0),
    )
}

fn smooth_edge_value(
    start_value: f64,
    end_value: f64,
    t: f64,
    is_first_edge: bool,
    is_last_edge: bool,
) -> f64 {
    if !is_first_edge && !is_last_edge {
        return start_value + (end_value - start_value) * t;
    }

    let average = (start_value + end_value) * HALF;
    let left = if is_first_edge {
        start_value
    } else if is_last_edge {
        average
    } else {
        start_value
    };
    let right = if is_last_edge {
        end_value
    } else if is_first_edge {
        average
    } else {
        end_value
    };
    let s = 1.0 - t;

    (left * t * 3.0 + s * start_value) * s * s + (right * s * 3.0 + t * end_value) * t * t
}

pub fn linear_value(
    values: &[f64],
    segment: f64,
    local_t: f64,
    double_first: bool,
    double_last: bool,
) -> f64 {
    let (segment, t) = resolve_segment(values.len(), segment, local_t);
    if values.is_empty() {
        return 0.0;
    }
    if values.len() == 1 {
        return values[0];
    }

    let segment = segment as usize;
    smooth_edge_value(
        values[segment],
        values[segment + 1],
        t,
        double_first && segment == 0,
        double_last && segment == values.len() - 2,
    )
}

fn axis_count(axis1: &[f64], axis2: &[f64], axis3: &[f64]) -> usize {
    [axis1.len(), axis2.len(), axis3.len()]
        .into_iter()
        .filter(|len| *len > 0)
        .min()
        .unwrap_or(0)
}

pub fn segment_lengths(
    axis1: &[f64],
    axis2: &[f64],
    axis3: &[f64],
    double_first: bool,
    double_last: bool,
) -> Vec<f64> {
    let point_count = axis_count(axis1, axis2, axis3);
    if point_count <= 1 {
        return vec![];
    }

    let mut lengths = Vec::with_capacity(point_count - 1);
    for i in 0..point_count - 1 {
        let mut sum = 0.0;
        for axis in [axis1, axis2, axis3] {
            if axis.is_empty() {
                continue;
            }
            let delta = axis[i + 1] - axis[i];
            sum += delta * delta;
        }
        lengths.push(sum.sqrt());
    }

    if !lengths.is_empty() {
        if double_first {
            lengths[0] *= 2.0;
        }
        if double_last {
            let last = lengths.len() - 1;
            lengths[last] *= 2.0;
        }
    }

    lengths
}

pub fn weighted_segment(
    axis1: &[f64],
    axis2: &[f64],
    axis3: &[f64],
    t: f64,
    double_first: bool,
    double_last: bool,
) -> (i32, f64, Vec<f64>) {
    let lengths = segment_lengths(axis1, axis2, axis3, double_first, double_last);
    if lengths.is_empty() {
        let (segment, local_t) = resolve_segment(axis_count(axis1, axis2, axis3), 0.0, t);
        return (segment, local_t, lengths);
    }

    let total: f64 = lengths.iter().sum();
    if total <= EPS {
        let (segment, local_t) = resolve_segment(axis_count(axis1, axis2, axis3), 0.0, t);
        return (segment, local_t, lengths);
    }

    let mut rest = clamp(t, 0.0, 1.0) * total;
    for (i, length) in lengths.iter().enumerate() {
        if rest <= *length {
            return (
                i as i32,
                if *length <= EPS { 0.0 } else { rest / *length },
                lengths,
            );
        }
        rest -= *length;
    }

    ((lengths.len() - 1) as i32, 1.0, lengths)
}

#[expect(clippy::too_many_arguments)]
pub fn catmull_rom(
    start_prev: f64,
    start_value: f64,
    end_value: f64,
    end_next: f64,
    len_prev: f64,
    len_cur: f64,
    len_next: f64,
    t: f64,
) -> f64 {
    let len_prev = len_prev.max(EPS);
    let len_cur = len_cur.max(EPS);
    let len_next = len_next.max(EPS);

    let m0 = ((end_value - start_prev) / (len_prev + len_cur)) * len_cur * HALF;
    let m1 = ((end_next - start_value) / (len_cur + len_next)) * len_cur * HALF;
    let s = 1.0 - t;

    ((start_value + m0) * t * 3.0 + s * start_value) * s * s
        + ((end_value - m1) * s * 3.0 + t * end_value) * t * t
}

pub fn interpolation_value(
    values: &[f64],
    lengths: &[f64],
    segment: f64,
    local_t: f64,
    double_first: bool,
    double_last: bool,
) -> f64 {
    let (segment, t) = resolve_segment(values.len(), segment, local_t);

    if values.is_empty() {
        return 0.0;
    }
    if values.len() == 1 {
        return values[0];
    }

    let i = segment as usize;
    let p0 = values[i];
    let p1 = values[i + 1];
    let mut pm1 = values[i.saturating_sub(1)];
    let mut p2 = values[(i + 2).min(values.len() - 1)];

    if i == 0 && !double_first {
        pm1 = (2.0 * p0 - p1) + HALF * p2;
    }
    if i == values.len() - 2 && !double_last {
        p2 = (2.0 * p1 - p0) + HALF * pm1;
    }

    let len_prev = lengths.get(i.saturating_sub(1)).copied().unwrap_or(1.0);
    let len_cur = lengths.get(i).copied().unwrap_or(1.0);
    let len_next = lengths
        .get((i + 1).min(lengths.len().saturating_sub(1)))
        .copied()
        .unwrap_or(len_cur);

    catmull_rom(pm1, p0, p1, p2, len_prev, len_cur, len_next, t)
}

pub fn build_rotation_series(values: &[f64], period: f64) -> Vec<f64> {
    if values.is_empty() {
        return vec![];
    }

    let mut out = vec![values[0]];
    for value in values.iter().skip(1) {
        let mut delta = (value - out[out.len() - 1]).rem_euclid(period);
        if delta > period * HALF {
            delta -= period;
        }
        out.push(out[out.len() - 1] + delta);
    }
    out
}

fn quat_normalize(quat: [f64; 4]) -> [f64; 4] {
    let length =
        (quat[0] * quat[0] + quat[1] * quat[1] + quat[2] * quat[2] + quat[3] * quat[3]).sqrt();
    if length <= EPS {
        return [0.0, 0.0, 0.0, 1.0];
    }
    [
        quat[0] / length,
        quat[1] / length,
        quat[2] / length,
        quat[3] / length,
    ]
}

fn quat_dot(a: [f64; 4], b: [f64; 4]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3]
}

fn quat_mul(a: [f64; 4], b: [f64; 4]) -> [f64; 4] {
    [
        a[3] * b[0] + a[0] * b[3] + a[1] * b[2] - a[2] * b[1],
        a[3] * b[1] - a[0] * b[2] + a[1] * b[3] + a[2] * b[0],
        a[3] * b[2] + a[0] * b[1] - a[1] * b[0] + a[2] * b[3],
        a[3] * b[3] - a[0] * b[0] - a[1] * b[1] - a[2] * b[2],
    ]
}

fn quat_from_euler_xyz(rx: f64, ry: f64, rz: f64, order: &str) -> [f64; 4] {
    fn axis_angle_quat(axis: char, angle_rad: f64) -> [f64; 4] {
        let half = angle_rad * HALF;
        let s = half.sin();
        match axis {
            'x' => [s, 0.0, 0.0, half.cos()],
            'y' => [0.0, s, 0.0, half.cos()],
            _ => [0.0, 0.0, s, half.cos()],
        }
    }

    let x = axis_angle_quat('x', rx.to_radians());
    let y = axis_angle_quat('y', ry.to_radians());
    let z = axis_angle_quat('z', rz.to_radians());

    let mut result = [0.0, 0.0, 0.0, 1.0];
    for axis in order.chars().rev() {
        let quat = match axis {
            'x' => x,
            'y' => y,
            _ => z,
        };
        result = quat_mul(quat, result);
    }
    quat_normalize(result)
}

fn quat_to_euler_xyz(quat: [f64; 4]) -> [f64; 3] {
    let quat = quat_normalize(quat);
    let [x, y, z, w] = quat;

    let sinr_cosp = 2.0 * (w * x - y * z);
    let cosr_cosp = 1.0 - 2.0 * (x * x + y * y);
    let rx = sinr_cosp.atan2(cosr_cosp);

    let sinp = 2.0 * (w * y + z * x);
    let ry = if sinp.abs() >= 1.0 {
        sinp.signum() * (std::f64::consts::PI * HALF)
    } else {
        sinp.asin()
    };

    let siny_cosp = 2.0 * (w * z - x * y);
    let cosy_cosp = 1.0 - 2.0 * (y * y + z * z);
    let rz = siny_cosp.atan2(cosy_cosp);

    [rx.to_degrees(), ry.to_degrees(), rz.to_degrees()]
}

pub fn quat_slerp(mut a: [f64; 4], mut b: [f64; 4], t: f64) -> [f64; 4] {
    let mut dot = quat_dot(a, b);
    if dot < 0.0 {
        b = [-b[0], -b[1], -b[2], -b[3]];
        dot = -dot;
    }

    if dot > 0.9995 {
        return quat_normalize([
            a[0] * (1.0 - t) + b[0] * t,
            a[1] * (1.0 - t) + b[1] * t,
            a[2] * (1.0 - t) + b[2] * t,
            a[3] * (1.0 - t) + b[3] * t,
        ]);
    }

    let theta_0 = clamp(dot, -1.0, 1.0).acos();
    let theta = theta_0 * t;
    let sin_theta = theta.sin();
    let sin_theta_0 = theta_0.sin();
    let s0 = theta.cos() - dot * sin_theta / sin_theta_0;
    let s1 = sin_theta / sin_theta_0;

    a = [a[0] * s0, a[1] * s0, a[2] * s0, a[3] * s0];
    b = [b[0] * s1, b[1] * s1, b[2] * s1, b[3] * s1];
    [a[0] + b[0], a[1] + b[1], a[2] + b[2], a[3] + b[3]]
}

pub fn euler_quat_at(
    axis1: &[f64],
    axis2: &[f64],
    axis3: &[f64],
    index: f64,
    order: &str,
) -> [f64; 4] {
    let point_count = axis_count(axis1, axis2, axis3);
    assert!(point_count > 0, "axes must not be empty");
    let index = clamp(index, 1.0, point_count as f64) as usize - 1;
    quat_from_euler_xyz(axis1[index], axis2[index], axis3[index], order)
}

pub fn rotation_component_from_quat(axis_index: i32, quat: [f64; 4]) -> f64 {
    assert!(
        (1..=3).contains(&axis_index),
        "axis_index must be between 1 and 3"
    );
    let euler = quat_to_euler_xyz(quat);
    euler[(axis_index - 1) as usize]
}
