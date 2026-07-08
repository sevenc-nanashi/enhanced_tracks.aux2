--param:Bank ID (Do not edit these parameters),0
--param:Keyframe ID,0
--param:Scene ID,0
--param:Project Session Nonce,0

local mod = obj.module("enhanced_tracks.aux2")
local ffi = require("ffi")
local o_bank_id, o_keyframe_id, o_scene_id, o_project_session_nonce = obj.getpoint("param")
local o_index, o_ratio = math.modf(obj.getpoint("index"))
local o_inspect = mod.debug_mode()

if o_bank_id == 0 then
  if o_inspect then
    print("== Keyframe Track Debug Info ==")
    print("Bank ID is 0, falling back to linear track")
  end
  local left = obj.getpoint(o_index)
  local right = obj.getpoint(o_index + 1)
  return left + (right - left) * o_ratio
end

local o_indices, o_script_name, o_script_ptr, o_script_len, o_script_dir, o_accelerate, o_decelerate, o_params = mod
    .get_keyframe(
      o_bank_id, o_keyframe_id, o_scene_id, o_project_session_nonce, o_index)

SCRIPT_CACHE = SCRIPT_CACHE or {}
if mod.is_cache_cleared() then
  print("@info", "clearing script cache")
  SCRIPT_CACHE = {}
  mod.reset_cache_cleared()
end

local f
if SCRIPT_CACHE[o_script_name] then
  f = SCRIPT_CACHE[o_script_name]
else
  local err
  local script = ffi.string(o_script_ptr, o_script_len)
  f, err = loadstring(script, o_script_name)
  if not f then
    error("Failed to load keyframe script: " .. err)
  end
  local inner_G = {}
  local inner_obj = {}

  inner_obj.getpoint = function(...)
    if ENHANCED_TRACKS_STATE == nil then
      return obj.getpoint(...)
    end
    local bank_id, keyframe_id, scene_id, project_session_nonce, index, indices, accelerate, decelerate, params = unpack(
      ENHANCED_TRACKS_STATE)
    if mod.debug_mode() then
      print("== Keyframe Track Debug Info @ getpoint ==")
      print("Indices:", indices)
      print("Accelerate:", accelerate)
      print("Decelerate:", decelerate)
      print("Params:", params)
      print("Arguments:", { ... })
    end
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
    elseif target == "frame_s" then
      local starting_index = indices[1]
      local starting_time = obj.getpoint("time", starting_index)
      return obj.getpoint("frame_s") + starting_time
    elseif target == "frame_e" then
      local ending_index = indices[#indices]
      local ending_time = obj.getpoint("time", ending_index)
      return obj.getpoint("frame_s") + ending_time
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

  local function module_env_with_inner_obj(env, env_cache)
    if env == inner_G then
      return inner_G
    end
    if env_cache[env] then
      return env_cache[env]
    end

    local replaced_env = {}
    setmetatable(replaced_env, {
      __index = function(_, key)
        if key == "obj" then
          return inner_obj
        end
        return env[key]
      end,
      __newindex = env,
    })
    env_cache[env] = replaced_env
    return replaced_env
  end

  local function replace_obj_in_loaded_module(module, seen, env_cache)
    if type(module) == "function" then
      local ok, env = pcall(getfenv, module)
      if ok and type(env) == "table" then
        pcall(setfenv, module, module_env_with_inner_obj(env, env_cache))
      end
      return
    end
    if type(module) ~= "table" then
      return
    end

    if seen[module] then
      return
    end
    seen[module] = true

    for _, value in pairs(module) do
      replace_obj_in_loaded_module(value, seen, env_cache)
    end
  end

  inner_G.require = function(name)
    local loader_prefix
    if #o_script_dir > 0 then
      loader_prefix = o_script_dir .. "/?.lua;" .. o_script_dir .. "/?.dll;"
      package.path = loader_prefix .. package.path
    end
    local ok, result = pcall(require, name)
    if #o_script_dir > 0 then
      package.path = package.path:sub(#loader_prefix + 1)
    end
    if not ok then
      error("Failed to require module '" .. name .. "': " .. tostring(result))
    end
    replace_obj_in_loaded_module(result, {}, {})
    return result
  end

  inner_G.obj = inner_obj

  setmetatable(inner_obj, { __index = obj, __newindex = obj })
  setmetatable(inner_G, { __index = _G, __newindex = _G })
  setfenv(f, inner_G)
  SCRIPT_CACHE[o_script_name] = f
end

ENHANCED_TRACKS_STATE = {
  o_bank_id, o_keyframe_id, o_scene_id, o_project_session_nonce, o_index, o_indices, o_accelerate, o_decelerate, o_params
}

if o_inspect then
  print("== Keyframe Track Debug Info @ script execution ==")
  print("Bank ID:", o_bank_id)
  print("Keyframe ID:", o_keyframe_id)
  print("Indices:", o_indices)
  print("Script Name:", o_script_name)
  print("Accelerate:", o_accelerate)
  print("Decelerate:", o_decelerate)
  print("Params:", o_params)
end

local res = f()
ENHANCED_TRACKS_STATE = nil

return res
