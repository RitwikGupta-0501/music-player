let _toastId = 0;

interface Toast {
    id: number;
    message: string;
    type: 'info' | 'success' | 'error';
    duration: number;
}

class ToastStore {
    toasts = $state<Toast[]>([]);

    show(message: string, type: 'info' | 'success' | 'error' = 'info', duration = 4000) {
        const id = ++_toastId;
        this.toasts = [...this.toasts, { id, message, type, duration }];

        setTimeout(() => {
            this.dismiss(id);
        }, duration);
    }

    dismiss(id: number) {
        this.toasts = this.toasts.filter(t => t.id !== id);
    }

    info(message: string) { this.show(message, 'info'); }
    success(message: string) { this.show(message, 'success'); }
    error(message: string) { this.show(message, 'error'); }
}

export const toastStore = new ToastStore();
export type { Toast };
