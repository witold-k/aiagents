_G.vim = vim -- make checker happy


-- obtain project dir
local script_path = debug.getinfo(1, "S").source:sub(2)
local proj = require("projectsetup")
proj.project_setup_cargo_just(script_path)

