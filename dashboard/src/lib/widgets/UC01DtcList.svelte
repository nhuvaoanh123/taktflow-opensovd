<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- UC01 — Read DTCs per component, status-mask filtered (FR-1.1) -->
<script lang="ts">
	import type { DtcEntry, DtcStatus, EcuId } from '$lib/types/sovd';
	import { CANNED_DTCS } from '$lib/api/sovdClient';

	interface Props {
		componentId: EcuId;
		onSelect?: (dtc: DtcEntry) => void;
	}

	let { componentId, onSelect }: Props = $props();

	const STATUS_OPTIONS: { value: DtcStatus | 'all'; label: string }[] = [
		{ value: 'all', label: 'All' },
		{ value: 'confirmed', label: 'Confirmed' },
		{ value: 'pending', label: 'Pending' },
		{ value: 'cleared', label: 'Cleared' },
		{ value: 'suppressed', label: 'Suppressed' },
		{ value: 'test_failed', label: 'Test Failed' }
	];

	let statusMask = $state<DtcStatus | 'all'>('all');
	let page = $state(0);
	const PAGE_SIZE = 5;

	const allFaults = $derived(CANNED_DTCS.filter((d) => d.component === componentId));
	const filtered = $derived(
		statusMask === 'all' ? allFaults : allFaults.filter((d) => d.status === statusMask)
	);
	const pageCount = $derived(Math.ceil(filtered.length / PAGE_SIZE));
	const visible = $derived(filtered.slice(page * PAGE_SIZE, (page + 1) * PAGE_SIZE));

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
			DTC List — {componentId.toUpperCase()} ({filtered.length})
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
		<p class="py-2 text-center text-xs text-muted-foreground">No faults for this filter.</p>
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
							<span class="rounded px-1 py-0.5 text-[10px] {SEVERITY_COLOR[dtc.severity]}"
								>{dtc.severity[0].toUpperCase()}</span
							>
						</td>
						<td class="py-1 {STATUS_COLOR[dtc.status]}">{dtc.status}</td>
						<td class="py-1 text-right tabular-nums">{dtc.occurrences}</td>
					</tr>
				{/each}
			</tbody>
		</table>
	{/if}

	{#if pageCount > 1}
		<div class="mt-2 flex justify-between text-xs text-muted-foreground">
			<button
				disabled={page === 0}
				onclick={() => (page = Math.max(0, page - 1))}
				class="rounded px-2 py-0.5 disabled:opacity-40 hover:bg-accent">&laquo; Prev</button
			>
			<span>{page + 1} / {pageCount}</span>
			<button
				disabled={page >= pageCount - 1}
				onclick={() => (page = Math.min(pageCount - 1, page + 1))}
				class="rounded px-2 py-0.5 disabled:opacity-40 hover:bg-accent">Next &raquo;</button
			>
		</div>
	{/if}
</div>
