local o_index, o_ratio = math.modf(obj.getpoint("index"))
local left = obj.getpoint(o_index)
local right = obj.getpoint(o_index + 1)
return left + (right - left) * o_ratio
