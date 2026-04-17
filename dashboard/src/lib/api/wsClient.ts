// SPDX-License-Identifier: Apache-2.0
// WebSocket client for /ws/telemetry with auto-reconnect.
// Stub: emits simulated frames every 2 s when real WS is unavailable.

import type { TelemetryFrame } from '$lib/types/sovd';
import { CANNED_DTCS } from './sovdClient';

type Listener = (frame: TelemetryFrame) => void;

const WS_URL =
	typeof window !== 'undefined'
		? (import.meta.env.VITE_WS_URL ?? `ws://${window.location.hostname}:8080/ws/telemetry`)
		: 'ws://localhost:8080/ws/telemetry';

const RECONNECT_DELAY_MS = 3000;

let ws: WebSocket | null = null;
let listeners: Listener[] = [];
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
let simulatorTimer: ReturnType<typeof setInterval> | null = null;
let connected = false;

export function subscribe(fn: Listener): () => void {
	listeners.push(fn);
	return () => {
		listeners = listeners.filter((l) => l !== fn);
	};
}

function emit(frame: TelemetryFrame): void {
	for (const l of listeners) l(frame);
}

function startSimulator(): void {
	if (simulatorTimer) return;
	let tick = 0;
	simulatorTimer = setInterval(() => {
		tick++;
		// Alternate between DTC and DID frames
		if (tick % 3 === 0) {
			const dtc = CANNED_DTCS[tick % CANNED_DTCS.length];
			emit({ type: 'dtc', payload: dtc, timestamp: new Date().toISOString() });
		} else {
			emit({
				type: 'did',
				payload: {
					component: (['cvc', 'sc', 'bcm'] as const)[tick % 3],
					batteryVoltage: +(12.5 + Math.random() * 2).toFixed(2),
					temperature: +(35 + Math.random() * 15).toFixed(1),
					vin: 'WBA3A5G59ENP26705',
					timestamp: new Date().toISOString()
				},
				timestamp: new Date().toISOString()
			});
		}
	}, 2000);
}

function stopSimulator(): void {
	if (simulatorTimer) {
		clearInterval(simulatorTimer);
		simulatorTimer = null;
	}
}

export function connect(): void {
	if (typeof window === 'undefined') return;
	try {
		ws = new WebSocket(WS_URL);
		ws.onopen = () => {
			connected = true;
			stopSimulator();
		};
		ws.onmessage = (ev) => {
			try {
				// eslint-disable-next-line @typescript-eslint/no-unsafe-argument
				const frame: TelemetryFrame = JSON.parse(ev.data);
				emit(frame);
			} catch {
				// malformed frame — ignore
			}
		};
		ws.onclose = () => {
			connected = false;
			startSimulator();
			reconnectTimer = setTimeout(connect, RECONNECT_DELAY_MS);
		};
		ws.onerror = () => {
			ws?.close();
		};
	} catch {
		// WebSocket unavailable in SSR or restricted env — fall back to simulator
		startSimulator();
	}
}

export function disconnect(): void {
	if (reconnectTimer) clearTimeout(reconnectTimer);
	stopSimulator();
	ws?.close();
	ws = null;
	connected = false;
}

export function isConnected(): boolean {
	return connected;
}
