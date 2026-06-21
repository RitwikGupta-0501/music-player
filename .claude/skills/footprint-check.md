# Skill: Binary & Memory Footprint Check

**Trigger:** When significantly refactoring Rust code, adding new Cargo dependencies, or finalizing a backend feature.

**Instructions:**
Our backend daemon has a strict budget. You must proactively validate the memory footprint of your changes.

1. **Footprint Check:** Run `cargo bloat` and/or `cargo size --release` (if available in the environment) to verify the binary footprint hasn't exploded.
2. **Idle RAM Target:** The Rust daemon must target an idle RAM tracking of roughly 10MB-15MB.
3. **Optimization:** If the binary size increases significantly, suggest or implement Cargo optimizations in `Cargo.toml` such as:
   - `lto = true`
   - `opt-level = "z"`
   - `codegen-units = 1`
   - `panic = "abort"`
   - `strip = true`
