--speed:0,0
--twopoint

local num = obj.getpoint("num")
local values = {}
for i = 0, num - 1 do
  values[i + 1] = obj.getpoint(i)
end

if #values == 0 then
  return 0.0
end
if #values == 1 then
  return values[1]
end

local t = num <= 1 and 0.0 or math.max(0.0, math.min(1.0, obj.getpoint("index") / (num - 1)))
local ok, timecontrol_value = pcall(obj.getpoint, "timecontrol", "value")
if ok and timecontrol_value then
  t = timecontrol_value
end

return values[1] + values[2] * t
