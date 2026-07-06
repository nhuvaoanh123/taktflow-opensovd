<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- Static fault processing path. -->
<script lang="ts">
	const STAGES = [
		{ label: 'Fault ingest', sub: 'Report endpoint' },
		{ label: 'Debounce', sub: 'Status normalization' },
		{ label: 'Operation cycle', sub: 'Cycle state binding' },
		{ label: 'DTC store', sub: 'Persisted fault record' }
	];
</script>

<div class="rounded-md border border-border bg-card p-3">
	<h3 class="mb-3 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
		Fault processing
	</h3>
	<div class="grid gap-2 sm:grid-cols-4">
		{#each STAGES as stage, i (stage.label)}
			<div class="rounded border border-border bg-muted/40 px-2 py-2 text-xs">
				<div class="mb-1 flex items-center justify-between gap-2">
					<span class="font-medium">{stage.label}</span>
					<span class="font-mono text-[10px] text-muted-foreground">{i + 1}</span>
				</div>
				<p class="text-[10px] text-muted-foreground">{stage.sub}</p>
			</div>
		{/each}
	</div>
</div>
