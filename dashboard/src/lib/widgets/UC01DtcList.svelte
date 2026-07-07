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
	let unavailable = $state(false);
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
			const faults = await listFaults(id);
			unavailable = faults === null;
			allFaults = faults ?? [];
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
		critical: 'border-red-300 bg-red-50 text-red-700',
		high: 'border-orange-300 bg-orange-50 text-orange-700',
		medium: 'border-amber-300 bg-amber-50 text-amber-700',
		low: 'border-slate-300 bg-slate-50 text-slate-700'
	};

	const STATUS_COLOR: Record<string, string> = {
		confirmed: 'text-red-700',
		pending: 'text-amber-700',
		cleared: 'text-emerald-700',
		suppressed: 'text-slate-600',
		test_failed: 'text-orange-700',
		warning_indicator: 'text-slate-700'
	};
</script>

<div class="text-card-foreground">
	<div class="mb-2 flex items-center justify-end gap-2">
		<label class="text-xs text-muted-foreground" for="fault-status-filter">Status</label>
		<select
			id="fault-status-filter"
			bind:value={statusMask}
			class="rounded border border-input bg-background px-2 py-1 text-xs text-foreground"
		>
			{#each STATUS_OPTIONS as opt (opt.value)}
				<option value={opt.value}>{opt.label}</option>
			{/each}
		</select>
	</div>

	{#if visible.length === 0}
		<p class="py-2 text-center text-xs text-muted-foreground">
			{#if loading}
				Loading faults...
			{:else if unavailable}
				Fault route unavailable for {componentId.toUpperCase()}.
			{:else}
				No faults for this filter.
			{/if}
		</p>
	{:else}
		<table class="w-full text-sm">
			<thead>
				<tr class="border-b border-border text-xs">
					<th class="py-1.5 text-left font-medium text-muted-foreground">Code</th>
					<th class="py-1.5 text-left font-medium text-muted-foreground">Description</th>
					<th class="py-1.5 text-left font-medium text-muted-foreground">Sev</th>
					<th class="py-1.5 text-left font-medium text-muted-foreground">Status</th>
					<th class="py-1.5 text-right font-medium text-muted-foreground">#</th>
				</tr>
			</thead>
			<tbody>
				{#each visible as dtc (dtc.id)}
					<tr
						class="cursor-pointer border-b border-border/60 hover:bg-indigo-50/40"
						onclick={() => onSelect?.(dtc)}
					>
						<td class="py-1.5 font-mono font-semibold">{dtc.code}</td>
						<td class="max-w-[140px] truncate py-1.5 text-muted-foreground">{dtc.description}</td>
						<td class="py-1.5">
							<span
								class="rounded border px-1.5 py-0.5 text-[11px] font-medium {SEVERITY_COLOR[
									dtc.severity
								]}"
							>
								{dtc.severity}
							</span>
						</td>
						<td class="py-1.5 text-xs {STATUS_COLOR[dtc.status]}">{dtc.status}</td>
						<td class="py-1.5 text-right tabular-nums">{dtc.occurrences ?? '--'}</td>
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
