--param:Bank ID (Do not edit these parameters),0
--param:Keyframe ID,0
--param:Scene ID,0
--param:Project Session Nonce,0

local mod = obj.module("enhanced_tracks.aux2")
local ffi = require("ffi")
local bank_id, keyframe_id, scene_id, project_session_nonce = obj.getpoint("param")
local index, ratio = math.modf(obj.getpoint("index"))
local inspect = mod.debug_mode()

if bank_id == 0 then
  if inspect then
    print("== Keyframe Track Debug Info ==")
    print("Bank ID is 0, falling back to linear track")
  end
  local left = obj.getpoint(index)
  local right = obj.getpoint(index + 1)
  return left + (right - left) * ratio
end

local indices, script_name, script_ptr, script_len, script_dir, accelerate, decelerate, params = mod.get_keyframe(
  bank_id, keyframe_id, scene_id, project_session_nonce, index)

SCRIPT_CACHE = SCRIPT_CACHE or {}
local f
if SCRIPT_CACHE[script_name] then
  f = SCRIPT_CACHE[script_name]
else
  local err
  local script = ffi.string(script_ptr, script_len)
  f, err = loadstring(script, script_name)
  if not f then
    error("Failed to load keyframe script: " .. err)
  end
  local inner_G = {}
  local inner_obj = {}

  ENHANCED_TRACKS_STATE = {
    indices, accelerate, decelerate, params
  }

  inner_obj.getpoint = function(...)
    local indices, accelerate, decelerate, params = unpack(ENHANCED_TRACKS_STATE)
    local args = { ... }
    local target = args[1]
    local option = args[2]
    local option2 = args[3]
    if type(target) == "number" then
      local remapped_index = indices[target + 1]
      if target < 0 then
        remapped_index = 0
      elseif target >= #indices then
        remapped_index = indices[#indices]
      end
      if #args > 1 then
        return obj.getpoint(remapped_index, option)
      else
        return obj.getpoint(remapped_index)
      end
    elseif target == "time" then
      if option then
        local remapped_index = indices[option + 1]
        if option < 0 then
          remapped_index = indices[1]
        elseif option >= #indices then
          remapped_index = indices[#indices]
        end
        return obj.getpoint("time", remapped_index) - obj.getpoint("time", indices[1])
      else
        return obj.getpoint("time") - obj.getpoint("time", indices[1])
      end
    elseif target == "accelerate" then
      return accelerate
    elseif target == "decelerate" then
      return decelerate
    elseif target == "index" then
      local current_time = obj.getpoint("time")
      for i = 1, #indices - 1 do
        local left_time = obj.getpoint("time", indices[i])
        local right_time = obj.getpoint("time", indices[i + 1])
        if current_time < left_time then
          return i - 1
        elseif current_time < right_time then
          return i - 1 + (current_time - left_time) / (right_time - left_time)
        end
      end
      return #indices - 1
    elseif target == "param" then
      return unpack(params)
    elseif target == "num" then
      return #indices
    elseif target == "timecontrol" then
      local target_time = option2 or obj.getpoint("time")
      local left_time = obj.getpoint("time", indices[1])
      local right_time = obj.getpoint("time", indices[#indices])
      ratio = target_time / (right_time - left_time)
      local value = mod.get_timecontrol_value(bank_id, keyframe_id, scene_id, project_session_nonce, index, ratio)
      if option == "value" then
        return value
      end
      local remapped_time = left_time + value * (right_time - left_time)
      if option == "time" then
        return remapped_time - left_time
      end

      if remapped_time < left_time then
        local first_section_time = obj.getpoint("time", indices[1])
        local second_section_time = obj.getpoint("time", indices[2])
        return -1 + (remapped_time - first_section_time) / (second_section_time - first_section_time)
      end
      for i = 1, #indices - 1 do
        local ileft_time = obj.getpoint("time", indices[i])
        local iright_time = obj.getpoint("time", indices[i + 1])
        if remapped_time < iright_time then
          return i - 1 + (remapped_time - ileft_time) / (iright_time - ileft_time)
        end
      end
      return #indices - 1 +
          (remapped_time - obj.getpoint("time", indices[#indices])) /
          (obj.getpoint("time", indices[#indices]) - obj.getpoint("time", indices[#indices - 1]))
    else
      return obj.getpoint(unpack(args))
    end
  end

  inner_G.require = function(name)
    local loader_prefix
    if #script_dir > 0 then
      loader_prefix = script_dir .. "/?.lua;" .. script_dir .. "/?.dll;"
      package.path = loader_prefix .. package.path
    end
    local ok, result = pcall(require, name)
    if #script_dir > 0 then
      package.path = package.path:sub(#loader_prefix + 1)
    end
    if not ok then
      error("Failed to require module '" .. name .. "': " .. tostring(result))
    end
    return result
  end

  if inspect then
    print("== Keyframe Track Debug Info ==")
    print("Bank ID:", bank_id)
    print("Keyframe ID:", keyframe_id)
    print("Indices:", indices)
    print("Script Name:", script_name)
    print("Accelerate:", accelerate)
    print("Decelerate:", decelerate)
    print("Params:", params)
    local original_getpoint = inner_obj.getpoint
    inner_obj.getpoint = function(...)
      local args = { ... }
      local ret = { original_getpoint(unpack(args)) }
      print("getpoint", args, "->", ret)
      return unpack(ret)
    end
  end

  inner_G.obj = inner_obj

  setmetatable(inner_obj, { __index = obj, __newindex = obj })
  setmetatable(inner_G, { __index = _G, __newindex = _G })
  setfenv(f, inner_G)
  SCRIPT_CACHE[script_name] = f
end

local res = f()

return res
