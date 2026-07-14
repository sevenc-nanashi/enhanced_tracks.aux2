--speed:0,0

local curves = obj.module("enhanced_tracks.aux2")

local EPS = 1.0e-12
local ROTATION_CONTROL_BLEND = 0.42500001192092896

local function atan2(y, x)
  if math.atan2 then
    return math.atan2(y, x)
  end
  return math.atan(y, x)
end

local function quat_normalize(quat)
  local length = math.sqrt(quat[1] * quat[1] + quat[2] * quat[2] + quat[3] * quat[3] + quat[4] * quat[4])
  if length <= EPS then
    return { 0.0, 0.0, 0.0, 1.0 }
  end
  return {
    quat[1] / length,
    quat[2] / length,
    quat[3] / length,
    quat[4] / length,
  }
end

local function quat_mul(a, b)
  return {
    a[4] * b[1] + a[1] * b[4] + a[2] * b[3] - a[3] * b[2],
    a[4] * b[2] - a[1] * b[3] + a[2] * b[4] + a[3] * b[1],
    a[4] * b[3] + a[1] * b[2] - a[2] * b[1] + a[3] * b[4],
    a[4] * b[4] - a[1] * b[1] - a[2] * b[2] - a[3] * b[3],
  }
end

local function quat_add(a, b)
  return {
    a[1] + b[1],
    a[2] + b[2],
    a[3] + b[3],
    a[4] + b[4],
  }
end

local function quat_scale(quat, scale)
  return {
    quat[1] * scale,
    quat[2] * scale,
    quat[3] * scale,
    quat[4] * scale,
  }
end

local function quat_align(reference, quat)
  if reference[1] * quat[1] + reference[2] * quat[2] + reference[3] * quat[3] + reference[4] * quat[4] < 0.0 then
    return { -quat[1], -quat[2], -quat[3], -quat[4] }
  end
  return quat
end

local function quat_inverse(quat)
  quat = quat_normalize(quat)
  return { -quat[1], -quat[2], -quat[3], quat[4] }
end

local function quat_log(quat)
  quat = quat_normalize(quat)
  local v_len = math.sqrt(quat[1] * quat[1] + quat[2] * quat[2] + quat[3] * quat[3])
  if v_len <= EPS then
    return { 0.0, 0.0, 0.0, 0.0 }
  end
  local angle = atan2(v_len, quat[4])
  local scale = angle / v_len
  return { quat[1] * scale, quat[2] * scale, quat[3] * scale, 0.0 }
end

local function quat_exp(quat)
  local v_len = math.sqrt(quat[1] * quat[1] + quat[2] * quat[2] + quat[3] * quat[3])
  local s = math.sin(v_len)
  if v_len <= EPS then
    return { quat[1], quat[2], quat[3], math.cos(v_len) }
  end
  local scale = s / v_len
  return {
    quat[1] * scale,
    quat[2] * scale,
    quat[3] * scale,
    math.cos(v_len),
  }
end

local function quat_squad_control(prev_q, cur_q, next_q)
  local inv_cur = quat_inverse(cur_q)
  local log1 = quat_log(quat_mul(inv_cur, prev_q))
  local log2 = quat_log(quat_mul(inv_cur, next_q))
  local blend = quat_scale(quat_add(log1, log2), -0.25)
  return quat_normalize(quat_mul(cur_q, quat_exp(blend)))
end

local function rotation_interpolation_value(values, lengths, segment, ratio, double_first, double_last)
  return curves.interpolation_value(
    curves.build_rotation_series(values, 360.0),
    lengths,
    segment,
    ratio,
    double_first,
    double_last
  )
end

local function interpolation_rotate_group_value(axes, segment, ratio, axis_index)
  segment, ratio = curves.resolve_segment(#axes[1], segment, ratio)
  local q_prev = curves.euler_quat_at(axes[1], axes[2], axes[3], segment, "xyz")
  local q_cur = curves.euler_quat_at(axes[1], axes[2], axes[3], segment + 1, "xyz")
  local q_next = quat_align(q_cur, curves.euler_quat_at(axes[1], axes[2], axes[3], segment + 2, "xyz"))
  q_prev = quat_align(q_cur, q_prev)
  local q_after = quat_align(q_next, curves.euler_quat_at(axes[1], axes[2], axes[3], segment + 3, "xyz"))

  local control_cur = quat_squad_control(q_prev, q_cur, q_next)
  local control_next = quat_squad_control(q_cur, q_next, q_after)
  local curve_cur = curves.quat_slerp(q_cur, control_cur, ROTATION_CONTROL_BLEND)
  local curve_next = curves.quat_slerp(q_next, control_next, ROTATION_CONTROL_BLEND)
  local curve = curves.quat_slerp(curve_cur, curve_next, ratio)
  local linear = curves.quat_slerp(q_cur, q_next, ratio)
  local blend = 4.0 * ratio * (1.0 - ratio)
  return curves.rotation_component_from_quat(axis_index, "xyz", curves.quat_slerp(linear, curve, blend))
end

local index, ratio = math.modf(obj.getpoint("index"))
local num = obj.getpoint("num")
local values = {}
for i = 0, num - 1 do
  values[i + 1] = obj.getpoint(i)
end

local link_index, link_count = obj.getpoint("link")
link_index = link_index or 0
link_count = link_count or 1

local linked_values = nil
if link_count > 1 then
  linked_values = {}
  for axis = 0, link_count - 1 do
    local axis_values = {}
    for i = 0, num - 1 do
      axis_values[i + 1] = obj.getpoint(i, axis - link_index)
    end
    linked_values[axis + 1] = axis_values
  end
end

local rotation_group = nil
if linked_values and #linked_values == 3 then
  rotation_group = linked_values
end
if rotation_group then
  return interpolation_rotate_group_value(rotation_group, index, ratio, link_index + 1)
end

local axis1 = values
local axis2 = {}
local axis3 = {}
if linked_values then
  axis1 = linked_values[1]
  axis2 = linked_values[2] or {}
  axis3 = linked_values[3] or {}
end
local lengths = curves.segment_lengths(axis1, axis2, axis3, obj.getpoint("accelerate"), obj.getpoint("decelerate"))
return rotation_interpolation_value(
  values,
  lengths,
  index,
  ratio,
  obj.getpoint("accelerate"),
  obj.getpoint("decelerate")
)
