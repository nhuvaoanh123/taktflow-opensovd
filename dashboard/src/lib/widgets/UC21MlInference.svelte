<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- UC21 predictive inference widget (ADR-0028 / ADR-0029 / Phase 8) -->
<script lang="ts">
	import type { EcuId, MlInferenceResult } from '$lib/types/sovd';
	import { runMlInference } from '$lib/api/sovdClient';

	interface Props {
		componentId: EcuId;
	}

	let { componentId }: Props = $props();

	let result = $state<MlInferenceResult | null>(null);
	let loading = $state(false);
	let rollingBack = $state(false);
	let error = $state<string | null>(null);

	const predictionChip: Record<MlInferenceResult['prediction'], string> = {
		normal: 'bg-emerald-700 text-white',
		warning: 'bg-amber-600 text-black',
		critical: 'bg-red-700 text-white'
	};

	async function refresh() {
		loading = true;
		error = null;
		try {
			result = await runMlInference(componentId);
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'ML inference failed';
		} finally {
			loading = false;
		}
	}

	async function rollback() {
		rollingBack = true;
		error = null;
		try {
			result = await runMlInference(componentId, {
				action: 'rollback',
				force_trigger: 'operator_rollback'
			});
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Rollback request failed';
		} finally {
			rollingBack = false;
		}
	}

	$effect(() => {
		result = null;
		void refresh();
	});
</script>

<div class="rounded-lg border border-border bg-card p-3 text-xs">
	<div class="mb-2 flex items-center justify-between gap-2">
		<h3 class="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
			UC21 Predictive Fault Inference - {componentId.toUpperCase()}
		</h3>
		<div class="flex items-center gap-1">
			<button
				onclick={refresh}
				class="rounded bg-primary px-2 py-0.5 text-[10px] text-primary-foreground hover:bg-primary/80 disabled:opacity-60"
				disabled={loading || rollingBack}
			>
				{loading ? 'Running...' : 'Run inference'}
			</button>
			{#if componentId === 'cvc'}
				<button
					onclick={rollback}
					class="rounded border border-border px-2 py-0.5 text-[10px] hover:bg-accent disabled:opacity-60"
					disabled={loading || rollingBack}
				>
					{rollingBack ? 'Rolling back...' : 'Force rollback'}
				</button>
			{/if}
		</div>
	</div>

	{#if error}
		<p class="mb-2 rounded border border-red-500/30 bg-red-500/10 px-2 py-1 text-[10px] text-red-200">
			{error}
		</p>
	{/if}

	{#if result}
		<div class="space-y-2">
			<div class="flex items-center justify-between gap-2">
				<div>
					<p class="font-medium">{result.modelName}</p>
					<p class="text-[10px] text-muted-foreground">
						v{result.modelVersion} · {result.source === 'live' ? 'live SOVD path' : 'stub fallback'} ·
						{result.lifecycleState === 'rolled_back' ? 'rolled back' : 'ready'}
					</p>
				</div>
				<span class="rounded px-1.5 py-0.5 text-[10px] font-semibold {predictionChip[result.prediction]}">
					{result.prediction}
				</span>
			</div>

			<p class="rounded border border-border/60 bg-background/60 px-2 py-1 text-[10px] text-foreground/90">
				{#if result.advisoryActive}
					Predictive advisory active for {componentId.toUpperCase()}.
				{:else if result.lifecycleState === 'rolled_back'}
					Advisory cleared after rollback to the safe baseline.
				{:else}
					No predictive advisory active.
				{/if}
			</p>

			<dl class="grid grid-cols-2 gap-x-3 gap-y-0.5">
				<dt class="text-muted-foreground">Confidence</dt>
				<dd class="font-mono">{(result.confidence * 100).toFixed(1)}%</dd>

				<dt class="text-muted-foreground">Status</dt>
				<dd class="font-mono">{result.status}</dd>

				<dt class="text-muted-foreground">Lifecycle</dt>
				<dd class="font-mono">{result.lifecycleState}</dd>

				<dt class="text-muted-foreground">Fingerprint</dt>
				<dd class="truncate font-mono text-[10px]">{result.fingerprint}</dd>

				<dt class="text-muted-foreground">Updated</dt>
				<dd class="tabular-nums">{new Date(result.updatedAt).toLocaleTimeString()}</dd>
			</dl>

			{#if result.rollbackTrigger}
				<dl class="grid grid-cols-2 gap-x-3 gap-y-0.5 rounded border border-border/60 bg-background/50 p-2">
					<dt class="text-muted-foreground">Rollback trigger</dt>
					<dd class="font-mono">{result.rollbackTrigger}</dd>

					<dt class="text-muted-foreground">From</dt>
					<dd class="font-mono">{result.rollbackFromModelVersion}</dd>

					<dt class="text-muted-foreground">To</dt>
					<dd class="font-mono">{result.rollbackToModelVersion}</dd>

					<dt class="text-muted-foreground">At</dt>
					<dd class="tabular-nums">
						{result.rollbackAt ? new Date(result.rollbackAt).toLocaleTimeString() : '--'}
					</dd>
				</dl>
			{/if}

			<p class="text-[10px] text-muted-foreground">
				Request path: <code>/sovd/v1/components/{componentId}/operations/ml-inference/executions</code>
			</p>
		</div>
	{:else}
		<p class="text-muted-foreground">Loading inference result...</p>
	{/if}
</div>
