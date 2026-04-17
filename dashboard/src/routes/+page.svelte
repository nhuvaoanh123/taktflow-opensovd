<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- Taktflow OpenSOVD — Capability Showcase Dashboard (ADR-0024 Stage 1) -->
<!-- Composes all 20 UC widgets per T24.1.7 -->
<script lang="ts">
	import type { DtcEntry, EcuId } from '$lib/types/sovd';

	// Widgets
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

	import { CANNED_DTCS } from '$lib/api/sovdClient';

	let selectedEcu = $state<EcuId>('cvc');
	let selectedDtc = $state<DtcEntry | null>(null);
	let dtcPage = $state(0);
	let showHistorical = $state(false);

	const PAGE_SIZE = 5;
	const filteredCount = $derived(CANNED_DTCS.filter((d) => d.component === selectedEcu).length);
</script>

<svelte:head>
	<title>Taktflow OpenSOVD — Dashboard</title>
</svelte:head>

<!-- UC02 DTC Detail Modal (floats above everything) -->
<UC02DtcDetail dtc={selectedDtc} onClose={() => (selectedDtc = null)} />

<div class="flex min-h-screen flex-col gap-3 bg-background p-3 text-foreground">

	<!-- ===== HEADER ===== -->
	<header class="flex items-center justify-between rounded-lg border border-border bg-card px-4 py-2">
		<div>
			<h1 class="text-base font-bold tracking-tight">Taktflow OpenSOVD</h1>
			<p class="text-[10px] text-muted-foreground">Capability Showcase Dashboard · ADR-0024 Stage 1</p>
		</div>
		<div class="text-[10px] text-muted-foreground">
			3-ECU bench: CVC + SC + BCM · ADR-0023
		</div>
	</header>

	<!-- ===== TOP BAR — UC08 Component Cards ===== -->
	<section>
		<p class="mb-1 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
			UC08 — Component Discovery + Capability Badges (FR-3.1, FR-3.4)
		</p>
		<UC08ComponentCards selectedId={selectedEcu} onSelect={(id) => { selectedEcu = id; dtcPage = 0; }} />
	</section>

	<!-- ===== MAIN 3-COLUMN LAYOUT ===== -->
	<div class="grid flex-1 gap-3 lg:grid-cols-3">

		<!-- ===== LEFT COLUMN — Faults ===== -->
		<div class="flex flex-col gap-3">
			<div class="rounded-lg border border-border bg-card/50 p-2">
				<p class="mb-1 text-[10px] font-semibold uppercase text-muted-foreground">
					UC01 Fault List (FR-1.1) · UC03 Clear · UC04 Pages
				</p>
				<UC01DtcList componentId={selectedEcu} onSelect={(dtc) => (selectedDtc = dtc)} />
				<div class="mt-2 flex items-center justify-between">
					<UC03ClearFaults componentId={selectedEcu} />
					<UC04Pagination
						total={filteredCount}
						pageSize={PAGE_SIZE}
						page={dtcPage}
						onPage={(p) => (dtcPage = p)}
					/>
				</div>
			</div>

			<div>
				<p class="mb-1 text-[10px] font-semibold uppercase text-muted-foreground">
					UC05 — Aggregated Fault Timeline (FR-1.5)
				</p>
				<UC05FaultsTimeline />
			</div>
		</div>

		<!-- ===== MIDDLE COLUMN — Operations + Data ===== -->
		<div class="flex flex-col gap-3">
			<div>
				<p class="mb-1 text-[10px] font-semibold uppercase text-muted-foreground">
					UC06 — Operations: Start/Stop/Poll (FR-2.1-2.3)
				</p>
				<UC06Operations componentId={selectedEcu} />
			</div>

			<div>
				<p class="mb-1 text-[10px] font-semibold uppercase text-muted-foreground">
					UC07 — Routine Catalog (FR-2.4)
				</p>
				<UC07RoutineCatalog componentId={selectedEcu} />
			</div>

			<div>
				<p class="mb-1 text-[10px] font-semibold uppercase text-muted-foreground">
					UC09 — HW/SW Version (FR-3.2)
				</p>
				<UC09HwSwVersion componentId={selectedEcu} />
			</div>

			<div>
				<p class="mb-1 text-[10px] font-semibold uppercase text-muted-foreground">
					UC10 — Live DID Reads @ 1 Hz (FR-3.3)
				</p>
				<UC10LiveDidReads componentId={selectedEcu} />
			</div>
		</div>

		<!-- ===== RIGHT COLUMN — Pipeline / Topology / Session / Safety ===== -->
		<div class="flex flex-col gap-3">
			<div>
				<p class="mb-1 text-[10px] font-semibold uppercase text-muted-foreground">
					UC11 — Fault Pipeline Animation (FR-4.x)
				</p>
				<UC11FaultPipeline />
			</div>

			<div>
				<p class="mb-1 text-[10px] font-semibold uppercase text-muted-foreground">
					UC12 — Operation Cycle State (FR-4.3)
				</p>
				<UC12OperationCycle />
			</div>

			<div>
				<p class="mb-1 text-[10px] font-semibold uppercase text-muted-foreground">
					UC13 — DTC Lifecycle (§6.1)
				</p>
				<UC13DtcLifecycle />
			</div>

			<div>
				<p class="mb-1 text-[10px] font-semibold uppercase text-muted-foreground">
					UC14 — CDA Topology (FR-5.1, FR-5.2)
				</p>
				<UC14CdaTopology />
			</div>

			<div>
				<p class="mb-1 text-[10px] font-semibold uppercase text-muted-foreground">
					UC15 — Session (FR-7.1, FR-7.2)
				</p>
				<UC15Session />
			</div>

			<div>
				<p class="mb-1 text-[10px] font-semibold uppercase text-muted-foreground">
					UC17 — Safety Boundary (SR-1.x, SR-4.x)
				</p>
				<UC17SafetyBoundary />
			</div>

			<div>
				<p class="mb-1 text-[10px] font-semibold uppercase text-muted-foreground">
					UC18 — Gateway Routing (FR-6.1, FR-6.2)
				</p>
				<UC18GatewayRouting />
			</div>
		</div>
	</div>

	<!-- ===== BOTTOM BAR — UC16 Audit Log ===== -->
	<section>
		<p class="mb-1 text-[10px] font-semibold uppercase text-muted-foreground">
			UC16 — Audit Log Stream (SEC-3.1)
		</p>
		<UC16AuditLog />
	</section>

	<!-- ===== UC19 Historical pane toggle ===== -->
	<section>
		<div class="flex items-center gap-3 mb-1">
			<p class="text-[10px] font-semibold uppercase text-muted-foreground">
				UC19 — Historical Trends (NFR-3.x · Prometheus)
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

	<!-- ===== FOOTER — UC20 Concurrent Testers ===== -->
	<footer class="rounded-lg border border-border bg-card px-3 py-2">
		<p class="mb-1 text-[10px] font-semibold uppercase text-muted-foreground">
			UC20 — Concurrent Testers (NFR-1.3)
		</p>
		<UC20ConcurrentTesters />
	</footer>
</div>
