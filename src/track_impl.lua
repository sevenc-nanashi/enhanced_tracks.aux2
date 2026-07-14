local mod = obj.module("enhanced_tracks.aux2")
local ffi = require("ffi")
local obj_getpoint = obj.getpoint

local SCRIPT_CACHE = {}
local PATCHED_MODULES = {
  ["enhanced_tracks.aux2"] = true,
  ffi = true,
  bit = true,
  bit32 = true,
  math = true,
  table = true,
  string = true,
  os = true,
  io = true,
  coroutine = true,
  package = true,
}

local function require_with_env(name, env)
  if package.loaded[name] ~= nil then
    return package.loaded[name]
  end

  local errors = {}

  for _, loader in ipairs(package.loaders) do
    local l, extra = loader(name)

    if type(l) == "function" then
      setfenv(l, env)

      local result = l(name, extra)

      if result ~= nil then
        package.loaded[name] = result
      elseif package.loaded[name] == nil then
        package.loaded[name] = true
      end

      return package.loaded[name]
    elseif type(l) == "string" then
      errors[#errors + 1] = l
    end
  end

  error("module '" .. name .. "' not found:" .. table.concat(errors))
end

local function create_patched_require(inner_G, o_script_dir)
  local patched_require = function(name)
    if PATCHED_MODULES[name] then
      return require(name)
    end

    local loader_prefix
    if #o_script_dir > 0 then
      loader_prefix = o_script_dir .. "/?.lua;" .. o_script_dir .. "/?.dll;"
      package.path = loader_prefix .. package.path
    end
    if package.loaded[name] then
      local result = package.loaded[name]
      if
        (type(result) == "table" and result["__enhanced_tracks_patched"])
        or (type(result) == "function" and getfenv(result)["__enhanced_tracks_patched"])
      then
        return result
      end
      print("@info", "Module '" .. name .. "' is already loaded, but not patched for enhanced tracks. Reloading...")
      package.loaded[name] = nil
    end
    print("@info", "Requiring module '" .. name .. "' for enhanced tracks")

    -- require内でのobjとかを置き換えるのは多分これが一番手っ取り早いはず...
    local ok, result = pcall(require_with_env, name, inner_G)

    if #o_script_dir > 0 then
      package.path = package.path:sub(#loader_prefix + 1)
    end
    if not ok then
      error("Failed to require module '" .. name .. "': " .. tostring(result))
    end
    if type(result) == "table" then
      result["__enhanced_tracks_patched"] = true
      for k, v in pairs(result) do
        if type(v) == "function" then
          setfenv(v, inner_G)
        end
      end
      package.loaded[name] = result
    elseif type(result) == "function" then
      local env = getfenv(result)
      env["__enhanced_tracks_patched"] = true
      setfenv(result, env)
      package.loaded[name] = result
    end
    return result
  end

  return patched_require
end

local function patched_getpoint(...)
  if ENHANCED_TRACKS_STATE == nil then
    return obj_getpoint(...)
  end
  local state = ENHANCED_TRACKS_STATE
  local indices = state[6]
  if mod.debug_mode() then
    local bank_id = state[1]
    local keyframe_id = state[2]
    local scene_id = state[3]
    local project_session_nonce = state[4]
    local index = state[5]
    local accelerate = state[7]
    local decelerate = state[8]
    local params = state[9]
    print("== Keyframe Track Debug Info @ getpoint ==")
    print("Bank ID:", bank_id)
    print("Keyframe ID:", keyframe_id)
    print("Scene ID:", scene_id)
    print("Project Session Nonce:", project_session_nonce)
    print("Index:", index)
    print("Indices:", indices)
    print("Accelerate:", accelerate)
    print("Decelerate:", decelerate)
    print("Params:", params)
    print("Arguments:", { ... })
  end
  local target, option, option2 = ...
  if type(target) == "number" then
    target = math.floor(target)
    local remapped_index = indices[target + 1]
    if target < 0 then
      remapped_index = 0
    elseif target >= #indices then
      remapped_index = indices[#indices]
    end
    if select("#", ...) > 1 then
      return obj_getpoint(remapped_index, option)
    else
      return obj_getpoint(remapped_index)
    end
  elseif target == "time" then
    if option then
      local remapped_index = indices[option + 1]
      if option < 0 then
        remapped_index = indices[1]
      elseif option >= #indices then
        remapped_index = indices[#indices]
      end
      local first_time = obj_getpoint("time", indices[1])
      return obj_getpoint("time", remapped_index) - first_time
    else
      local first_time = obj_getpoint("time", indices[1])
      return obj_getpoint("time") - first_time
    end
  elseif target == "frame_s" then
    local starting_index = indices[1]
    local starting_time = obj_getpoint("time", starting_index)
    return obj_getpoint("frame_s") + starting_time
  elseif target == "frame_e" then
    local ending_index = indices[#indices]
    local ending_time = obj_getpoint("time", ending_index)
    return obj_getpoint("frame_s") + ending_time
  elseif target == "accelerate" then
    return state[7]
  elseif target == "decelerate" then
    return state[8]
  elseif target == "index" then
    local current_time = obj_getpoint("time")
    local indices_count = #indices
    local left_time = obj_getpoint("time", indices[1])
    for i = 1, indices_count - 1 do
      local right_time = obj_getpoint("time", indices[i + 1])
      if current_time < left_time then
        return i - 1
      elseif current_time < right_time then
        return i - 1 + (current_time - left_time) / (right_time - left_time)
      end
      left_time = right_time
    end
    return indices_count - 1
  elseif target == "param" then
    return unpack(state[9])
  elseif target == "num" then
    return #indices
  elseif target == "timecontrol" then
    local indices_count = #indices
    local left_time = obj_getpoint("time", indices[1])
    local right_time = obj_getpoint("time", indices[indices_count])
    local duration = right_time - left_time
    local target_time = option2
    if target_time == nil then
      target_time = obj_getpoint("time") - left_time
    end
    local ratio = target_time / duration
    local bank_id = state[1]
    local keyframe_id = state[2]
    local scene_id = state[3]
    local project_session_nonce = state[4]
    local index = state[5]
    local value = mod.get_timecontrol_value(bank_id, keyframe_id, scene_id, project_session_nonce, index, ratio)
    if option == "value" then
      return value
    end
    local remapped_time = left_time + value * duration
    if option == "time" then
      return remapped_time - left_time
    end

    if remapped_time < left_time then
      local second_section_time = obj_getpoint("time", indices[2])
      return -1 + (remapped_time - left_time) / (second_section_time - left_time)
    end
    local ileft_time = left_time
    for i = 1, indices_count - 1 do
      local iright_time = obj_getpoint("time", indices[i + 1])
      if remapped_time < iright_time then
        return i - 1 + (remapped_time - ileft_time) / (iright_time - ileft_time)
      end
      ileft_time = iright_time
    end
    local previous_time = obj_getpoint("time", indices[indices_count - 1])
    return indices_count - 1 + (remapped_time - right_time) / (right_time - previous_time)
  else
    return obj_getpoint(...)
  end
end

local function run_script(o_bank_id, o_keyframe_id, o_scene_id, o_project_session_nonce)
  local o_index, o_ratio = math.modf(obj_getpoint("index"))
  local o_inspect = mod.debug_mode()

  if o_bank_id == 0 then
    if o_inspect then
      print("== Keyframe Track Debug Info ==")
      print("Bank ID is 0, falling back to linear track")
    end
    local left = obj_getpoint(o_index)
    local right = obj_getpoint(o_index + 1)
    return left + (right - left) * o_ratio
  end

  local o_indices, o_script_name, o_accelerate, o_decelerate, o_params =
    mod.get_keyframe(o_bank_id, o_keyframe_id, o_scene_id, o_project_session_nonce, o_index)

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
    local o_script_ptr, o_script_len, o_script_dir = mod.get_script(o_script_name)
    local script = ffi.string(o_script_ptr, o_script_len)
    f, err = loadstring(script, o_script_name)
    if not f then
      error("Failed to load keyframe script: " .. err)
    end
    local inner_G = {}
    local inner_obj = {}

    inner_obj.getpoint = patched_getpoint
    inner_G.obj = inner_obj
    inner_G.require = create_patched_require(inner_G, o_script_dir)

    setmetatable(inner_obj, { __index = obj, __newindex = obj })
    setmetatable(inner_G, { __index = _G, __newindex = _G })
    setfenv(f, inner_G)
    SCRIPT_CACHE[o_script_name] = f
  end

  ENHANCED_TRACKS_STATE = {
    o_bank_id,
    o_keyframe_id,
    o_scene_id,
    o_project_session_nonce,
    o_index,
    o_indices,
    o_accelerate,
    o_decelerate,
    o_params,
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
end

return {
  run_script = run_script,
}
