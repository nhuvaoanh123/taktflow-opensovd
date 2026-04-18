<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- UC19 — Historical trends: Grafana iframe placeholder (NFR-3.x — Prometheus, not Timestream, per ADR-0024 OQ-24.2) -->
<script lang="ts">
	interface Props {
		grafanaUrl?: string;
		visible?: boolean;
	}

	let { grafanaUrl = '', visible = false }: Props = $props();

	const DEFAULT_URL = '/grafana/d/sovd-stage1/taktflow-sovd-stage1?kiosk';
	const effectiveUrl = $derived(grafanaUrl || DEFAULT_URL);
	const shouldEmbed = $derived(Boolean(grafanaUrl) || import.meta.env.PROD);
</script>

{#if visible}
	<div class="rounded-lg border border-border bg-card p-3">
		<h3 class="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
			Historical Trends — Grafana / Prometheus (NFR-3.x)
		</h3>
		{#if shouldEmbed}
			<iframe
				title="Grafana Historical Dashboard"
				src={effectiveUrl}
				class="h-64 w-full rounded border border-border"
				sandbox="allow-scripts allow-same-origin allow-forms"
			></iframe>
		{:else}
			<div class="flex h-40 flex-col items-center justify-center rounded border border-dashed border-border text-center text-xs text-muted-foreground">
				<p class="font-semibold">Grafana iframe — Stage 2</p>
				<p class="mt-1">Set <code class="font-mono">VITE_GRAFANA_URL</code> env var to wire up in dev.</p>
				<p class="mt-1">Expected: <code class="font-mono">{DEFAULT_URL}</code></p>
				<p class="mt-2 text-[10px]">Prometheus datasource on Pi at :9090 · ADR-0024 OQ-24.2</p>
			</div>
		{/if}
	</div>
{:else}
	<div class="rounded-lg border border-dashed border-border bg-card/50 p-3 text-center text-xs text-muted-foreground">
		Historical panel hidden — click "Show Historical" to expand
	</div>
{/if}
