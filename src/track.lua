--param:Bank ID (Do not edit these parameters),0
--param:Keyframe ID,0
--param:Scene ID,0
--param:Project Session Nonce,0
--label:

-- NOTE: 行数が多すぎるとパースの時間がかかってパフォーマンスが劣化するので、requireで
-- パースをキャッシュする

local o_bank_id, o_keyframe_id, o_scene_id, o_project_session_nonce = obj.getpoint("param")
local core = require("enhanced_tracks_core")
return core.run_script(o_bank_id, o_keyframe_id, o_scene_id, o_project_session_nonce)
