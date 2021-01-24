-- function spread() {
--     src1 = source("/dbaudio1/positioning...")
--     src2 = source("/...")
    
--     return sync(src1, src2)
-- }

-- function bla() {
--     print("Config!")

-- }

require "jit"

jit.on()
print(jit.version)

print("Config")

function test()
    return 3
end

-- left = bridge.discover_ds100()
-- right = bridge.discover_wing()

-- left = bridge.add_device("wing", left)
-- right = bridge.add_device("ds100", right)

function gain_wing_to_ds100(val)
    if val > 0 then
        return val / 10 * 24
    else
        return val / 144.0 * 120.0
    end
end

function gain_ds100_to_wing(val)
    if val > 0 then
        return val / 24 * 10
    else
        return val / 120 * 144
    end
end

for n in 1 .. 24 do
    local xy_sync = bridge.new_sync({
        name = "ch" .. n .. "/pos",
        dim = 2,
        range = [
            { min = 0, max = 100 },
            { min = 0, max = 100 },
        ]
    })

    local y_sync = bridge.new_sync({
        name = "ch" .. n .. "/y",
        range = { min = 0, max = 100 }
    })


    x_sync.add_source({
        address = "ds100:/dbaudio1/coordinatemapping/source_position_xy/1/" .. n,
        read = function (self, msg) self.set(msg[1]) end,
        write = 
    }

    x_sync.add_source("wing:/ch/" .. n .. "/send/1/pan")
end