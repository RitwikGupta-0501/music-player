use mlua::Lua;

/// Hard cap on Lua instructions per call.
/// 1_000_000 ≈ a few hundred ms of tight Lua computation on modern hardware.
const INSTRUCTION_LIMIT: u32 = 1_000_000;

/// Hard cap on Lua heap allocation per VM (4 MB).
const LUA_MEMORY_LIMIT: usize = 4 * 1024 * 1024;

/// Configure a freshly created Lua VM with all sandbox restrictions.
///
/// Must be called immediately after `Lua::new()` before loading any untrusted
/// script content. `script_name` is used in telemetry so violations can be
/// attributed to the right provider.
pub fn configure_sandbox(lua: &Lua, script_name: &str) -> mlua::Result<()> {
    // ── 1. Instruction limit ─────────────────────────────────────────────────
    let name_for_hook = script_name.to_string();
    lua.set_hook(
        mlua::HookTriggers::new().every_nth_instruction(INSTRUCTION_LIMIT),
        move |_lua, _debug| {
            crate::telemetry::record_error(
                "lua_sandbox",
                &format!("instruction limit exceeded in provider '{}'", name_for_hook),
            );
            Err(mlua::Error::RuntimeError(
                "sandbox: instruction limit exceeded".into(),
            ))
        },
    )?;

    // ── 2. Memory limit ──────────────────────────────────────────────────────
    lua.set_memory_limit(LUA_MEMORY_LIMIT)?;

    // ── 3. Strip dangerous globals ───────────────────────────────────────────
    let globals = lua.globals();
    for name in &[
        // Standard dangerous modules
        "os", "io", "package", "require", "dofile", "loadfile", "load", "debug",
        // GC manipulation
        "collectgarbage",
        // Coroutines can bypass instruction-count hooks when resumed from C,
        // but removing it completely breaks mlua's async call_async machinery
        // raw* allow metatable bypass / sandbox escapes
        "rawequal", "rawget", "rawset", "rawlen",
    ] {
        // Ignore "not found" — some may not be present in all Lua 5.4 builds
        let _ = globals.raw_remove(*name);
    }

    Ok(())
}
