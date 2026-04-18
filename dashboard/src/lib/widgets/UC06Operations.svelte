<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- UC06 - Start / stop / poll routines (FR-2.1-2.3) -->
<script lang="ts">
	import {
		CANNED_ROUTINES,
		listRoutines,
		pollRoutine,
		startRoutine,
		stopRoutine
	} from '$lib/api/sovdClient';
	import type { EcuId, RoutineEntry } from '$lib/types/sovd';

	interface Props {
		componentId?: EcuId;
	}

	let { componentId }: Props = $props();

	let baseRoutines = $state<RoutineEntry[]>(CANNED_ROUTINES);
	let statusOverride = $state<Record<string, RoutineEntry>>({});

	const routines = $derived(baseRoutines.map((routine) => statusOverride[routine.id] ?? routine));

	$effect(() => {
		statusOverride = {};
		void load(componentId);
	});

	$effect(() => {
		if (!componentId) {
			return;
		}
		const timer = setInterval(() => {
			void refreshRunning(componentId);
		}, 1500);
		return () => clearInterval(timer);
	});

	const STATUS_CHIP: Record<string, string> = {
		idle: 'bg-slate-600 text-slate-200',
		running: 'bg-blue-600 text-white animate-pulse',
		completed: 'bg-green-700 text-white',
		failed: 'bg-red-700 text-white'
	};

	async function load(id?: EcuId) {
		baseRoutines = id ? await listRoutines(id) : CANNED_ROUTINES;
	}

	async function refreshRunning(id: EcuId) {
		const running = routines.filter((routine) => routine.status === 'running');
		if (running.length === 0) {
			return;
		}
		const updates = await Promise.all(
			running.map(async (routine) => [routine.id, await pollRoutine(id, routine.id)] as const)
		);
		statusOverride = {
			...statusOverride,
			...Object.fromEntries(updates)
		};
	}

	async function handleStart(routine: RoutineEntry) {
		const target = componentId ?? routine.component;
		await startRoutine(target, routine.id);
		statusOverride = {
			...statusOverride,
			[routine.id]: { ...routine, status: 'running', lastResult: 'Execution started' }
		};
		await refreshRunning(target);
	}

	async function handleStop(routine: RoutineEntry) {
		const target = componentId ?? routine.component;
		await stopRoutine(target, routine.id);
		statusOverride = {
			...statusOverride,
			[routine.id]: { ...routine, status: 'idle', lastResult: 'Stopped by user' }
		};
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
