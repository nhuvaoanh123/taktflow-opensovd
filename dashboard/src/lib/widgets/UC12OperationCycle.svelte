<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- UC12 — Operation cycle state machine viz (FR-4.3) -->
<script lang="ts">
	import { onMount, onDestroy } from 'svelte';

	type OpState = 'Idle' | 'Running' | 'Evaluating' | 'Complete';

	const STATES: OpState[] = ['Idle', 'Running', 'Evaluating', 'Complete'];

	const TRANSITIONS: Record<OpState, OpState> = {
		Idle: 'Running',
		Running: 'Evaluating',
		Evaluating: 'Complete',
		Complete: 'Idle'
	};

	const STATE_COLOR: Record<OpState, string> = {
		Idle: 'border-slate-500 bg-slate-800 text-slate-200',
		Running: 'border-blue-500 bg-blue-900 text-blue-200 animate-pulse',
		Evaluating: 'border-yellow-500 bg-yellow-900 text-yellow-200',
		Complete: 'border-green-500 bg-green-900 text-green-200'
	};

	let current = $state<OpState>('Idle');
	let timer: ReturnType<typeof setInterval> | null = null;

	onMount(() => {
		timer = setInterval(() => {
			current = TRANSITIONS[current];
		}, 1500);
	});

	onDestroy(() => {
		if (timer) clearInterval(timer);
	});
</script>

<div class="rounded-lg border border-border bg-card p-3">
	<h3 class="mb-3 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
		Operation Cycle State
	</h3>
	<div class="flex flex-wrap items-center gap-2">
		{#each STATES as state (state)}
			<div
				class="flex flex-1 min-w-[60px] flex-col items-center rounded border px-2 py-2 text-xs transition-all duration-300 {STATE_COLOR[state]}"
			>
				<span class="font-semibold">{state}</span>
				{#if current === state}
					<span class="mt-0.5 text-[10px] opacity-80">◀ current</span>
				{/if}
			</div>
			{#if state !== 'Complete'}
				<span class="text-muted-foreground">→</span>
			{/if}
		{/each}
	</div>
</div>
