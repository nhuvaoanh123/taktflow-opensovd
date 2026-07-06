<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- Historical trends iframe. -->
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
	<div class="rounded-md border border-border bg-card p-3">
		<h3 class="mb-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
			Grafana trends
		</h3>
		{#if shouldEmbed}
			<iframe
				title="Grafana Historical Dashboard"
				src={effectiveUrl}
				class="h-64 w-full rounded border border-border"
				sandbox="allow-scripts allow-same-origin allow-forms"
			></iframe>
		{:else}
			<div class="flex h-32 items-center justify-center rounded border border-dashed border-border text-center text-xs text-muted-foreground">
				Grafana URL not configured for this development build.
			</div>
		{/if}
	</div>
{:else}
	<div class="rounded-md border border-dashed border-border bg-card p-3 text-center text-xs text-muted-foreground">
		Historical panel hidden
	</div>
{/if}
