import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [tailwindcss(), sveltekit()],
	optimizeDeps: {
		exclude: ['pdfjs-dist']
	},
	test: {
		include: ['tests/unit/**/*.{test,spec}.ts', 'src/**/*.{test,spec}.ts'],
		environment: 'node'
	}
});
