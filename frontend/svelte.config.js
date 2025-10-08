import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

const config = {
    preprocess: vitePreprocess(),
    compilerOptions: {
        dev: process.env.NODE_ENV !== 'production',
    },
};

export default config;
