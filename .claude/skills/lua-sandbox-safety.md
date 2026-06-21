# Skill: Lua Sandbox Safety

**Trigger:** When modifying the `mlua` runtime environment, evaluating user-provided Lua scripts, or adding new Rust-to-Lua FFI functions.

**Instructions:**
When extending the `mlua` runtime, you MUST enforce strict sandboxing. Your goal is to ensure a rogue or poorly written provider script never compromises the application.

1. **Execution Bounding:** Always implement instruction count limits via `mlua`'s `set_hook` to prevent infinite loops.
2. **Memory Bounding:** Implement strict memory allocation limits for the Lua environment to respect our strict memory targets for the backend.
3. **Async Thread-Safety:** When writing async Rust functions for Lua (such as wrapping `reqwest` calls), ensure memory isn't leaked across the FFI boundary. Handle Tokio task cancellation gracefully if the Lua script is terminated or dropped.
