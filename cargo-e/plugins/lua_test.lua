local is_windows = package.config:sub(1,1) == '\\'
print("Script path is: " .. script_path())
return {
  name = "lua-test",
  matches = function(dir) return true end,
  collect_targets = function(dir)
    return '[{"name":"say_hello","metadata":null}]'
  end,
  build_command = function(dir, t)
    if is_windows then
      return '{"prog":"cmd","args":["/c","echo Lua:' .. t .. '"],"cwd":null}'
    else
      return '{"prog":"sh","args":["-c","echo Lua:' .. t .. '"],"cwd":null}'
    end
  end,
}