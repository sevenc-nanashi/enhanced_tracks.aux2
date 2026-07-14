--speed:0,0

local function point_value(axis, point)
	local index = point + 1
	if index < 1 then
		return axis[1]
	end
	if index > #axis then
		return axis[#axis]
	end
	return axis[index]
end

local function section_frame_length(point, point_count, framerate)
	if point >= point_count - 1 then
		point = point_count - 2
	end
	local start_time = obj.getpoint("time", point)
	local end_time = obj.getpoint("time", point + 1)
	assert(start_time ~= nil and end_time ~= nil, "trackbar movement time information is unavailable")
	return (end_time - start_time) * framerate
end

local function interpolation_ratio(segment, ratio, point_count, framerate)
	local current_length = section_frame_length(segment, point_count, framerate)
	local previous_length = current_length
	if segment > 0 then
		previous_length = section_frame_length(segment - 1, point_count, framerate)
	end
	local next_length = current_length
	if segment < point_count - 2 then
		next_length = section_frame_length(segment + 1, point_count, framerate)
	end
	local previous_total = math.max(1.0, previous_length + current_length)
	local next_total = math.max(1.0, current_length + next_length)
	local reverse_ratio = 1.0 - ratio
	local previous_control = current_length / previous_total * 0.75 * ratio
	return ((1.0 - current_length / next_total * 0.75) * reverse_ratio * 3.0 + ratio) * ratio * ratio
		+ previous_control * 3.0 * reverse_ratio * reverse_ratio
end

local function point_distance(a, b)
	assert(#a == #b, "trackbar interpolation point dimensions must match")
	local square_sum = 0.0
	for axis_index = 1, #a do
		local difference = b[axis_index] - a[axis_index]
		square_sum = square_sum + difference * difference
	end
	return math.sqrt(square_sum)
end

local function standard_interpolation(ratio, p0, p1, p2, p3)
	local previous_length = point_distance(p0, p1)
	local current_length = point_distance(p1, p2)
	local next_length = point_distance(p2, p3)
	local previous_total = math.max(0.0001, previous_length + current_length)
	local next_total = math.max(0.0001, current_length + next_length)
	local reverse_ratio = 1.0 - ratio
	local values = {}
	for axis_index = 1, #p1 do
		local previous_control = p1[axis_index]
			+ (
				previous_length * (p2[axis_index] - p1[axis_index])
				+ current_length * (p1[axis_index] - p0[axis_index])
			) / previous_total / 3.0
		local next_control = p2[axis_index]
			- (
				current_length * (p3[axis_index] - p2[axis_index])
				+ next_length * (p2[axis_index] - p1[axis_index])
			) / next_total / 3.0
		values[axis_index] = p1[axis_index] * reverse_ratio * reverse_ratio * reverse_ratio
			+ previous_control * 3.0 * reverse_ratio * reverse_ratio * ratio
			+ next_control * 3.0 * reverse_ratio * ratio * ratio
			+ p2[axis_index] * ratio * ratio * ratio
	end
	return values
end

local function interpolation_value(axes, segment, ratio, framerate)
	local point_count = #axes[1]
	assert(point_count > 0, "trackbar movement requires at least one point")
	if point_count == 1 then
		return axes[1][1]
	end

	if segment >= point_count - 1 then
		segment = point_count - 2
		ratio = 1.0
	else
		segment = math.max(0, segment)
	end
	local is_first_edge = segment == 0
	local is_last_edge = segment == point_count - 2
	ratio = interpolation_ratio(segment, ratio, point_count, framerate)

	local p0 = {}
	local p1 = {}
	local p2 = {}
	local p3 = {}
	local accelerate = obj.getpoint("accelerate")
	local decelerate = obj.getpoint("decelerate")
	for axis_index, axis in ipairs(axes) do
		p0[axis_index] = point_value(axis, segment - 1)
		p1[axis_index] = point_value(axis, segment)
		p2[axis_index] = point_value(axis, segment + 1)
		p3[axis_index] = point_value(axis, segment + 2)
		local original_p0 = p0[axis_index]
		if is_first_edge and not accelerate then
			p0[axis_index] = p1[axis_index] * (5.0 / 3.0) - p2[axis_index] + p3[axis_index] / 3.0
		end
		if is_last_edge and not decelerate then
			p3[axis_index] = p2[axis_index] * (5.0 / 3.0) - p1[axis_index] + original_p0 / 3.0
		end
	end

	local values = standard_interpolation(ratio, p0, p1, p2, p3)
	if #axes == 1 then
		return values[1]
	end
	if #axes == 2 then
		return values[1], values[2]
	end
	assert(#axes == 3, "trackbar interpolation supports at most three linked axes")
	return values[1], values[2], values[3]
end

local segment, ratio = math.modf(obj.getpoint("index"))
local point_count = obj.getpoint("num")
local framerate = obj.getpoint("framerate")
assert(framerate ~= nil and framerate > 0.0, "trackbar movement framerate is unavailable")
local link_index, link_count = obj.getpoint("link")
assert(link_index ~= nil and link_count ~= nil, "trackbar movement link information is unavailable")

local axes = {}
for axis_index = 0, link_count - 1 do
	local axis = {}
	for point = 0, point_count - 1 do
		axis[point + 1] = obj.getpoint(point, axis_index - link_index)
	end
	axes[axis_index + 1] = axis
end

local values = { interpolation_value(axes, segment, ratio, framerate) }
return values[link_index + 1]
