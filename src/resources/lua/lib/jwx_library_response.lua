jwx = {}

local resp = {}

function resp.writeContent(data)
    internal_writeToResponse(data);
end

function resp.writeHeader(headerName, headerValue)
    internal_setResponseHeader(headerName, headerValue)
end

function resp.getHeader(headerName)
    return internal_getResponseHeader(headerName)
end

resp.setStatusCode = function (code)
    internal_setResponseStatusCode(code)
end

resp.setStatusText = function (text)
    internal_setResponseStatusText(text)
end


jwx.response = resp