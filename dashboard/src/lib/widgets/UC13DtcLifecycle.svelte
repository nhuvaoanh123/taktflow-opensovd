<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- UC13 — Per-DTC lifecycle: Pending → Confirmed → Cleared → Suppressed (SYSTEM-SPEC §6.1) -->
<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { CANNED_DTCS } from '$lib/api/sovdClient';

	type LifecycleState = 'Pending' | 'Confirmed' | 'Cleared' | 'Suppressed';

	const LIFECYCLE: LifecycleState[] = ['Pending', 'Confirmed', 'Cleared', 'Suppressed'];

	const STATE_STYLE: Record<LifecycleState, string> = {
		Pending: 'bg-yellow-800 text-yellow-200 border-yellow-600',
		Confirmed: 'bg-red-800 text-red-200 border-red-600',
		Cleared: 'bg-green-800 text-green-200 border-green-600',
		Suppressed: 'bg-slate-700 text-slate-300 border-slate-500'
	};

	// Show first 4 DTCs, each at a different lifecycle stage
	const dtcStubs = CANNED_DTCS.slice(0, 4);

	let states = $state<LifecycleState[]>(
		['Pending', 'Confirmed', 'Cleared', 'Suppressed']
	);

	let timer: ReturnType<typeof setInterval> | null = null;

	onMount(() => {
		timer = setInterval(() => {
			// Randomly advance one DTC's lifecycle for animation effect
			const idx = Math.floor(Math.random() * states.length);
			const curr = LIFECYCLE.indexOf(states[idx]);
			states = states.map((s, i) =>
				i === idx ? LIFECYCLE[(curr + 1) % LIFECYCLE.length] : s
			);
		}, 2500);
	});

	onDestroy(() => {
		if (timer) clearInterval(timer);
	});
</script>

<div class="rounded-lg border border-border bg-card p-3">
	<h3 class="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
		DTC Lifecycle (§6.1)
	</h3>
	<div class="space-y-1.5">
		{#each dtcStubs as dtc, i (dtc.id)}
			<div class="flex items-center gap-2 text-xs">
				<span class="w-12 font-mono font-semibold shrink-0">{dtc.code}</span>
				<div class="flex flex-1 gap-1">
					{#each LIFECYCLE as stage (stage)}
						<span
							class="flex-1 rounded border px-1 py-0.5 text-center text-[10px] font-semibold transition-colors duration-500
								{states[i] === stage ? STATE_STYLE[stage] : 'border-border bg-muted/10 text-muted-foreground/40'}"
						>
							{stage}
						</span>
					{/each}
				</div>
			</div>
		{/each}
	</div>
</div>
