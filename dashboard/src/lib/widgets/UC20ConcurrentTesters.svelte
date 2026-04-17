<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- UC20 — Concurrent tester support: connected clients strip (NFR-1.3) -->
<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { isConnected } from '$lib/api/wsClient';

	interface TesterClient {
		id: string;
		type: 'ws' | 'rest';
		agent: string;
		since: string;
	}

	let clients = $state<TesterClient[]>([
		{ id: 'ws-01', type: 'ws', agent: 'SvelteKit dashboard', since: new Date(Date.now() - 30000).toISOString() },
		{ id: 'ws-02', type: 'ws', agent: 'SOVD-Tester v2.1', since: new Date(Date.now() - 120000).toISOString() },
		{ id: 'rest-01', type: 'rest', agent: 'curl/8.2', since: new Date(Date.now() - 5000).toISOString() }
	]);

	let timer: ReturnType<typeof setInterval> | null = null;

	onMount(() => {
		timer = setInterval(() => {
			// Simulate client churn
			const connected = isConnected();
			if (Math.random() > 0.8) {
				const add: TesterClient = {
					id: `rest-${Date.now()}`,
					type: 'rest',
					agent: 'Prometheus scraper',
					since: new Date().toISOString()
				};
				clients = [...clients, add].slice(-10);
			}
		}, 4000);
	});

	onDestroy(() => {
		if (timer) clearInterval(timer);
	});

	function age(iso: string): string {
		const s = Math.floor((Date.now() - new Date(iso).getTime()) / 1000);
		if (s < 60) return `${s}s`;
		return `${Math.floor(s / 60)}m`;
	}
</script>

<div class="flex items-center gap-3 px-2 text-xs">
	<span class="shrink-0 font-semibold text-muted-foreground">
		Testers ({clients.length}):
	</span>
	<div class="flex flex-wrap gap-1.5">
		{#each clients as c (c.id)}
			<span
				class="flex items-center gap-1 rounded-full border px-2 py-0.5 text-[10px]
					{c.type === 'ws' ? 'border-blue-500 bg-blue-900/40 text-blue-200' : 'border-slate-500 bg-slate-800 text-slate-300'}"
			>
				<span>{c.type === 'ws' ? '⚡' : '⬡'}</span>
				<span>{c.agent}</span>
				<span class="opacity-60">{age(c.since)}</span>
			</span>
		{/each}
	</div>
</div>
