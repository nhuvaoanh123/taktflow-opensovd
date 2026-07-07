<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- Live DID reads: VIN, battery voltage, and temperature. -->
<script lang="ts">
	import { onDestroy, onMount } from 'svelte';

	import { readDid } from '$lib/api/sovdClient';
	import type { EcuId, LiveDid } from '$lib/types/sovd';

	interface Props {
		componentId: EcuId;
	}

	let { componentId }: Props = $props();

	let data = $state<LiveDid | null>(null);
	let loading = $state(true);
	let timer: ReturnType<typeof setInterval> | null = null;

	async function poll() {
		data = await readDid(componentId);
		loading = false;
	}

	onMount(() => {
		void poll();
		timer = setInterval(() => {
			void poll();
		}, 1000);
	});

	onDestroy(() => {
		if (timer) clearInterval(timer);
	});
</script>

<div class="rounded-md border border-border bg-card p-3 text-xs">
	<h3 class="mb-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
		Live data - {componentId.toUpperCase()} <span class="text-emerald-700">1 Hz</span>
	</h3>
	{#if data}
		<dl class="grid grid-cols-2 gap-x-3 gap-y-0.5">
			<dt class="text-muted-foreground">VIN</dt>
			<dd class="font-mono">{data.vin ?? '--'}</dd>

			<dt class="text-muted-foreground">Battery</dt>
			<dd class="font-mono font-semibold">
				{data.batteryVoltage !== undefined ? `${data.batteryVoltage.toFixed(2)} V` : '--'}
			</dd>

			<dt class="text-muted-foreground">Temp</dt>
			<dd class="font-mono">
				{data.temperature !== undefined ? `${data.temperature.toFixed(1)} C` : '--'}
			</dd>

			<dt class="text-muted-foreground">Updated</dt>
			<dd class="tabular-nums">{new Date(data.timestamp).toLocaleTimeString()}</dd>
		</dl>
	{:else}
		<p class="text-muted-foreground">
			{loading
				? 'Loading...'
				: `Data route unavailable for ${componentId.toUpperCase()}.`}
		</p>
	{/if}
</div>
