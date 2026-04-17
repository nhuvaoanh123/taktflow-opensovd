<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- UC06 — Start / stop / poll routines (FR-2.1-2.3) -->
<script lang="ts">
	import type { RoutineEntry, EcuId } from '$lib/types/sovd';
	import { CANNED_ROUTINES, startRoutine, stopRoutine } from '$lib/api/sovdClient';

	interface Props {
		componentId?: EcuId;
	}

	let { componentId }: Props = $props();

	// Base list derived from prop (reactive to ECU selection changes)
	const baseRoutines = $derived(
		componentId ? CANNED_ROUTINES.filter((r) => r.component === componentId) : CANNED_ROUTINES
	);
	// Overlay for local status mutations (start/stop)
	let statusOverride = $state<Record<string, RoutineEntry>>({});

	const routines = $derived(
		baseRoutines.map((r) => statusOverride[r.id] ?? r)
	);

	const STATUS_CHIP: Record<string, string> = {
		idle: 'bg-slate-600 text-slate-200',
		running: 'bg-blue-600 text-white animate-pulse',
		completed: 'bg-green-700 text-white',
		failed: 'bg-red-700 text-white'
	};

	async function handleStart(rt: RoutineEntry) {
		await startRoutine(rt.id);
		statusOverride = { ...statusOverride, [rt.id]: { ...rt, status: 'running' } };
	}

	async function handleStop(rt: RoutineEntry) {
		await stopRoutine(rt.id);
		statusOverride = { ...statusOverride, [rt.id]: { ...rt, status: 'idle', lastResult: 'Stopped by user' } };
	}
</script>

<div class="rounded-lg border border-border bg-card p-3">
	<h3 class="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
		Operations
	</h3>
	<div class="space-y-1.5">
		{#each routines as rt (rt.id)}
			<div class="flex items-center gap-2 rounded bg-muted/30 px-2 py-1.5 text-xs">
				<span class="grow truncate font-medium">{rt.name}</span>
				<span class="rounded px-1.5 py-0.5 text-[10px] font-semibold {STATUS_CHIP[rt.status]}">
					{rt.status}
				</span>
				{#if rt.status === 'idle' || rt.status === 'failed' || rt.status === 'completed'}
					<button
						onclick={() => handleStart(rt)}
						class="rounded bg-primary px-2 py-0.5 text-[10px] text-primary-foreground hover:bg-primary/80"
					>
						Start
					</button>
				{:else if rt.status === 'running'}
					<button
						onclick={() => handleStop(rt)}
						class="rounded bg-destructive px-2 py-0.5 text-[10px] text-destructive-foreground hover:bg-destructive/80"
					>
						Stop
					</button>
				{/if}
			</div>
			{#if rt.lastResult}
				<p class="ml-2 text-[10px] text-muted-foreground">{rt.lastResult}</p>
			{/if}
		{/each}
	</div>
</div>
