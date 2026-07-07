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

<div class="text-sm">
	<p class="mb-2 flex items-baseline justify-between gap-2">
		<span class="font-medium">Live data</span>
		<span class="flex items-center gap-1.5 text-[11px] text-muted-foreground">
			<span class="h-1.5 w-1.5 animate-pulse rounded-full bg-emerald-500"></span>
			polled at 1 Hz
		</span>
	</p>
	{#if data}
		{@const hasValues =
			data.vin !== undefined || data.batteryVoltage !== undefined || data.temperature !== undefined}
		<dl class="grid grid-cols-2 gap-x-3 gap-y-1">
			<dt class="text-muted-foreground">VIN</dt>
			<dd class="font-mono">{data.vin ?? '--'}</dd>

			<dt class="text-muted-foreground">Battery</dt>
			<dd class="font-mono text-base font-semibold">
				{data.batteryVoltage !== undefined ? `${data.batteryVoltage.toFixed(2)} V` : '--'}
			</dd>

			<dt class="text-muted-foreground">Temp</dt>
			<dd class="font-mono">
				{data.temperature !== undefined ? `${data.temperature.toFixed(1)} C` : '--'}
			</dd>

			<!-- An Updated stamp with no values ever decoded would suggest data
			     that is not arriving, so it stays -- until a value shows up. -->
			<dt class="text-muted-foreground">Updated</dt>
			<dd class="tabular-nums">
				{hasValues ? new Date(data.timestamp).toLocaleTimeString() : '--'}
			</dd>
		</dl>
	{:else}
		<p class="text-muted-foreground">
			{loading
				? 'Loading...'
				: `Data route unavailable for ${componentId.toUpperCase()}.`}
		</p>
	{/if}
</div>
