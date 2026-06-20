--speed:0,0

---$embed
local curves = require("common")

local function smooth_edge_value(start_value, end_value, t, is_first_edge, is_last_edge)
	if not is_first_edge and not is_last_edge then
		return start_value + (end_value - start_value) * t
	end

	local average = (start_value + end_value) * 0.5
	local left = is_first_edge and start_value or (is_last_edge and average or start_value)
	local right = is_last_edge and end_value or (is_first_edge and average or end_value)
	local s = 1.0 - t

	return (left * t * 3.0 + s * start_value) * s * s + (right * s * 3.0 + t * end_value) * t * t
end

local function rotation_linear_value(values, segment, ratio, double_first, double_last)
	return curves.linear_value(curves.build_rotation_series(values, 360.0), segment, ratio, nil, double_first, double_last)
end

local function linear_rotate_group_value(axes, segment, ratio, axis_index, double_first, double_last)
	segment, ratio = curves.resolve_segment(#axes[1], segment, ratio, nil)
	local smooth_t = smooth_edge_value(
		0.0,
		1.0,
		ratio,
		double_first and segment == 0,
		double_last and segment == #axes[1] - 2
	)
	local q0 = curves.euler_quat_at(axes, segment + 1, "xyz")
	local q1 = curves.euler_quat_at(axes, segment + 2, "xyz")
	return curves.rotation_component_from_quat(axis_index, "xyz", curves.quat_slerp(q0, q1, smooth_t))
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

local axes = curves.rotation_axes(linked_values)
if axes then
	return linear_rotate_group_value(axes, index, ratio, link_index + 1, obj.getpoint("accelerate"), obj.getpoint("decelerate"))
end

return rotation_linear_value(values, index, ratio, obj.getpoint("accelerate"), obj.getpoint("decelerate"))
