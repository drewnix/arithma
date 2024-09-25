import {defineConfig} from 'vite'
import path from "path"
import react from '@vitejs/plugin-react'
import wasm from 'vite-plugin-wasm';

// https://vitejs.dev/config/
export default defineConfig({
    plugins: [react(), wasm()],
    resolve: {
        alias: {
            "@": path.resolve(__dirname, "./src"),
        },
    },
    define: {
        'process.env': {}
    },
    server: {
        fs: {
            allow: ['..'], // Allow access to the parent directory where the WASM files are located
        },
    },
})
