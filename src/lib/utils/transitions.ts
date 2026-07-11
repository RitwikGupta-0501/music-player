import { flushSync } from "svelte";

export function transitionLayout(updateFn: () => void) {
    if (document.startViewTransition) {
        document.startViewTransition(() => {
            flushSync(() => {
                updateFn();
            });
        });
    } else {
        updateFn();
    }
}
