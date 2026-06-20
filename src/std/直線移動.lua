--speed:0,0

local curves = obj.module("enhanced_tracks.aux2")

local index, ratio = math.modf(obj.getpoint("index"))
local num = obj.getpoint("num")
local values = {}
for i = 0, num - 1 do
	values[i + 1] = obj.getpoint(i)
end

return curves.linear_value(values, index, ratio, obj.getpoint("accelerate"), obj.getpoint("decelerate"))
