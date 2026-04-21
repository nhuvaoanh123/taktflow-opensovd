<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- Taktflow OpenSOVD - Capability Showcase Dashboard (ADR-0024 Stage 1) -->
<!-- Composes all 20 UC widgets per T24.1.7 -->
<script lang="ts">
	import { onMount } from 'svelte';

	import { subscribe } from '$lib/api/wsClient';
	import type { DtcEntry, EcuId, TelemetryFrame } from '$lib/types/sovd';

	import UC01DtcList from '$lib/widgets/UC01DtcList.svelte';
	import UC02DtcDetail from '$lib/widgets/UC02DtcDetail.svelte';
	import UC03ClearFaults from '$lib/widgets/UC03ClearFaults.svelte';
	import UC04Pagination from '$lib/widgets/UC04Pagination.svelte';
	import UC05FaultsTimeline from '$lib/widgets/UC05FaultsTimeline.svelte';
	import UC06Operations from '$lib/widgets/UC06Operations.svelte';
	import UC07RoutineCatalog from '$lib/widgets/UC07RoutineCatalog.svelte';
	import UC08ComponentCards from '$lib/widgets/UC08ComponentCards.svelte';
	import UC09HwSwVersion from '$lib/widgets/UC09HwSwVersion.svelte';
	import UC10LiveDidReads from '$lib/widgets/UC10LiveDidReads.svelte';
	import UC11FaultPipeline from '$lib/widgets/UC11FaultPipeline.svelte';
	import UC12OperationCycle from '$lib/widgets/UC12OperationCycle.svelte';
	import UC13DtcLifecycle from '$lib/widgets/UC13DtcLifecycle.svelte';
	import UC14CdaTopology from '$lib/widgets/UC14CdaTopology.svelte';
	import UC15Session from '$lib/widgets/UC15Session.svelte';
	import UC16AuditLog from '$lib/widgets/UC16AuditLog.svelte';
	import UC17SafetyBoundary from '$lib/widgets/UC17SafetyBoundary.svelte';
	import UC18GatewayRouting from '$lib/widgets/UC18GatewayRouting.svelte';
	import UC19Historical from '$lib/widgets/UC19Historical.svelte';
	import UC20ConcurrentTesters from '$lib/widgets/UC20ConcurrentTesters.svelte';
	import UC21MlInference from '$lib/widgets/UC21MlInference.svelte';

	let selectedEcu = $state<EcuId>('cvc');
	let selectedDtc = $state<DtcEntry | null>(null);
	let dtcPage = $state(0);
	let showHistorical = $state(false);
	let filteredCount = $state(0);
	let faultRefreshNonce = $state(0);
	let liveFaults = $state<DtcEntry[]>([]);

	const PAGE_SIZE = 5;

	onMount(() => {
		return subscribe((frame: TelemetryFrame) => {
			if (frame.type !== 'dtc') {
				return;
			}
			liveFaults = [frame.payload as DtcEntry, ...liveFaults].slice(0, 50);
		});
	});
</script>

<svelte:head>
	<title>Taktflow OpenSOVD - Dashboard</title>
</svelte:head>

<UC02DtcDetail dtc={selectedDtc} onClose={() => (selectedDtc = null)} />

<div class="flex min-h-screen flex-col gap-3 bg-background p-3 text-foreground">
	<header class="flex flex-wrap items-center justify-between gap-2 rounded-lg border border-border bg-card px-4 py-2">
		<div>
			<h1 class="text-base font-bold tracking-tight">Taktflow OpenSOVD</h1>
			<p class="text-[10px] text-muted-foreground">Capability Showcase Dashboard - ADR-0024 Stage 1</p>
		</div>
		<nav class="flex flex-wrap items-center gap-2 text-[11px]">
			<a
				href="https://taktflow-systems.com/"
				class="rounded border border-border bg-background px-2 py-1 font-semibold text-foreground hover:border-cyan-400 hover:text-cyan-300"
			>
				&larr; Taktflow Systems
			</a>
			<a
				href="/sovd/"
				class="rounded border border-border bg-background px-2 py-1 font-semibold text-foreground hover:border-cyan-400 hover:text-cyan-300"
			>
				Engineering Spec
			</a>
			<a
				href="/sovd/grafana/"
				class="rounded border border-border bg-background px-2 py-1 font-semibold text-foreground hover:border-emerald-400 hover:text-emerald-300"
			>
				Grafana &rarr;
			</a>
		</nav>
		<div class="text-[10px] text-muted-foreground">
			3-ECU bench: CVC + SC + BCM - ADR-0023
		</div>
	</header>

	<section>
		<p class="mb-1 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
			UC08 - Component Discovery + Capability Badges (FR-3.1, FR-3.4)
		</p>
		<UC08ComponentCards
			selectedId={selectedEcu}
			onSelect={(id) => {
				selectedEcu = id;
				dtcPage = 0;
			}}
		/>
	</section>

	<div class="grid flex-1 gap-3 lg:grid-cols-3">
		<div class="flex flex-col gap-3">
			<div class="rounded-lg border border-border bg-card/50 p-2">
				<p class="mb-1 text-[10px] font-semibold uppercase text-muted-foreground">
					UC01 Fault List (FR-1.1) - UC03 Clear - UC04 Pages
				</p>
				<UC01DtcList
					componentId={selectedEcu}
					page={dtcPage}
					pageSize={PAGE_SIZE}
					refreshNonce={faultRefreshNonce}
					onSelect={(dtc) => (selectedDtc = dtc)}
					onPage={(pageNumber) => (dtcPage = pageNumber)}
					onTotalChange={(total) => (filteredCount = total)}
				/>
				<div class="mt-2 flex items-center justify-between">
					<UC03ClearFaults
						componentId={selectedEcu}
						onCleared={() => {
							faultRefreshNonce += 1;
							liveFaults = liveFaults.filter((fault) => fault.component !== selectedEcu);
						}}
					/>
					<UC04Pagination
						total={filteredCount}
						pageSize={PAGE_SIZE}
						page={dtcPage}
						onPage={(pageNumber) => (dtcPage = pageNumber)}
					/>
				</div>
			</div>

			<div>
				<p class="mb-1 text-[10px] font-semibold uppercase text-muted-foreground">
					UC05 - Aggregated Fault Timeline (FR-1.5)
				</p>
				<UC05FaultsTimeline extraFaults={liveFaults} refreshNonce={faultRefreshNonce} />
			</div>

			<div class="rounded-lg border border-border bg-card/50 p-3">
				<p class="mb-2 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
					How it works
				</p>
				<ul class="space-y-1.5 text-[11px] leading-snug text-foreground/90">
					<li>
						<span class="font-semibold text-cyan-400">Browser</span> &rarr; Caddy TLS &rarr;
						<span class="font-semibold text-emerald-400">sovd-main</span> (Rust + axum, SQLite).
					</li>
					<li>
						Every widget above is a live fetch against
						<code class="rounded bg-muted px-1 py-0.5 text-[10px]">/sovd/v1/*</code> &mdash; ASAM
						SOVD v1.1 / ISO 17978-3.
					</li>
					<li>
						CDA bridge translates SOVD REST &rarr; UDS/DoIP for the physical ECU tier.
					</li>
					<li>
						DFM scores faults, publishes to MQTT; ws-bridge relays to this dashboard for live
						timeline updates.
					</li>
					<li>
						Prometheus + blackbox-exporter probe
						<code class="rounded bg-muted px-1 py-0.5 text-[10px]">/health</code>; Grafana panels
						under UC19.
					</li>
				</ul>
			</div>
		</div>

		<div class="flex flex-col gap-3">
			<div>
				<p class="mb-1 text-[10px] font-semibold uppercase text-muted-foreground">
					UC06 - Operations: Start/Stop/Poll (FR-2.1-2.3)
				</p>
				<UC06Operations componentId={selectedEcu} />
			</div>

			<div>
				<p class="mb-1 text-[10px] font-semibold uppercase text-muted-foreground">
					UC07 - Routine Catalog (FR-2.4)
				</p>
				<UC07RoutineCatalog componentId={selectedEcu} />
			</div>

			<div>
				<p class="mb-1 text-[10px] font-semibold uppercase text-muted-foreground">
					UC09 - HW/SW Version (FR-3.2)
				</p>
				<UC09HwSwVersion componentId={selectedEcu} />
			</div>

			<div>
				<p class="mb-1 text-[10px] font-semibold uppercase text-muted-foreground">
					UC10 - Live DID Reads @ 1 Hz (FR-3.3)
				</p>
				<UC10LiveDidReads componentId={selectedEcu} />
			</div>

			<div>
				<p class="mb-1 text-[10px] font-semibold uppercase text-muted-foreground">
					UP3-06 - ML Inference Scaffold (ADR-0028, ADR-0029)
				</p>
				<UC21MlInference componentId={selectedEcu} />
			</div>
		</div>

		<div class="flex flex-col gap-3">
			<div>
				<p class="mb-1 text-[10px] font-semibold uppercase text-muted-foreground">
					UC11 - Fault Pipeline Animation (FR-4.x)
				</p>
				<UC11FaultPipeline />
			</div>

			<div>
				<p class="mb-1 text-[10px] font-semibold uppercase text-muted-foreground">
					UC12 - Operation Cycle State (FR-4.3)
				</p>
				<UC12OperationCycle />
			</div>

			<div>
				<p class="mb-1 text-[10px] font-semibold uppercase text-muted-foreground">
					UC13 - DTC Lifecycle (Section 6.1)
				</p>
				<UC13DtcLifecycle />
			</div>

			<div>
				<p class="mb-1 text-[10px] font-semibold uppercase text-muted-foreground">
					UC14 - CDA Topology (FR-5.1, FR-5.2)
				</p>
				<UC14CdaTopology />
			</div>

			<div>
				<p class="mb-1 text-[10px] font-semibold uppercase text-muted-foreground">
					UC15 - Session (FR-7.1, FR-7.2)
				</p>
				<UC15Session />
			</div>

			<div>
				<p class="mb-1 text-[10px] font-semibold uppercase text-muted-foreground">
					UC17 - Safety Boundary (SR-1.x, SR-4.x)
				</p>
				<UC17SafetyBoundary />
			</div>

			<div>
				<p class="mb-1 text-[10px] font-semibold uppercase text-muted-foreground">
					UC18 - Gateway Routing (FR-6.1, FR-6.2)
				</p>
				<UC18GatewayRouting />
			</div>
		</div>
	</div>

	<section>
		<p class="mb-1 text-[10px] font-semibold uppercase text-muted-foreground">
			UC16 - Audit Log Stream (SEC-3.1)
		</p>
		<UC16AuditLog />
	</section>

	<section>
		<div class="mb-1 flex items-center gap-3">
			<p class="text-[10px] font-semibold uppercase text-muted-foreground">
				UC19 - Historical Trends (NFR-3.x - Prometheus)
			</p>
			<button
				onclick={() => (showHistorical = !showHistorical)}
				class="rounded border border-border px-2 py-0.5 text-[10px] hover:bg-accent"
			>
				{showHistorical ? 'Hide' : 'Show'} Historical
			</button>
		</div>
		<UC19Historical visible={showHistorical} grafanaUrl={import.meta.env.VITE_GRAFANA_URL ?? ''} />
	</section>

	<footer class="rounded-lg border border-border bg-card px-3 py-2">
		<p class="mb-1 text-[10px] font-semibold uppercase text-muted-foreground">
			UC20 - Concurrent Testers (NFR-1.3)
		</p>
		<UC20ConcurrentTesters />
	</footer>
</div>
