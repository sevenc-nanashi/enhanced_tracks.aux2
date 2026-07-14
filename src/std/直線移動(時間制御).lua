--speed:0,0
--timecontrol

local num = obj.getpoint("num")
assert(num > 0, "trackbar movement requires at least one point")

local values = {}
for i = 0, num - 1 do
  values[i + 1] = obj.getpoint(i)
end

if num == 1 then
  return values[1]
end

local position = obj.getpoint("timecontrol", "index")
local segment
local ratio
if position < 0.0 then
  segment = 0
  ratio = position
elseif position >= num - 1 then
  segment = num - 2
  ratio = position - segment
else
  segment, ratio = math.modf(position)
end

local start_value = values[segment + 1]
local end_value = values[segment + 2]
local accelerate = segment == 0 and obj.getpoint("accelerate")
local decelerate = segment == num - 2 and obj.getpoint("decelerate")
if not accelerate and not decelerate then
  return start_value + (end_value - start_value) * ratio
end

local average = (start_value + end_value) * 0.5
local previous_control = average
if accelerate then
  previous_control = start_value
end
local next_control = average
if decelerate then
  next_control = end_value
end
local reverse_ratio = 1.0 - ratio
return (previous_control * ratio * 3.0 + reverse_ratio * start_value) * reverse_ratio * reverse_ratio
  + (next_control * reverse_ratio * 3.0 + ratio * end_value) * ratio * ratio
