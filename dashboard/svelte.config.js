// SPDX-License-Identifier: Apache-2.0
import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	preprocess: vitePreprocess(),
	kit: {
		adapter: adapter({
			pages: 'build',
			assets: 'build',
			fallback: 'index.html',
			precompress: false,
			strict: false
		}),
		paths: {
			// Production deploy mounts at /dashboard/ on sovd.taktflow-systems.com.
			// Set SOVD_BASE_PATH=/dashboard at build time; dev server stays at /.
			base: process.env.SOVD_BASE_PATH || ''
		},
		alias: {
			$lib: './src/lib'
		}
	}
};

export default config;
