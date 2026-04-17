<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- UC11 — Animated fault pipeline: Shim → Debouncer → OpCycle → DTC (FR-4.x) -->
<script lang="ts">
	import { onMount, onDestroy } from 'svelte';

	type Stage = 'shim' | 'debouncer' | 'opcycle' | 'dtc';

	const STAGES: { id: Stage; label: string; sub: string }[] = [
		{ id: 'shim', label: 'FaultShim', sub: 'Report()' },
		{ id: 'debouncer', label: 'Debouncer', sub: '50 ms window' },
		{ id: 'opcycle', label: 'Op Cycle', sub: 'Idle→Running' },
		{ id: 'dtc', label: 'DTC Store', sub: 'P0A1F stored' }
	];

	let active = $state<Stage>('shim');
	let animating = $state(false);
	let timer: ReturnType<typeof setInterval> | null = null;

	function nextStage(s: Stage): Stage {
		const idx = STAGES.findIndex((x) => x.id === s);
		return STAGES[(idx + 1) % STAGES.length].id;
	}

	function tick() {
		active = nextStage(active);
	}

	onMount(() => {
		animating = true;
		timer = setInterval(tick, 900);
	});

	onDestroy(() => {
		if (timer) clearInterval(timer);
	});
</script>

<div class="rounded-lg border border-border bg-card p-3">
	<h3 class="mb-3 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
		Fault Pipeline
	</h3>
	<div class="flex items-center gap-1">
		{#each STAGES as stage, i (stage.id)}
			<div
				class="flex flex-1 flex-col items-center rounded-md border px-2 py-2 text-center text-xs transition-colors duration-300
					{active === stage.id
					? 'border-primary bg-primary/20 text-primary'
					: 'border-border bg-muted/20 text-muted-foreground'}"
			>
				<span class="font-semibold">{stage.label}</span>
				<span class="text-[10px]">{stage.sub}</span>
			</div>
			{#if i < STAGES.length - 1}
				<span
					class="text-sm transition-colors {active === STAGES[i + 1].id
						? 'text-primary'
						: 'text-muted-foreground'}"
				>
					→
				</span>
			{/if}
		{/each}
	</div>
	<p class="mt-2 text-[10px] text-muted-foreground">
		Active stage: <span class="font-mono text-foreground">{active}</span>
	</p>
</div>
