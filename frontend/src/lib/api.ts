import type { BackendConfig } from './types';

const DEFAULT_BACKEND = '';

function backendBaseUrl(): string {
    const fromEnv = import.meta.env.VITE_BACKEND_URL as string | undefined;
    if (fromEnv && fromEnv.length > 0) {
        return fromEnv.replace(/\/$/, '');
    }

    if (typeof window !== 'undefined') {
        // Use relative URL to support proxying
        return '';
    }

    return DEFAULT_BACKEND;
}

export async function fetchConfig(): Promise<BackendConfig> {
    const response = await fetch(`${backendBaseUrl()}/config`, {
        headers: {
            Accept: 'application/json',
        },
    });

    if (!response.ok) {
        throw new Error(`Backend responded with ${response.status}`);
    }

    return response.json() as Promise<BackendConfig>;
}

export async function fetchHealth(): Promise<void> {
    const response = await fetch(`${backendBaseUrl()}/health`, {
        cache: 'no-store',
    });

    if (!response.ok) {
        throw new Error(`Health check failed (${response.status})`);
    }
}

export function streamUrl(): string {
    return `${backendBaseUrl()}/stream`;
}
