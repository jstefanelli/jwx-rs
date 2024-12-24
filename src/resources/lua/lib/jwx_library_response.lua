jwx = {}

local resp = {
    ---@type table
    headers = {},
    ---@type number
    statusCode = 200,
    ---@type string
    statusText = "OK",
    ---@type string
    content = "",
    ---@param self table
    ---@param data string
    writeContent = function(self, data)
        self.content = self.content .. data
        local len =  string.len(self.content)
        self:writeHeader("Content-Length", "" .. len)
    end,
    ---@param self table
    ---@param data string
    replaceContent = function(self, data)
        self.content = data
        self:writeHeader("Content-Length", "" .. string.len(self.content))
    end,
    ---@param self table
    ---@param headerName string
    ---@param headerValue string
    writeHeader = function(self, headerName, headerValue)
        self.headers[headerName] = headerValue
    end,
    ---@param self table
    ---@param headerName string
    ---@return string|nil
    getHeader = function(self, headerName)
        return self.headers[headerName]
    end,
    ---@param self table
    ---@param code number
    setStatusCode = function(self, code)
        self.statusCode = code
    end,
    ---@param self table
    ---@param text string
    setStatusText = function(self, text)
        self.statusText = text
    end
}

jwx.response = resp