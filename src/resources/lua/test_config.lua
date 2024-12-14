config_add_library_folder("./lua/lib/?.lua")

config_set_endpoint("/lua", "./lua/test_endpoint.lua")
print("[Config-Lua] Done");