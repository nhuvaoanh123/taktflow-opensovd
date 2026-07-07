<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- UC18 - Gateway routing / backend registry topology (FR-6.1, FR-6.2) -->
<script lang="ts">
	import { ChevronDown, Network } from 'lucide-svelte';
	import { onMount } from 'svelte';

	import { getGatewayHealth, listGatewayBackends } from '$lib/api/sovdClient';
	import type { GatewayBackend, GatewayHealth } from '$lib/types/sovd';

	let backends = $state<GatewayBackend[] | null>(null);
	let loading = $state(true);
	let health = $state<GatewayHealth | null>(null);

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
		loading = false;
	}

	function probeTone(status: GatewayHealth['sovdDb']['status']): string {
		if (status === 'degraded') return 'text-amber-700';
		if (status === 'unavailable') return 'text-red-700';
		return 'text-emerald-700';
	}
</script>

<div class="rounded-lg border border-border bg-card p-5 shadow-sm">
	<h3 class="flex items-center gap-2 text-base font-semibold">
		<span class="flex h-6 w-6 items-center justify-center rounded-md bg-sky-50 text-sky-600">
			<Network class="h-3.5 w-3.5" />
		</span>
		Gateway
	</h3>
	<p class="mb-3 mt-0.5 text-xs text-muted-foreground">
		sovd-main health, and how it reaches each component.
	</p>
	{#if health}
		<div class="mb-3 grid gap-2 text-[11px] md:grid-cols-4">
			<div class="rounded-md border border-border bg-muted/40 p-2">
				<div class="text-muted-foreground">Server</div>
				<div class="text-sm font-semibold">v{health.version}</div>
			</div>
			<div class="rounded-md border border-border bg-muted/40 p-2">
				<div class="text-muted-foreground">Cycle</div>
				<div class="text-sm font-semibold capitalize">{health.operationCycle ?? 'idle'}</div>
			</div>
			<div class="rounded-md border border-border bg-muted/40 p-2">
				<div class="text-muted-foreground">SOVD DB</div>
				<div class="text-sm font-semibold {probeTone(health.sovdDb.status)}">
					{health.sovdDb.status}
				</div>
				{#if health.sovdDb.reason}
					<div class="truncate text-muted-foreground">{health.sovdDb.reason}</div>
				{/if}
			</div>
			<div class="rounded-md border border-border bg-muted/40 p-2">
				<div class="text-muted-foreground">Fault sink</div>
				<div class="text-sm font-semibold {probeTone(health.faultSink.status)}">
					{health.faultSink.status}
				</div>
				{#if health.faultSink.reason}
					<div class="truncate text-muted-foreground">{health.faultSink.reason}</div>
				{/if}
			</div>
		</div>
		<p class="mb-2 text-[11px] text-muted-foreground">
			Health latency {health.latencyMs} ms
		</p>
	{:else if !loading}
		<p class="mb-2 text-[11px] text-muted-foreground">Health route unavailable.</p>
	{/if}
	{#if backends && backends.length > 0}
		{@const upCount = backends.filter((b) => b.reachable).length}
		<details class="group">
			<summary class="flex cursor-pointer list-none items-center justify-between gap-2 py-1 text-xs font-medium text-muted-foreground [&::-webkit-details-marker]:hidden">
				<span>
					Backend routes ({backends.length}) —
					{upCount === backends.length ? 'all up' : `${upCount} up, ${backends.length - upCount} down`}
				</span>
				<ChevronDown class="h-3.5 w-3.5 shrink-0 transition-transform group-open:rotate-180" />
			</summary>
		<table class="w-full text-xs">
			<thead>
				<tr class="border-b border-border">
					<th class="py-1.5 text-left font-medium text-muted-foreground">Backend</th>
					<th class="py-1.5 text-left font-medium text-muted-foreground">Address</th>
					<th class="py-1.5 text-left font-medium text-muted-foreground">Proto</th>
					<th class="py-1.5 text-right font-medium text-muted-foreground">Latency</th>
					<th class="py-1.5 text-right font-medium text-muted-foreground">Status</th>
				</tr>
			</thead>
			<tbody>
				{#each backends as b (b.id)}
					<tr class="border-b border-border/60">
						<td class="py-1.5 font-mono font-medium">{b.id}</td>
						<td class="py-1.5 font-mono text-muted-foreground">{b.address}</td>
						<td class="py-1.5 uppercase">{b.protocol}</td>
						<td class="py-1.5 text-right tabular-nums">
							{b.reachable ? `${b.latencyMs} ms` : '--'}
						</td>
						<td class="py-1.5 text-right">
							{#if b.reachable}
								<span class="rounded-full border border-emerald-200 bg-emerald-50 px-1.5 py-0.5 text-[10px] font-medium text-emerald-700">up</span>
							{:else}
								<span class="rounded-full border border-red-200 bg-red-50 px-1.5 py-0.5 text-[10px] font-medium text-red-700">down</span>
							{/if}
						</td>
					</tr>
				{/each}
			</tbody>
		</table>
		</details>
	{:else}
		<p class="py-2 text-center text-xs text-muted-foreground">
			{#if loading}
				Loading backend routes...
			{:else if backends === null}
				Backend registry route unavailable.
			{:else}
				No backend routes registered.
			{/if}
		</p>
	{/if}
</div>
