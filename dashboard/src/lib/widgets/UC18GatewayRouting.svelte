<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- UC18 - Gateway routing / backend registry topology (FR-6.1, FR-6.2) -->
<script lang="ts">
	import { onMount } from 'svelte';

	import { CANNED_BACKENDS, getGatewayHealth, listGatewayBackends } from '$lib/api/sovdClient';
	import type { GatewayBackend, GatewayHealth } from '$lib/types/sovd';

	let backends = $state<GatewayBackend[]>([...CANNED_BACKENDS]);
	let health = $state<GatewayHealth | null>(null);
	let backendsLive = $state(false);

	onMount(() => {
		void load();
	});

	async function load() {
		const [loadedBackends, loadedHealth] = await Promise.all([
			listGatewayBackends(),
			getGatewayHealth()
		]);
		backends = loadedBackends;
		health = loadedHealth;
		backendsLive = loadedBackends !== CANNED_BACKENDS;
	}

	function probeTone(status: GatewayHealth['sovdDb']['status']): string {
		if (status === 'degraded') return 'text-yellow-300';
		if (status === 'unavailable') return 'text-red-400';
		return 'text-green-400';
	}
</script>

<div class="rounded-lg border border-border bg-card p-3">
	<h3 class="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
		Gateway Routing
	</h3>
	{#if health}
		<div class="mb-3 grid gap-2 text-[10px] md:grid-cols-4">
			<div class="rounded border border-border/50 bg-background/50 p-2">
				<div class="text-muted-foreground">Server</div>
				<div class="font-mono">{health.version}</div>
			</div>
			<div class="rounded border border-border/50 bg-background/50 p-2">
				<div class="text-muted-foreground">Cycle</div>
				<div class="font-mono">{health.operationCycle ?? 'idle'}</div>
			</div>
			<div class="rounded border border-border/50 bg-background/50 p-2">
				<div class="text-muted-foreground">SOVD DB</div>
				<div class="font-semibold {probeTone(health.sovdDb.status)}">{health.sovdDb.status}</div>
				{#if health.sovdDb.reason}
					<div class="truncate text-muted-foreground">{health.sovdDb.reason}</div>
				{/if}
			</div>
			<div class="rounded border border-border/50 bg-background/50 p-2">
				<div class="text-muted-foreground">Fault Sink</div>
				<div class="font-semibold {probeTone(health.faultSink.status)}">
					{health.faultSink.status}
				</div>
				{#if health.faultSink.reason}
					<div class="truncate text-muted-foreground">{health.faultSink.reason}</div>
				{/if}
			</div>
		</div>
		<p class="mb-2 text-[10px] text-muted-foreground">
			Live probe from <span class="font-mono">/sovd/v1/health</span> in {health.latencyMs} ms.
			{#if backendsLive}
				Route list below is live from <span class="font-mono">/sovd/v1/gateway/backends</span>.
			{:else}
				Route list below is still on fallback data.
			{/if}
		</p>
	{:else}
		<p class="mb-2 text-[10px] text-muted-foreground">
			Live gateway probe unavailable; showing fallback routing data only.
		</p>
	{/if}
	<table class="w-full text-xs">
		<thead>
			<tr class="border-b border-border">
				<th class="py-1 text-left font-medium text-muted-foreground">Backend</th>
				<th class="py-1 text-left font-medium text-muted-foreground">Address</th>
				<th class="py-1 text-left font-medium text-muted-foreground">Proto</th>
				<th class="py-1 text-right font-medium text-muted-foreground">Latency</th>
				<th class="py-1 text-right font-medium text-muted-foreground">Status</th>
			</tr>
		</thead>
		<tbody>
			{#each backends as b (b.id)}
				<tr class="border-b border-border/40">
					<td class="py-1 font-mono">{b.id}</td>
					<td class="py-1 font-mono text-muted-foreground">{b.address}</td>
					<td class="py-1 uppercase">{b.protocol}</td>
					<td class="py-1 text-right tabular-nums">
						{b.reachable ? `${b.latencyMs} ms` : '--'}
					</td>
					<td class="py-1 text-right">
						{#if b.reachable}
							<span class="text-green-400">up</span>
						{:else}
							<span class="text-red-400">down</span>
						{/if}
					</td>
				</tr>
			{/each}
		</tbody>
	</table>
</div>
