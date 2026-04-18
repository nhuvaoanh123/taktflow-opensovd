<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- UC01 - Read DTCs per component, status-mask filtered (FR-1.1) -->
<script lang="ts">
	import { listFaults } from '$lib/api/sovdClient';
	import type { DtcEntry, DtcStatus, EcuId } from '$lib/types/sovd';

	interface Props {
		componentId: EcuId;
		onSelect?: (dtc: DtcEntry) => void;
		page?: number;
		pageSize?: number;
		onPage?: (page: number) => void;
		onTotalChange?: (total: number) => void;
		refreshNonce?: number;
	}

	let {
		componentId,
		onSelect,
		page = 0,
		pageSize = 5,
		onPage,
		onTotalChange,
		refreshNonce = 0
	}: Props = $props();

	const STATUS_OPTIONS: { value: DtcStatus | 'all'; label: string }[] = [
		{ value: 'all', label: 'All' },
		{ value: 'confirmed', label: 'Confirmed' },
		{ value: 'pending', label: 'Pending' },
		{ value: 'cleared', label: 'Cleared' },
		{ value: 'suppressed', label: 'Suppressed' },
		{ value: 'test_failed', label: 'Test Failed' }
	];

	let statusMask = $state<DtcStatus | 'all'>('all');
	let localPage = $state(0);
	let allFaults = $state<DtcEntry[]>([]);
	let loading = $state(true);
	let lastResetKey = $state('');

	const currentPage = $derived(onPage ? page : localPage);
	const filtered = $derived(
		statusMask === 'all' ? allFaults : allFaults.filter((fault) => fault.status === statusMask)
	);
	const pageCount = $derived(Math.max(1, Math.ceil(filtered.length / pageSize)));
	const visible = $derived(filtered.slice(currentPage * pageSize, (currentPage + 1) * pageSize));

	$effect(() => {
		const resetKey = `${componentId}:${statusMask}:${refreshNonce}`;
		if (resetKey === lastResetKey) {
			return;
		}
		lastResetKey = resetKey;
		if (onPage) {
			onPage(0);
			return;
		}
		localPage = 0;
	});

	$effect(() => {
		void load(componentId, refreshNonce);
	});

	$effect(() => {
		onTotalChange?.(filtered.length);
		const lastPage = Math.max(0, pageCount - 1);
		if (currentPage <= lastPage) {
			return;
		}
		if (onPage) {
			onPage(lastPage);
		} else {
			localPage = lastPage;
		}
	});

	async function load(id: EcuId, _refreshNonce: number) {
		loading = true;
		try {
			allFaults = await listFaults(id);
		} finally {
			loading = false;
		}
	}

	function setPage(nextPage: number) {
		const bounded = Math.max(0, Math.min(pageCount - 1, nextPage));
		if (onPage) {
			onPage(bounded);
		} else {
			localPage = bounded;
		}
	}

	const SEVERITY_COLOR: Record<string, string> = {
		critical: 'bg-red-700 text-white',
		high: 'bg-orange-600 text-white',
		medium: 'bg-yellow-500 text-black',
		low: 'bg-slate-500 text-white'
	};

	const STATUS_COLOR: Record<string, string> = {
		confirmed: 'text-red-400',
		pending: 'text-yellow-400',
		cleared: 'text-green-400',
		suppressed: 'text-slate-400',
		test_failed: 'text-orange-400',
		warning_indicator: 'text-purple-400'
	};
</script>

<div class="rounded-lg border border-border bg-card p-3 text-card-foreground">
	<div class="mb-2 flex items-center justify-between">
		<span class="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
			DTC List - {componentId.toUpperCase()} ({filtered.length})
		</span>
		<select
			bind:value={statusMask}
			class="rounded border border-border bg-background px-1 py-0.5 text-xs text-foreground"
		>
			{#each STATUS_OPTIONS as opt (opt.value)}
				<option value={opt.value}>{opt.label}</option>
			{/each}
		</select>
	</div>

	{#if visible.length === 0}
		<p class="py-2 text-center text-xs text-muted-foreground">
			{loading ? 'Loading faults...' : 'No faults for this filter.'}
		</p>
	{:else}
		<table class="w-full text-xs">
			<thead>
				<tr class="border-b border-border">
					<th class="py-1 text-left font-medium text-muted-foreground">Code</th>
					<th class="py-1 text-left font-medium text-muted-foreground">Description</th>
					<th class="py-1 text-left font-medium text-muted-foreground">Sev</th>
					<th class="py-1 text-left font-medium text-muted-foreground">Status</th>
					<th class="py-1 text-right font-medium text-muted-foreground">#</th>
				</tr>
			</thead>
			<tbody>
				{#each visible as dtc (dtc.id)}
					<tr
						class="cursor-pointer border-b border-border/40 hover:bg-accent/30"
						onclick={() => onSelect?.(dtc)}
					>
						<td class="py-1 font-mono font-semibold">{dtc.code}</td>
						<td class="max-w-[120px] truncate py-1 text-muted-foreground">{dtc.description}</td>
						<td class="py-1">
							<span class="rounded px-1 py-0.5 text-[10px] {SEVERITY_COLOR[dtc.severity]}">
								{dtc.severity[0].toUpperCase()}
							</span>
						</td>
						<td class="py-1 {STATUS_COLOR[dtc.status]}">{dtc.status}</td>
						<td class="py-1 text-right tabular-nums">{dtc.occurrences}</td>
					</tr>
				{/each}
			</tbody>
		</table>
	{/if}

	{#if !onPage && pageCount > 1}
		<div class="mt-2 flex justify-between text-xs text-muted-foreground">
			<button
				disabled={currentPage === 0}
				onclick={() => setPage(currentPage - 1)}
				class="rounded px-2 py-0.5 disabled:opacity-40 hover:bg-accent"
			>
				Prev
			</button>
			<span>{currentPage + 1} / {pageCount}</span>
			<button
				disabled={currentPage >= pageCount - 1}
				onclick={() => setPage(currentPage + 1)}
				class="rounded px-2 py-0.5 disabled:opacity-40 hover:bg-accent"
			>
				Next
			</button>
		</div>
	{/if}
</div>
