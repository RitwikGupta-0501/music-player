import { invoke } from "@tauri-apps/api/core";

export class SettingsStore {
    glassyPlayerBar = $state(false);

    async init() {
        try {
            const val = await invoke<string | null>("get_setting", { key: "glassy_player_bar" });
            this.glassyPlayerBar = val === "true";
        } catch (e) {
            console.error("Failed to load settings:", e);
        }
    }

    async setGlassyPlayerBar(val: boolean) {
        this.glassyPlayerBar = val;
        try {
            await invoke("set_setting", { key: "glassy_player_bar", value: val ? "true" : "false" });
        } catch (e) {
            console.error("Failed to save setting:", e);
        }
    }
}

export const settingsStore = new SettingsStore();
