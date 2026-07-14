--speed:0,0
--twopoint

local num = obj.getpoint("num")
assert(num >= 2, "specified-distance movement requires two points")

local start_value = obj.getpoint(0)
local movement_amount = obj.getpoint(1)
local segment = math.min(math.floor(obj.getpoint("index")), num - 2)
local current_time = obj.getpoint("time")
local segment_time = obj.getpoint("time", segment)
local framerate = obj.getpoint("framerate")
assert(current_time ~= nil and segment_time ~= nil, "trackbar movement time information is unavailable")
assert(framerate ~= nil and framerate > 0.0, "trackbar movement framerate is unavailable")

local elapsed_frames = (current_time - segment_time) * framerate
return start_value + movement_amount * elapsed_frames
