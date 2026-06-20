--speed:0,0
--timecontrol

local curves = obj.module("enhanced_tracks.aux2")

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

local t = num <= 1 and 0.0 or math.max(0.0, math.min(1.0, obj.getpoint("index") / (num - 1)))
local ok, timecontrol_value = pcall(obj.getpoint, "timecontrol", "value")
if ok and timecontrol_value then
	t = timecontrol_value
end

local axis1 = values
local axis2 = {}
local axis3 = {}
if linked_values then
	axis1 = linked_values[1]
	axis2 = linked_values[2] or {}
	axis3 = linked_values[3] or {}
end
local segment, local_t, lengths = curves.weighted_segment(axis1, axis2, axis3, t, obj.getpoint("accelerate"), obj.getpoint("decelerate"))

return curves.interpolation_value(
	values,
	lengths,
	segment,
	local_t,
	obj.getpoint("accelerate"),
	obj.getpoint("decelerate")
)
