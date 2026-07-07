// SPDX-License-Identifier: Apache-2.0
import { sveltekit } from '@sveltejs/kit/vite';
import { svelteTesting } from '@testing-library/svelte/vite';
import { defineConfig } from 'vitest/config';

// Local verification harness: `SOVD_PROXY_TARGET=https://... vite preview`
// serves the built dashboard against a remote SOVD API without CORS issues.
const sovdProxy = process.env.SOVD_PROXY_TARGET
	? { '/sovd': { target: process.env.SOVD_PROXY_TARGET, changeOrigin: true } }
	: undefined;

export default defineConfig({
	plugins: [sveltekit(), svelteTesting()],
	server: { proxy: sovdProxy },
	preview: { proxy: sovdProxy },
	test: {
		environment: 'jsdom',
		setupFiles: ['./tests/setup.ts'],
		include: ['tests/**/*.test.ts']
	}
});
