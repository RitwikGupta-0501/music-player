use mlua::Lua;

const LUA_MEMORY_LIMIT: usize = 4 * 1024 * 1024;

pub fn configure_sandbox(lua: &Lua) -> mlua::Result<()> {
    // 1. Instruction limit
    lua.set_hook(
        mlua::HookTriggers::new().every_nth_instruction(100_000),
        |_lua, _debug| Err(mlua::Error::RuntimeError("instruction limit exceeded".into()))
    )?;

    // 2. Memory limit
    lua.set_memory_limit(LUA_MEMORY_LIMIT)?;

    // 3. Restrict globals
    let globals = lua.globals();
    globals.raw_remove("os")?;
    globals.raw_remove("io")?;
    globals.raw_remove("package")?;
    globals.raw_remove("require")?;
    globals.raw_remove("dofile")?;
    globals.raw_remove("loadfile")?;
    globals.raw_remove("load")?;
    globals.raw_remove("debug")?;

    Ok(())
}
