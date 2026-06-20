local M = {}

local EPS = 1.0e-12
local HALF = 0.5

local function clamp(value, min_value, max_value)
	if value < min_value then
		return min_value
	end
	if value > max_value then
		return max_value
	end
	return value
end

local function smooth_edge_value(start_value, end_value, t, is_first_edge, is_last_edge)
	if not is_first_edge and not is_last_edge then
		return start_value + (end_value - start_value) * t
	end

	local average = (start_value + end_value) * HALF
	local left = is_first_edge and start_value or (is_last_edge and average or start_value)
	local right = is_last_edge and end_value or (is_first_edge and average or end_value)
	local s = 1.0 - t

	return (left * t * 3.0 + s * start_value) * s * s + (right * s * 3.0 + t * end_value) * t * t
end

local function resolve_segment(point_count, segment, local_t, t)
	if point_count <= 1 then
		return 0, 0.0
	end

	if segment ~= nil then
		if segment >= point_count - 1 then
			return point_count - 2, 1.0
		end
		return clamp(segment, 0, point_count - 2), clamp(local_t or t or 0.0, 0.0, 1.0)
	end

	t = clamp(t or 0.0, 0.0, 1.0)
	local scaled = t * (point_count - 1)
	segment = math.min(point_count - 2, math.floor(scaled))
	return segment, scaled - segment
end

local function deg_to_rad(value)
	return value * math.pi / 180.0
end

local function rad_to_deg(value)
	return value * 180.0 / math.pi
end

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

local function quat_dot(a, b)
	return a[1] * b[1] + a[2] * b[2] + a[3] * b[3] + a[4] * b[4]
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

local function quat_negate(quat)
	return { -quat[1], -quat[2], -quat[3], -quat[4] }
end

local function axis_angle_quat(axis, angle_rad)
	local half = angle_rad * HALF
	local s = math.sin(half)
	if axis == "x" then
		return { s, 0.0, 0.0, math.cos(half) }
	end
	if axis == "y" then
		return { 0.0, s, 0.0, math.cos(half) }
	end
	return { 0.0, 0.0, s, math.cos(half) }
end

local function quat_from_euler_xyz(rx, ry, rz, order)
	order = order or "xyz"
	local quats = {
		x = axis_angle_quat("x", deg_to_rad(rx)),
		y = axis_angle_quat("y", deg_to_rad(ry)),
		z = axis_angle_quat("z", deg_to_rad(rz)),
	}

	local result = { 0.0, 0.0, 0.0, 1.0 }
	for i = #order, 1, -1 do
		local axis = order:sub(i, i)
		result = quat_mul(quats[axis], result)
	end
	return quat_normalize(result)
end

local function quat_to_euler_xyz(quat, order)
	order = order or "xyz"
	quat = quat_normalize(quat)

	if order ~= "xyz" then
		order = "xyz"
	end

	local x, y, z, w = quat[1], quat[2], quat[3], quat[4]

	local sinr_cosp = 2.0 * (w * x - y * z)
	local cosr_cosp = 1.0 - 2.0 * (x * x + y * y)
	local rx = atan2(sinr_cosp, cosr_cosp)

	local sinp = 2.0 * (w * y + z * x)
	local ry
	if math.abs(sinp) >= 1.0 then
		ry = (sinp >= 0.0 and 1.0 or -1.0) * (math.pi * HALF)
	else
		ry = math.asin(sinp)
	end

	local siny_cosp = 2.0 * (w * z - x * y)
	local cosy_cosp = 1.0 - 2.0 * (y * y + z * z)
	local rz = atan2(siny_cosp, cosy_cosp)

	return {
		rad_to_deg(rx),
		rad_to_deg(ry),
		rad_to_deg(rz),
	}
end

local function quat_slerp(a, b, t)
	local dot = quat_dot(a, b)
	if dot < 0.0 then
		b = quat_negate(b)
		dot = -dot
	end

	if dot > 0.9995 then
		return quat_normalize(quat_add(quat_scale(a, 1.0 - t), quat_scale(b, t)))
	end

	local theta_0 = math.acos(clamp(dot, -1.0, 1.0))
	local theta = theta_0 * t
	local sin_theta = math.sin(theta)
	local sin_theta_0 = math.sin(theta_0)

	local s0 = math.cos(theta) - dot * sin_theta / sin_theta_0
	local s1 = sin_theta / sin_theta_0
	return quat_add(quat_scale(a, s0), quat_scale(b, s1))
end

local function build_rotation_series(values, period)
	if #values == 0 then
		return {}
	end

	local out = { values[1] }
	for i = 2, #values do
		local delta = (values[i] - out[i - 1]) % period
		if delta > period * HALF then
			delta = delta - period
		end
		out[i] = out[i - 1] + delta
	end
	return out
end

local function collect_axes(values, linked_values)
	if linked_values then
		local axes = {}
		for _, axis_values in ipairs(linked_values) do
			axes[#axes + 1] = axis_values
		end
		if #axes > 0 then
			return axes
		end
	end

	return { values or {} }
end

local function rotation_axes(linked_values)
	if not linked_values or #linked_values ~= 3 then
		return nil
	end
	return {
		linked_values[1],
		linked_values[2],
		linked_values[3],
	}
end

local function euler_quat_at(axes, index, order)
	local max_index = #axes[1]
	local i = clamp(index, 1, max_index)
	return quat_from_euler_xyz(axes[1][i], axes[2][i], axes[3][i], order)
end

local function rotation_component_from_quat(axis_index, rotation_order, quat)
	local euler = quat_to_euler_xyz(quat, rotation_order or "xyz")
	return euler[clamp(axis_index or 1, 1, 3)]
end

local function segment_lengths(axes, double_first, double_last)
	local point_count = #axes[1]
	if point_count <= 1 then
		return {}
	end

	local lengths = {}
	for i = 1, point_count - 1 do
		local sum = 0.0
		for _, axis in ipairs(axes) do
			local delta = axis[i + 1] - axis[i]
			sum = sum + delta * delta
		end
		lengths[i] = math.sqrt(sum)
	end

	if #lengths > 0 then
		if double_first then
			lengths[1] = lengths[1] * 2.0
		end
		if double_last then
			lengths[#lengths] = lengths[#lengths] * 2.0
		end
	end

	return lengths
end

local function weighted_segment(axes, t, double_first, double_last)
	local lengths = segment_lengths(axes, double_first, double_last)
	if #lengths == 0 then
		return 0, clamp(t or 0.0, 0.0, 1.0), lengths
	end

	local total = 0.0
	for i = 1, #lengths do
		total = total + lengths[i]
	end
	if total <= EPS then
		return resolve_segment(#axes[1], nil, nil, t), lengths
	end

	local rest = clamp(t or 0.0, 0.0, 1.0) * total
	for i = 1, #lengths do
		if rest <= lengths[i] then
			return i - 1, lengths[i] <= EPS and 0.0 or rest / lengths[i], lengths
		end
		rest = rest - lengths[i]
	end

	return #lengths - 1, 1.0, lengths
end

local function catmull_rom(start_prev, start_value, end_value, end_next, len_prev, len_cur, len_next, t)
	len_prev = math.max(len_prev or 1.0, EPS)
	len_cur = math.max(len_cur or 1.0, EPS)
	len_next = math.max(len_next or 1.0, EPS)

	local m0 = ((end_value - start_prev) / (len_prev + len_cur)) * len_cur * HALF
	local m1 = ((end_next - start_value) / (len_cur + len_next)) * len_cur * HALF
	local s = 1.0 - t

	return ((start_value + m0) * t * 3.0 + s * start_value) * s * s
		+ ((end_value - m1) * s * 3.0 + t * end_value) * t * t
end

local function linear_value(values, segment, local_t, t, double_first, double_last)
	segment, t = resolve_segment(#values, segment, local_t, t)
	if #values == 0 then
		return 0.0
	end
	if #values == 1 then
		return values[1]
	end

	return smooth_edge_value(
		values[segment + 1],
		values[segment + 2],
		t,
		double_first and segment == 0,
		double_last and segment == #values - 2
	)
end

local function interpolation_value(values, lengths, segment, local_t, t, double_first, double_last)
	segment, t = resolve_segment(#values, segment, local_t, t)

	if #values == 0 then
		return 0.0
	end
	if #values == 1 then
		return values[1]
	end

	local i = segment + 1
	local p0 = values[i]
	local p1 = values[i + 1]
	local pm1 = values[math.max(i - 1, 1)]
	local p2 = values[math.min(i + 2, #values)]

	if i == 1 and not double_first then
		pm1 = (2.0 * p0 - p1) + HALF * p2
	end
	if i == #values - 1 and not double_last then
		p2 = (2.0 * p1 - p0) + HALF * pm1
	end

	local len_prev = lengths and lengths[math.max(i - 1, 1)] or 1.0
	local len_cur = lengths and lengths[i] or 1.0
	local len_next = lengths and lengths[math.min(i + 1, #lengths)] or len_cur

	return catmull_rom(pm1, p0, p1, p2, len_prev, len_cur, len_next, t)
end

M.build_rotation_series = build_rotation_series
M.catmull_rom = catmull_rom
M.collect_axes = collect_axes
M.euler_quat_at = euler_quat_at
M.interpolation_value = interpolation_value
M.linear_value = linear_value
M.quat_slerp = quat_slerp
M.resolve_segment = resolve_segment
M.rotation_component_from_quat = rotation_component_from_quat
M.rotation_axes = rotation_axes
M.segment_lengths = segment_lengths
M.weighted_segment = weighted_segment

return M
