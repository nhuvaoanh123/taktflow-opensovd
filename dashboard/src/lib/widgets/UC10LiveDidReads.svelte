<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- UC10 — Live DID reads: VIN + battery voltage + temperature at 1 Hz (FR-3.3) -->
<script lang="ts">
	import type { EcuId, LiveDid } from '$lib/types/sovd';
	import { readDid } from '$lib/api/sovdClient';
	import { onMount, onDestroy } from 'svelte';

	interface Props {
		componentId: EcuId;
	}

	let { componentId }: Props = $props();

	let data = $state<LiveDid | null>(null);
	let timer: ReturnType<typeof setInterval> | null = null;

	async function poll() {
		data = await readDid(componentId);
	}

	onMount(() => {
		poll();
		timer = setInterval(poll, 1000);
	});

	onDestroy(() => {
		if (timer) clearInterval(timer);
	});
</script>

<div class="rounded-lg border border-border bg-card p-3 text-xs">
	<h3 class="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
		Live DIDs — {componentId.toUpperCase()} <span class="text-green-400">● 1 Hz</span>
	</h3>
	{#if data}
		<dl class="grid grid-cols-2 gap-x-3 gap-y-0.5">
			<dt class="text-muted-foreground">VIN</dt>
			<dd class="font-mono">{data.vin}</dd>

			<dt class="text-muted-foreground">Battery</dt>
			<dd class="font-mono font-semibold">{data.batteryVoltage.toFixed(2)} V</dd>

			<dt class="text-muted-foreground">Temp</dt>
			<dd class="font-mono">{data.temperature.toFixed(1)} °C</dd>

			<dt class="text-muted-foreground">Updated</dt>
			<dd class="tabular-nums">{new Date(data.timestamp).toLocaleTimeString()}</dd>
		</dl>
	{:else}
		<p class="text-muted-foreground">Loading…</p>
	{/if}
</div>
