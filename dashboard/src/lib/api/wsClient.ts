// SPDX-License-Identifier: Apache-2.0
// WebSocket client for the ADR-0024 ws-bridge with auto-reconnect.
// Falls back to a simulator when the bridge is unavailable.

import type { TelemetryFrame } from '$lib/types/sovd';
import { CANNED_DTCS, telemetryPayloadToDtc } from './sovdClient';

type Listener = (frame: TelemetryFrame) => void;

const RECONNECT_DELAY_MS = 3000;
const DEFAULT_DEV_PORT = '8080';

let ws: WebSocket | null = null;
let listeners: Listener[] = [];
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
let simulatorTimer: ReturnType<typeof setInterval> | null = null;
let connected = false;
let shouldReconnect = true;

function sessionValue(key: string): string | null {
	if (typeof window === 'undefined') {
		return null;
	}
	try {
		return window.sessionStorage.getItem(key);
	} catch {
		return null;
	}
}

function defaultWsUrl(): string {
	if (typeof window === 'undefined') {
		return `ws://localhost:${DEFAULT_DEV_PORT}/ws`;
	}
	const isLocalDev = window.location.port === '5173' || window.location.port === '4173';
	if (isLocalDev) {
		return `ws://${window.location.hostname}:${DEFAULT_DEV_PORT}/ws`;
	}
	return `${window.location.origin.replace(/^http/, 'ws')}/ws`;
}

function configuredWsUrl(): string {
	if (typeof window === 'undefined') {
		return `ws://localhost:${DEFAULT_DEV_PORT}/ws`;
	}
	const raw = import.meta.env.VITE_WS_URL ?? defaultWsUrl();
	const url = new URL(raw, window.location.href);
	const token = import.meta.env.VITE_WS_TOKEN ?? sessionValue('wsBridgeToken') ?? '';
	if (token && !url.searchParams.has('token')) {
		url.searchParams.set('token', token);
	}
	return url.toString();
}

export function subscribe(fn: Listener): () => void {
	listeners.push(fn);
	return () => {
		listeners = listeners.filter((listener) => listener !== fn);
	};
}

function emit(frame: TelemetryFrame): void {
	for (const listener of listeners) {
		listener(frame);
	}
}

function startSimulator(): void {
	if (simulatorTimer) return;
	let tick = 0;
	simulatorTimer = setInterval(() => {
		tick++;
		if (tick % 3 === 0) {
			const dtc = CANNED_DTCS[tick % CANNED_DTCS.length];
			emit({ type: 'dtc', payload: dtc, timestamp: new Date().toISOString() });
			return;
		}
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
	}, 2000);
}

function stopSimulator(): void {
	if (!simulatorTimer) return;
	clearInterval(simulatorTimer);
	simulatorTimer = null;
}

function parseIncoming(raw: string): TelemetryFrame | null {
	try {
		const parsed = JSON.parse(raw) as Record<string, unknown>;
		if (
			typeof parsed.type === 'string' &&
			typeof parsed.timestamp === 'string' &&
			'payload' in parsed
		) {
			return {
				type: parsed.type as TelemetryFrame['type'],
				payload: parsed.payload,
				timestamp: parsed.timestamp
			};
		}
		if (typeof parsed.topic !== 'string') {
			return null;
		}
		const timestamp =
			parsed.payload &&
			typeof parsed.payload === 'object' &&
			typeof (parsed.payload as Record<string, unknown>).timestamp === 'string'
				? ((parsed.payload as Record<string, unknown>).timestamp as string)
				: new Date().toISOString();
		if (parsed.topic.startsWith('vehicle/dtc/')) {
			const dtc = telemetryPayloadToDtc(parsed.payload);
			if (dtc) {
				return {
					type: 'dtc',
					payload: dtc,
					timestamp: dtc.lastSeen
				};
			}
		}
		return {
			type: 'health',
			payload: {
				topic: parsed.topic,
				payload: parsed.payload
			},
			timestamp
		};
	} catch {
		return null;
	}
}

export function connect(): void {
	if (typeof window === 'undefined' || ws) return;
	shouldReconnect = true;
	try {
		ws = new WebSocket(configuredWsUrl());
		ws.onopen = () => {
			connected = true;
			stopSimulator();
		};
		ws.onmessage = (event) => {
			if (typeof event.data !== 'string') {
				return;
			}
			const frame = parseIncoming(event.data);
			if (frame) {
				emit(frame);
			}
		};
		ws.onclose = () => {
			ws = null;
			connected = false;
			startSimulator();
			if (shouldReconnect) {
				reconnectTimer = setTimeout(connect, RECONNECT_DELAY_MS);
			}
		};
		ws.onerror = () => {
			ws?.close();
		};
	} catch {
		startSimulator();
	}
}

export function disconnect(): void {
	shouldReconnect = false;
	if (reconnectTimer) {
		clearTimeout(reconnectTimer);
		reconnectTimer = null;
	}
	stopSimulator();
	connected = false;
	ws?.close();
	ws = null;
}

export function isConnected(): boolean {
	return connected;
}
