package.path = package.path .. ";./lua/lib/?.lua"

require("jwx_library_response")

function run_request()
    jwx.response.setStatusCode(500)
    jwx.response.setStatusText("Internal server error")
    jwx.response.writeHeader("Content-Type", "text/plain")
    jwx.response.writeContent("JWX LUA error: Your endpoint script has no 'run_request()' function")
end