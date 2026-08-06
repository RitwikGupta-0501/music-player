/**
 * Keyboard shortcut mapping system.
 * 
 * Designed for re-mappability: the Settings page can later
 * read/write these bindings to the SQLite `settings` table.
 * The KeyboardHandler component reads from this store at runtime.
 */

export interface KeyBinding {
    /** The key value (e.g., " " for Space, "ArrowLeft", "s") */
    key: string;
    /** Modifier flags */
    ctrl?: boolean;
    shift?: boolean;
    alt?: boolean;
}

export interface KeymapEntry {
    /** Human-readable label for the Settings UI */
    label: string;
    /** The action identifier */
    action: string;
    /** The key binding */
    binding: KeyBinding;
}

/** The action identifiers used throughout the app */
export type KeyAction =
    | 'playPause'
    | 'seekBack'
    | 'seekForward'
    | 'prevTrack'
    | 'nextTrack'
    | 'volumeUp'
    | 'volumeDown'
    | 'toggleShuffle'
    | 'cycleRepeat'
    | 'search'
    | 'escape';

/** Default keymap — these are the factory defaults */
export const DEFAULT_KEYMAP: Record<KeyAction, KeymapEntry> = {
    playPause: {
        label: 'Play / Pause',
        action: 'playPause',
        binding: { key: ' ' },
    },
    seekBack: {
        label: 'Seek Back 5s',
        action: 'seekBack',
        binding: { key: 'ArrowLeft' },
    },
    seekForward: {
        label: 'Seek Forward 5s',
        action: 'seekForward',
        binding: { key: 'ArrowRight' },
    },
    prevTrack: {
        label: 'Previous Track',
        action: 'prevTrack',
        binding: { key: 'ArrowLeft', ctrl: true },
    },
    nextTrack: {
        label: 'Next Track',
        action: 'nextTrack',
        binding: { key: 'ArrowRight', ctrl: true },
    },
    volumeUp: {
        label: 'Volume Up',
        action: 'volumeUp',
        binding: { key: 'ArrowUp', ctrl: true },
    },
    volumeDown: {
        label: 'Volume Down',
        action: 'volumeDown',
        binding: { key: 'ArrowDown', ctrl: true },
    },
    toggleShuffle: {
        label: 'Toggle Shuffle',
        action: 'toggleShuffle',
        binding: { key: 's', ctrl: true },
    },
    cycleRepeat: {
        label: 'Cycle Repeat',
        action: 'cycleRepeat',
        binding: { key: 'r', ctrl: true },
    },
    escape: {
        label: 'Close / Back',
        action: 'escape',
        binding: { key: 'Escape' },
    },
    search: {
        label: 'Global Search',
        action: 'search',
        binding: { key: 'k', ctrl: true },
    },
};

/**
 * Check if a KeyboardEvent matches a KeyBinding.
 */
export function matchesBinding(event: KeyboardEvent, binding: KeyBinding): boolean {
    if (event.key !== binding.key) return false;
    if ((binding.ctrl ?? false) !== event.ctrlKey) return false;
    if ((binding.shift ?? false) !== event.shiftKey) return false;
    if ((binding.alt ?? false) !== event.altKey) return false;
    return true;
}
