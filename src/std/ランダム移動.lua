--speed:0,0

local curves = obj.module("enhanced_tracks.aux2")

local RAND_MAX = 2147483647

local function random_unit(seed, index)
  local frame = obj.frame or 0
  return obj.rand(0, RAND_MAX, seed + index * 65537, frame) / RAND_MAX
end

local function random_move_value(values, index, ratio)
  if #values == 0 then
    return 0.0
  end
  if #values == 1 then
    return values[1]
  end

  local segment, t = curves.resolve_segment(#values, index, ratio)
  local base = values[1]
  local span = values[#values] - base
  local seed = 0

  local function point(point_index)
    return base + random_unit(seed, point_index) * span
  end

  local p0 = point(segment)
  local p1 = point(segment + 1)
  local p2 = point(segment + 2)
  local p3 = point(segment + 3)

  return curves.catmull_rom(p0, p1, p2, p3, 1.0, 1.0, 1.0, t)
end

local index, ratio = math.modf(obj.getpoint("index"))
local num = obj.getpoint("num")
local values = {}
for i = 0, num - 1 do
  values[i + 1] = obj.getpoint(i)
end

return random_move_value(values, index, ratio)
