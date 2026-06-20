--speed:0,0

local curves = obj.module("enhanced_tracks.aux2")

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

local axis1 = values
local axis2 = {}
local axis3 = {}
if linked_values then
	axis1 = linked_values[1]
	axis2 = linked_values[2] or {}
	axis3 = linked_values[3] or {}
end
local lengths = curves.segment_lengths(axis1, axis2, axis3, obj.getpoint("accelerate"), obj.getpoint("decelerate"))

return curves.interpolation_value(values, lengths, index, ratio, obj.getpoint("accelerate"), obj.getpoint("decelerate"))
