require("jwx_library_main")

function run_request()
    jwx.response.writeContent("Test ok.")
    jwx.response.setStatusCode(200)
    jwx.response.setStatusText("Ok")
end