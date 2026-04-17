<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- UC15 — Session id, security level, timeout countdown (FR-7.1, FR-7.2) -->
<script lang="ts">
	import type { SessionInfo } from '$lib/types/sovd';
	import { CANNED_SESSION } from '$lib/api/sovdClient';
	import { onMount, onDestroy } from 'svelte';

	let session = $state<SessionInfo>({ ...CANNED_SESSION });
	let remaining = $state(0);
	let timer: ReturnType<typeof setInterval> | null = null;

	function calcRemaining(): number {
		return Math.max(0, Math.floor((new Date(session.expiresAt).getTime() - Date.now()) / 1000));
	}

	onMount(() => {
		remaining = calcRemaining();
		timer = setInterval(() => {
			remaining = calcRemaining();
			// Refresh session stub every ~120 s
			if (remaining === 0) {
				session = { ...CANNED_SESSION, expiresAt: new Date(Date.now() + 120_000).toISOString() };
				remaining = 120;
			}
		}, 1000);
	});

	onDestroy(() => {
		if (timer) clearInterval(timer);
	});

	const LEVEL_COLOR: Record<string, string> = {
		default: 'text-slate-300',
		programming: 'text-yellow-300',
		extended: 'text-blue-300'
	};

	function secBar(level: number): string {
		const filled = '█'.repeat(level);
		const empty = '░'.repeat(3 - level);
		return filled + empty;
	}
</script>

<div class="rounded-lg border border-border bg-card p-3 text-xs">
	<h3 class="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
		Session
	</h3>
	<dl class="space-y-0.5">
		<div class="flex justify-between">
			<dt class="text-muted-foreground">Session ID</dt>
			<dd class="font-mono">{session.sessionId}</dd>
		</div>
		<div class="flex justify-between">
			<dt class="text-muted-foreground">Level</dt>
			<dd class="font-semibold {LEVEL_COLOR[session.level]}">{session.level}</dd>
		</div>
		<div class="flex justify-between">
			<dt class="text-muted-foreground">Security</dt>
			<dd class="font-mono">{secBar(session.securityLevel)} L{session.securityLevel}</dd>
		</div>
		<div class="flex justify-between">
			<dt class="text-muted-foreground">Expires in</dt>
			<dd
				class="tabular-nums font-semibold {remaining < 30
					? 'text-red-400 animate-pulse'
					: 'text-green-400'}"
			>
				{remaining}s
			</dd>
		</div>
	</dl>
</div>
