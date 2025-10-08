import { cwd } from 'node:process';
import { defineConfig, loadEnv } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig(({ mode }: { mode: string }) => {
    const env = loadEnv(mode, cwd(), '');
    const backendUrl = env.VITE_BACKEND_URL ?? 'http://localhost:8080';

    return {
        plugins: [svelte()],
        server: {
            host: true,
            port: 5173,
            proxy: {
                '^/(stream|config|health)$': {
                    target: backendUrl,
                    changeOrigin: true,
                    secure: false,
                },
            },
        },
        preview: {
            host: true,
            port: 4173,
        },
    };
});
