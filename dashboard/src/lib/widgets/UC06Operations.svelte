<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- Routine execution monitor. Public dashboard builds default to read-only. -->
<script lang="ts">
	import { Play } from 'lucide-svelte';

	import { listRoutines, pollRoutine, startRoutine } from '$lib/api/sovdClient';
	import type { EcuId, RoutineEntry } from '$lib/types/sovd';
	import Panel from './Panel.svelte';

	interface Props {
		componentId?: EcuId;
		controlEnabled?: boolean;
	}

	let { componentId, controlEnabled = false }: Props = $props();

	let baseRoutines = $state<RoutineEntry[]>([]);
	let loading = $state(true);
	let unavailable = $state(false);
	let statusOverride = $state<Record<string, RoutineEntry>>({});
	let actionError = $state<string | null>(null);

	const routines = $derived(baseRoutines.map((routine) => statusOverride[routine.id] ?? routine));

	$effect(() => {
		statusOverride = {};
		actionError = null;
		void load(componentId);
	});

	$effect(() => {
		if (!componentId) {
			return;
		}
		const timer = setInterval(() => {
			void refreshRunning(componentId);
		}, 3000);
		return () => clearInterval(timer);
	});

	const STATUS_CHIP: Record<string, string> = {
		idle: 'border-slate-300 bg-slate-50 text-slate-700',
		running: 'border-blue-300 bg-blue-50 text-blue-700',
		completed: 'border-emerald-300 bg-emerald-50 text-emerald-700',
		failed: 'border-red-300 bg-red-50 text-red-700'
	};

	async function load(id?: EcuId) {
		loading = true;
		try {
			const listed = id ? await listRoutines(id) : [];
			unavailable = listed === null;
			baseRoutines = listed ?? [];
		} finally {
			loading = false;
		}
	}

	async function refreshRunning(id: EcuId) {
		const running = routines.filter((routine) => routine.status === 'running');
		if (running.length === 0) {
			return;
		}
		const updates = await Promise.all(
			running.map(async (routine) => {
				const polled = await pollRoutine(id, routine.id);
				// A failed poll keeps the last known state instead of inventing one.
				return polled === null
					? null
					: ([routine.id, { ...routine, status: polled.status, lastResult: polled.lastResult }] as const);
			})
		);
		statusOverride = {
			...statusOverride,
			...Object.fromEntries(updates.filter((update) => update !== null))
		};
	}

	async function handleStart(routine: RoutineEntry) {
		if (!controlEnabled) {
			return;
		}
		const target = componentId ?? routine.component;
		actionError = null;
		try {
			await startRoutine(target, routine.id);
			statusOverride = {
				...statusOverride,
				[routine.id]: { ...routine, status: 'running', lastResult: 'Execution started' }
			};
			await refreshRunning(target);
		} catch (cause) {
			actionError = cause instanceof Error ? cause.message : 'Routine start failed.';
		}
	}
</script>

<Panel
	title="Operations"
	meta={componentId}
	hint="Diagnostic routines this component exposes — startable only on an operator build; the public build is read-only."
	chip="bg-emerald-50 text-emerald-600"
>
	{#snippet icon()}<Play class="h-3.5 w-3.5" />{/snippet}
	{#snippet actions()}
		<span class="rounded-full border border-border bg-muted/50 px-2 py-0.5 text-[11px] font-medium text-muted-foreground">
			{controlEnabled ? 'Control enabled' : 'Read-only'}
		</span>
	{/snippet}

	{#if actionError}
		<p class="mb-2 rounded border border-red-200 bg-red-50 px-2 py-1 text-[10px] text-red-700">
			{actionError}
		</p>
	{/if}

	{#if routines.length === 0}
		<p class="py-2 text-center text-xs text-muted-foreground">
			{#if loading}
				Loading operations...
			{:else if unavailable}
				Operations route unavailable{componentId ? ` for ${componentId.toUpperCase()}` : ''}.
			{:else}
				No operations exposed.
			{/if}
		</p>
	{:else}
	<div class="divide-y divide-border rounded-md border border-border">
		{#each routines as rt (rt.id)}
			<div class="flex items-center gap-3 px-3 py-2.5 text-sm">
				<div class="min-w-0 grow">
					<p class="truncate font-medium">{rt.name}</p>
					{#if rt.name !== rt.id}
						<p class="truncate font-mono text-[10px] text-muted-foreground">{rt.id}</p>
					{/if}
					{#if rt.lastResult}
						<p class="truncate text-[10px] text-muted-foreground">{rt.lastResult}</p>
					{/if}
				</div>
				<span
					class="rounded border px-2 py-0.5 text-[11px] font-medium uppercase tracking-wide {STATUS_CHIP[
						rt.status
					]}"
				>
					{rt.status}
				</span>
				{#if controlEnabled && (rt.status === 'idle' || rt.status === 'failed' || rt.status === 'completed')}
					<button
						onclick={() => handleStart(rt)}
						class="rounded border border-border bg-white px-2 py-0.5 text-[10px] font-medium text-foreground hover:bg-muted"
					>
						Start
					</button>
				{/if}
			</div>
		{/each}
	</div>
	{/if}
</Panel>
