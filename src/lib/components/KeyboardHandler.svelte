<script lang="ts">
    import { audioStore } from "$lib/stores/audio.svelte";
    import { DEFAULT_KEYMAP, matchesBinding, type KeyAction } from "$lib/stores/keymap";

    $effect(() => {
        function handleKeydown(e: KeyboardEvent) {
            // Don't intercept when user is typing in an input/textarea
            const target = e.target as HTMLElement;
            if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable) {
                return;
            }

            // Find matching action from the keymap
            let matchedAction: KeyAction | null = null;
            for (const [action, entry] of Object.entries(DEFAULT_KEYMAP)) {
                if (matchesBinding(e, entry.binding)) {
                    matchedAction = action as KeyAction;
                    break;
                }
            }

            if (!matchedAction) return;

            // Prevent default browser behavior for matched shortcuts
            e.preventDefault();

            // Dispatch the action
            switch (matchedAction) {
                case 'playPause':
                    if (audioStore.playbackState === 'Playing') {
                        audioStore.pause();
                    } else {
                        audioStore.play();
                    }
                    break;

                case 'seekBack':
                    audioStore.seek(Math.max(0, audioStore.currentTime - 5));
                    break;

                case 'seekForward':
                    audioStore.seek(Math.min(audioStore.duration, audioStore.currentTime + 5));
                    break;

                case 'prevTrack':
                    audioStore.previous();
                    break;

                case 'nextTrack':
                    audioStore.next();
                    break;

                case 'volumeUp':
                    audioStore.setVolume(Math.min(1, audioStore.volume + 0.05));
                    break;

                case 'volumeDown':
                    audioStore.setVolume(Math.max(0, audioStore.volume - 0.05));
                    break;

                case 'toggleShuffle':
                    audioStore.toggleShuffle();
                    break;

                case 'cycleRepeat':
                    audioStore.cycleRepeat();
                    break;

                case 'escape':
                    // Escape is handled by parent — we just bubble it
                    // The +page.svelte can listen for this
                    document.dispatchEvent(new CustomEvent('echo:escape'));
                    break;
            }
        }

        document.addEventListener('keydown', handleKeydown);
        return () => document.removeEventListener('keydown', handleKeydown);
    });
</script>
