<!-- SPDX-License-Identifier: Apache-2.0 -->
<script lang="ts">
	import '../app.css';
	import { onMount } from 'svelte';
	import { connect } from '$lib/api/wsClient';

	interface Props {
		children: import('svelte').Snippet;
	}

	let { children }: Props = $props();

	onMount(() => {
		connect();
	});
</script>

{@render children()}
