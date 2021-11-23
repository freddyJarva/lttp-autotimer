if not event then
  -- detect snes9x by absence of 'event'
  is_snes9x = true
  memory.usememorydomain = function()
    -- snes9x always uses "System Bus" domain, which cannot be switched
  end
  memory.read_u8 = memory.readbyte
  memory.read_s8 = memory.readbytesigned
  memory.read_u16_le = memory.readword
  memory.read_s16_le = memory.readwordsigned
  memory.read_u32_le = memory.readdword
  memory.read_s32_le = memory.readdwordsigned
  memory.read_u16_be = function(addr) return bit.rshift(bit.bswap(memory.read_u16_le(addr)),16) end
  local color_b2s = function(bizhawk_color)
    if bizhawk_color == nil then return nil end
    return bit.rol(bizhawk_color,8)
  end
  gui.drawText = function(x,y,text,color)
    gui.text(x,y,text,color_b2s(color))
  end
  gui.drawLine = function(x1,y1,x2,y2,color)
    gui.line(x1,y1,x2,y2,color_b2s(color))
  end
  gui.drawBox = function(x1,y1,x2,y2,outline_color,fill_color)
    gui.box(x1,y1,x2,y2,color_b2s(fill_color),color_b2s(outline_color))
  end
  event = {}
  event.onframeend = function(luaf,name)
    local on_gui_update_old = gui.register()
    local function on_gui_update_new()
      if on_gui_update_old then
        on_gui_update_old()
      end
      luaf()
    end
    gui.register(on_gui_update_new)
  end
end

function readbyterange(addr, lenght, domain)
  if is_snes9x then
      -- print("readbyterange, addr: ", addr, "length: ", lenght, "domain: ", domain)
      return memory.readbyterange(addr, lenght)
  else
      local mtable = memory.readbyterange(addr, lenght, domain)
      local toret = {};
      for i=0, (lenght - 1) do
          table.insert(toret, mtable[i])
      end
      return toret
  end

end
function writebyte(addr, value, domain)
  if is_snes9x then
    memory.writebyte(addr, value)
  else
    memory.writebyte(addr, value, domain)
  end
end
function DrawNiceText(text_x, text_y, str, color)
  --local sh = client.screenheight
  --local sw = client.screenwidth
  if is_snes9x then 
    gui.text(text_x, text_y, str, color)
  else
    local calc_x = client.transformPointX(text_x)
    local calc_y = client.transformPointY(text_y)
    gui.text(calc_x, calc_y, str, color)
  end
end

-- End of Bizhawk compatibility layer
-----------------------------------------------


local socket = require("socket.core")

local qusb_connection
local timer_connection
local host = '127.0.0.1'
local timer_port = 46700
local qusb_port = 65398
local qusb_connected = false
local timer_connected = false
local stopped = false
local version = "4"
if is_snes9x then
version = 1
else
version = "BizHawk"
end
local name = "Unnamed"

memory.usememorydomain("System Bus")

local function onQusbMessage(s)
  local parts = {}
  for part in string.gmatch(s, '([^|]+)') do
      parts[#parts + 1] = part
  end
  if parts[1] == "Read" then
      local adr = tonumber(parts[2])
      local length = tonumber(parts[3])
      local domain
      if is_snes9x ~= true then
        domain = parts[4]
      end
      local byteRange = readbyterange(adr, length, domain)
      qusb_connection:send("{\"data\": [" .. table.concat(byteRange, ",") .. "]}\n")
  elseif parts[1] == "Write" then
      local adr = tonumber(parts[2])
      local domain
      local offset = 2
      if is_snes9x ~= true then
        domain = parts[3]
        offset = 3
      end
      for k, v in pairs(parts) do
          if k > offset then
              writebyte(adr + k - offset - 1, tonumber(v), domain)
          end
      end
  elseif parts[1] == "SetName" then
    name = parts[2]
    print("My name is " .. name .. "!")

  elseif parts[1] == "Message" then
      print(parts[2])
  elseif parts[1] == "Exit" then
      print("Lua script stopped, to restart the script press \"Restart\"")
      stopped = true
  elseif parts[1] == "Version" then
      qusb_connection:send("Version|Multitroid LUA|" .. version .. "|\n")
  end
end

function TableConcat(t1,t2)
  for i=1,#t2 do
      t1[#t1+1] = t2[i]
  end
  return t1
end

local function onMessage(s)
  local parts = {}
  local domain
  for part in string.gmatch(s, '([^|]+)') do
      parts[#parts + 1] = part
  end
  local length = tonumber(parts[3])
  if parts[1] == "READ" then
      local addresses = {}
      for adr in string.gmatch(parts[2], '([^,]+)') do
        addresses = TableConcat(addresses, readbyterange(tonumber(adr), length, domain))
      end
      local return_message = "{\"data\": [" .. table.concat(addresses, ",") .. "]}\n"
      timer_connection:send(return_message)
  elseif parts[1] == "Write" then
      local adr = tonumber(parts[2])
      local domain
      local offset = 2
      if is_snes9x ~= true then
        domain = parts[3]
        offset = 3
      end
      for k, v in pairs(parts) do
          if k > offset then
              writebyte(adr + k - offset - 1, tonumber(v), domain)
          end
      end
  elseif parts[1] == "SetName" then
    name = parts[2]
    print("My name is " .. name .. "!")

  elseif parts[1] == "Message" then
      print(parts[2])
  elseif parts[1] == "Exit" then
      print("Lua script stopped, to restart the script press \"Restart\"")
      stopped = true
  elseif parts[1] == "Version" then
      timer_connection:send("Version|Multitroid LUA|" .. version .. "|\n")
  end
end


local main = function()
  if stopped then
      return nil
  end

  if not timer_connected then
      print('LuaBridge r' .. version)
      print('Connecting to Timer at ' .. host .. ':' .. timer_port)
      timer_connection, err = socket:tcp()
      if err ~= nil then
          emu.print(err)
          return
      end

      local returnCode, errorMessage = timer_connection:connect(host, timer_port)
      if (returnCode == nil) then
          print("Error while connecting: " .. errorMessage)
          stopped = true
          timer_connected = false
          print("Please press \"Restart\" to try to reconnect to Timer, make sure it's running and the Lua bridge device is activated")
          return
      end

      timer_connection:settimeout(0)
      timer_connected = true
      print('connected to Timer')
      return
  end
  if not qusb_connected then
    print('LuaBridge r' .. version)
    print('Connecting to qusb at ' .. host .. ':' .. qusb_port)
    qusb_connection, err = socket:tcp()
    if err ~= nil then
        emu.print(err)
        return
    end

    local returnCode, errorMessage = qusb_connection:connect(host, qusb_port)
    if (returnCode == nil) then
        print("Error while connecting: " .. errorMessage)
        stopped = true
        qusb_connected = false
        print("Please press \"Restart\" to try to reconnect to qusb, make sure it's running and the Lua bridge device is activated")
        return
    end

    qusb_connection:settimeout(0)
    qusb_connected = true
    print('connected to qusb')
    return
  end
  
  local s, status = timer_connection:receive('*l')
  if s then
      onMessage(s)
  end
  if status == 'closed' then
      print('timer_connection to timer is closed')
      timer_connection:close()
      timer_connected = false
      return
  end
  local qusb_s, qusb_status = qusb_connection:receive('*l')
  if qusb_s then
      onQusbMessage(qusb_s)
  end
  if qusb_status == 'closed' then
      print('timer_connection to timer is closed')
      timer_connection:close()
      timer_connected = false
      return
  end
end
if is_snes9x then
  emu.registerbefore(main)
else
  while true do
    main()
    emu.frameadvance()
  end
end
