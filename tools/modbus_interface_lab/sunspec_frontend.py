#!/usr/bin/env python3
"""SunSpec model polling helpers for the browser test console."""

from __future__ import annotations

import math
import os
from datetime import datetime
from typing import Any


SUNSPEC_ADAPTER = "sunspec_modbus_tcp"


class SunSpecUnavailable(RuntimeError):
    """Raised when the optional SunSpec runtime is not installed."""


POINTS_BY_MODEL: dict[int, list[dict[str, str]]] = {
    1: [
        {"name": "Mn", "label": "Manufacturer", "kind": "raw"},
        {"name": "Md", "label": "Model", "kind": "raw"},
        {"name": "Vr", "label": "Firmware", "kind": "raw"},
        {"name": "SN", "label": "Serial", "kind": "raw"},
        {"name": "DA", "label": "Device address", "kind": "raw"},
    ],
    802: [
        {"name": "State", "label": "State", "kind": "enum"},
        {"name": "ChaSt", "label": "Charge state", "kind": "enum"},
        {"name": "LocRemCtl", "label": "Local/remote control", "kind": "enum"},
        {"name": "Typ", "label": "Battery type", "kind": "enum"},
        {"name": "SetOp", "label": "Set operation", "kind": "enum"},
        {"name": "SetInvState", "label": "Set inverter state", "kind": "enum"},
        {"name": "ReqInvState", "label": "Requested inverter state", "kind": "enum"},
        {"name": "V", "label": "Voltage", "kind": "computed", "unit": "V"},
        {"name": "A", "label": "Current", "kind": "computed", "unit": "A"},
        {"name": "W", "label": "Power", "kind": "computed", "unit": "W"},
        {"name": "Soc", "label": "SoC", "kind": "computed", "unit": "%"},
        {"name": "SoH", "label": "SoH", "kind": "computed", "unit": "%"},
        {"name": "DoD", "label": "DoD", "kind": "computed", "unit": "%"},
        {"name": "NCyc", "label": "Cycle count", "kind": "raw"},
        {"name": "AHRtg", "label": "Ah rating", "kind": "computed", "unit": "Ah"},
        {"name": "WHRtg", "label": "Wh rating", "kind": "computed", "unit": "Wh"},
        {"name": "AChaMax", "label": "Max charge current", "kind": "computed", "unit": "A"},
        {"name": "ADisChaMax", "label": "Max discharge current", "kind": "computed", "unit": "A"},
        {"name": "CellVMax", "label": "Cell voltage max", "kind": "computed", "unit": "V"},
        {"name": "CellVMaxStr", "label": "CellV max string", "kind": "raw"},
        {"name": "CellVMaxMod", "label": "CellV max module", "kind": "raw"},
        {"name": "CellVMin", "label": "Cell voltage min", "kind": "computed", "unit": "V"},
        {"name": "CellVMinStr", "label": "CellV min string", "kind": "raw"},
        {"name": "CellVMinMod", "label": "CellV min module", "kind": "raw"},
        {"name": "CellVAvg", "label": "Cell voltage avg", "kind": "computed", "unit": "V"},
        {"name": "Hb", "label": "Heartbeat", "kind": "raw"},
        {"name": "CtrlHb", "label": "Control heartbeat", "kind": "raw"},
        {"name": "Evt1", "label": "Event 1", "kind": "bits"},
        {"name": "Evt2", "label": "Event 2", "kind": "bits"},
        {"name": "EvtVnd1", "label": "Vendor event 1", "kind": "bits"},
        {"name": "EvtVnd2", "label": "Vendor event 2", "kind": "bits"},
    ],
    804: [
        {"name": "NMod", "label": "Module count", "kind": "raw"},
        {"name": "NCellBal", "label": "Balancing cell count", "kind": "raw"},
        {"name": "St", "label": "String status", "kind": "bits"},
        {"name": "ConSt", "label": "Contactor status", "kind": "bits"},
        {"name": "ConFail", "label": "Contactor failure", "kind": "enum"},
        {"name": "SetEna", "label": "Set enable", "kind": "enum"},
        {"name": "SetCon", "label": "Set contactor", "kind": "enum"},
        {"name": "V", "label": "String voltage", "kind": "computed", "unit": "V"},
        {"name": "A", "label": "String current", "kind": "computed", "unit": "A"},
        {"name": "Soc", "label": "String SoC", "kind": "computed", "unit": "%"},
        {"name": "SoH", "label": "String SoH", "kind": "computed", "unit": "%"},
        {"name": "DoD", "label": "String DoD", "kind": "computed", "unit": "%"},
        {"name": "CellVMax", "label": "Cell voltage max", "kind": "computed", "unit": "V"},
        {"name": "CellVMaxMod", "label": "CellV max module", "kind": "raw"},
        {"name": "CellVMin", "label": "Cell voltage min", "kind": "computed", "unit": "V"},
        {"name": "CellVMinMod", "label": "CellV min module", "kind": "raw"},
        {"name": "CellVAvg", "label": "Cell voltage avg", "kind": "computed", "unit": "V"},
        {"name": "ModTmpMax", "label": "Module temp max", "kind": "computed", "unit": "degC"},
        {"name": "ModTmpMaxMod", "label": "Max temp module", "kind": "raw"},
        {"name": "ModTmpMin", "label": "Module temp min", "kind": "computed", "unit": "degC"},
        {"name": "ModTmpMinMod", "label": "Min temp module", "kind": "raw"},
        {"name": "ModTmpAvg", "label": "Module temp avg", "kind": "computed", "unit": "degC"},
        {"name": "Evt1", "label": "Event 1", "kind": "bits"},
        {"name": "Evt2", "label": "Event 2", "kind": "bits"},
        {"name": "EvtVnd1", "label": "Vendor event 1", "kind": "bits"},
        {"name": "EvtVnd2", "label": "Vendor event 2", "kind": "bits"},
    ],
    805: [
        {"name": "ModIdx", "label": "Module index", "kind": "raw"},
        {"name": "NCell", "label": "Cell count", "kind": "raw"},
        {"name": "SoC", "label": "Module SoC", "kind": "computed", "unit": "%"},
        {"name": "SoH", "label": "Module SoH", "kind": "computed", "unit": "%"},
        {"name": "V", "label": "Module voltage", "kind": "computed", "unit": "V"},
        {"name": "CellVMax", "label": "Cell voltage max", "kind": "computed", "unit": "V"},
        {"name": "CellVMin", "label": "Cell voltage min", "kind": "computed", "unit": "V"},
        {"name": "CellTmpMax", "label": "Cell temp max", "kind": "computed", "unit": "degC"},
        {"name": "CellTmpMin", "label": "Cell temp min", "kind": "computed", "unit": "degC"},
        {"name": "NCellBal", "label": "Balancing cell count", "kind": "raw"},
    ],
    64093: [
        {"name": "SW_VER_MAJOR", "label": "SW major", "kind": "raw"},
        {"name": "SW_VER_MINOR", "label": "SW minor", "kind": "raw"},
        {"name": "SW_VER_PATCH", "label": "SW patch", "kind": "raw"},
        {"name": "HWVer", "label": "HW version", "kind": "raw"},
        {"name": "ActiveConfig", "label": "Active config", "kind": "enum"},
        {"name": "Ecu_Reset_Counter", "label": "Reset count", "kind": "raw"},
        {"name": "PowerSavingStatus", "label": "Power saving", "kind": "enum"},
        {"name": "IsoMonEnable", "label": "Isolation monitor enable", "kind": "raw"},
        {"name": "IsoMonStatus", "label": "Isolation monitor status", "kind": "enum"},
        {"name": "IsoMonResMea", "label": "Isolation resistance", "kind": "computed", "unit": "kOhm"},
        {"name": "DiscSwSt", "label": "Disconnect switch", "kind": "enum"},
        {"name": "DCDCStpOutP", "label": "DCDC stop output", "kind": "enum"},
        {"name": "SOCCalibSt", "label": "SOC calibration", "kind": "enum"},
        {"name": "DCBusV", "label": "DC bus voltage", "kind": "computed", "unit": "V"},
        {"name": "StrV", "label": "String voltage", "kind": "computed", "unit": "V"},
        {"name": "PCBTemp", "label": "PCB temp", "kind": "computed", "unit": "degC"},
        {"name": "DCPMTemp", "label": "DCPM temp", "kind": "computed", "unit": "degC"},
        {"name": "String_Actual_SOC", "label": "Actual SoC", "kind": "computed", "unit": "%"},
        {"name": "String_Actual_SOH", "label": "Actual SoH", "kind": "computed", "unit": "%"},
        {"name": "PDisChaMax", "label": "Max discharge power", "kind": "computed", "unit": "kW"},
        {"name": "PChaMax", "label": "Max charge power", "kind": "computed", "unit": "kW"},
        {"name": "PDisChaMaxInst", "label": "Instant max discharge power", "kind": "computed", "unit": "kW"},
        {"name": "PChaMaxInst", "label": "Instant max charge power", "kind": "computed", "unit": "kW"},
        {"name": "TotDisChaEnrgKWh", "label": "Discharged energy", "kind": "computed", "unit": "kWh"},
        {"name": "TotChaEnrgKWh", "label": "Charged energy", "kind": "computed", "unit": "kWh"},
        {"name": "DCIR", "label": "DCIR", "kind": "computed", "unit": "mOhm"},
        {"name": "TotNo_L2Faults", "label": "L2 fault count", "kind": "raw"},
        {"name": "No_L3UVP", "label": "L3 UVP count", "kind": "raw"},
        {"name": "No_L3OVP", "label": "L3 OVP count", "kind": "raw"},
        {"name": "No_L3OTP", "label": "L3 OTP count", "kind": "raw"},
        {"name": "No_L3OCP", "label": "L3 OCP count", "kind": "raw"},
        {"name": "No_L3ODP", "label": "L3 ODP count", "kind": "raw"},
        {"name": "CusEvn1", "label": "Custom event 1", "kind": "bits"},
        {"name": "CusEvn2", "label": "Custom event 2", "kind": "bits"},
        {"name": "CusEvn3", "label": "Custom event 3", "kind": "bits"},
        {"name": "CusEvn4", "label": "Custom event 4", "kind": "bits"},
        {"name": "CusEvn5", "label": "Custom event 5", "kind": "bits"},
    ],
}

MODEL_804_GROUP_POINTS = [
    {"name": "ModNCell", "label": "Cell count", "kind": "raw"},
    {"name": "ModSoc", "label": "Module SoC", "kind": "computed", "unit": "%"},
    {"name": "ModCellVMax", "label": "Cell voltage max", "kind": "computed", "unit": "V"},
    {"name": "ModCellVMin", "label": "Cell voltage min", "kind": "computed", "unit": "V"},
    {"name": "ModCellTmpMax", "label": "Cell temp max", "kind": "computed", "unit": "degC"},
    {"name": "ModCellTmpMin", "label": "Cell temp min", "kind": "computed", "unit": "degC"},
]


def load_sunspec_modules() -> tuple[Any, Any]:
    try:
        import sunspec2.device as device
        import sunspec2.modbus.client as client
    except ImportError as exc:
        raise SunSpecUnavailable(
            "SunSpec adapter requires the optional Python package 'sunspec2'. "
            "Install it in the same environment that runs interface_console.py."
        ) from exc
    return device, client


def register_vendor_model_path(device: Any) -> str | None:
    try:
        import service_vehicle_customer_interfaces_badger as vendor_pkg
    except ImportError:
        return None

    vendor_dir = os.path.dirname(vendor_pkg.__file__)
    paths = list(device.get_model_defs_path())
    if vendor_dir not in paths:
        paths.append(vendor_dir)
        device.set_model_defs_path(paths)
    return vendor_dir


def connect_and_scan(ip: str, port: int, slave_id: int) -> tuple[Any, str | None]:
    device, client = load_sunspec_modules()
    vendor_model_path = register_vendor_model_path(device)
    bms = client.SunSpecModbusClientDeviceTCP(slave_id=slave_id, ipaddr=ip, ipport=port)
    bms.scan()
    return bms, vendor_model_path


def close_device(bms: Any) -> None:
    disconnect = getattr(bms, "disconnect", None)
    if callable(disconnect):
        disconnect()


def model_layout(bms: Any) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    seen: set[tuple[Any, Any, Any]] = set()
    models = getattr(bms, "models", {}) or {}
    for model_id, model_list in sorted(models.items(), key=lambda item: (0, item[0]) if isinstance(item[0], int) else (1, str(item[0]))):
        for index, model in enumerate(model_list):
            key = (model_id, getattr(model, "model_addr", None), getattr(model, "model_len", None))
            if key in seen:
                continue
            seen.add(key)
            rows.append(
                {
                    "model_id": model_id,
                    "instance": index + 1,
                    "name": getattr(model, "name", "") or "",
                    "start_address": getattr(model, "model_addr", None),
                    "length": getattr(model, "model_len", None),
                }
            )
    return rows


def poll_summary(bms: Any) -> dict[str, Any]:
    models = getattr(bms, "models", {}) or {}
    points: list[dict[str, Any]] = []
    models_polled: list[int] = []

    for model_id in (1, 802, 804, 64093):
        model_list = models.get(model_id, [])
        if not model_list:
            continue
        model = model_list[0]
        model.read()
        models_polled.append(model_id)
        points.extend(points_for_model(model_id, model, 1))
        if model_id == 804:
            points.extend(points_for_model_804_groups(model))

    for index, model in enumerate(models.get(805, []), 1):
        model.read()
        if 805 not in models_polled:
            models_polled.append(805)
        points.extend(points_for_model(805, model, index))

    return {
        "timestamp": datetime.now().isoformat(timespec="milliseconds"),
        "models_polled": models_polled,
        "points": points,
    }


def points_for_model(model_id: int, model: Any, instance: int) -> list[dict[str, Any]]:
    model_name = getattr(model, "name", "") or f"Model {model_id}"
    return [
        point_payload(model, model_id, model_name, instance, point_def)
        for point_def in POINTS_BY_MODEL.get(model_id, [])
        if hasattr(model, point_def["name"])
    ]


def points_for_model_804_groups(model: Any) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    modules = getattr(model, "lithium_ion_string_module", []) or []
    for index, module in enumerate(modules, 1):
        for point_def in MODEL_804_GROUP_POINTS:
            if hasattr(module, point_def["name"]):
                rows.append(
                    point_payload(
                        module,
                        804,
                        getattr(model, "name", "") or "Model 804",
                        1,
                        point_def,
                        group="lithium_ion_string_module",
                        group_index=index,
                    )
                )
    return rows


def point_payload(
    source: Any,
    model_id: int,
    model_name: str,
    instance: int,
    point_def: dict[str, str],
    group: str | None = None,
    group_index: int | None = None,
) -> dict[str, Any]:
    point_name = point_def["name"]
    point = getattr(source, point_name)
    kind = point_def.get("kind", "raw")
    raw = safe_json_value(getattr(point, "value", None))
    computed = safe_json_value(getattr(point, "cvalue", None))
    value = computed if kind == "computed" and computed is not None else raw
    label_parts = [f"M{model_id}"]
    if instance > 1:
        label_parts.append(f"#{instance}")
    if group_index is not None:
        label_parts.append(f"module {group_index}")
    label_parts.append(point_def.get("label") or point_name)
    text = point_text(point, kind, value, point_def.get("unit", ""))
    return {
        "model_id": model_id,
        "model_name": model_name,
        "instance": instance,
        "group": group,
        "group_index": group_index,
        "point": point_name,
        "label": " ".join(label_parts),
        "kind": kind,
        "unit": point_def.get("unit", ""),
        "raw": raw,
        "value": value,
        "text": text,
    }


def point_text(point: Any, kind: str, value: Any, unit: str) -> str:
    if value is None:
        return "N/A"
    if kind == "enum":
        symbol = symbol_name(point, value)
        return f"{value} ({symbol})" if symbol else str(value)
    if kind == "bits":
        try:
            int_value = int(value)
        except (TypeError, ValueError):
            return str(value)
        names = set_bit_names(point, int_value)
        if names:
            return f"0x{int_value:08X} [{', '.join(names)}]"
        if int_value == 0:
            return f"0x{int_value:08X} [clear]"
        bits = ", ".join(str(index) for index in range(32) if int_value & (1 << index))
        return f"0x{int_value:08X} [bits: {bits}]"
    return f"{value} {unit}".strip()


def symbol_name(point: Any, value: Any) -> str | None:
    try:
        int_value = int(value)
    except (TypeError, ValueError):
        return None
    for symbol in point_symbols(point):
        if symbol.get("value") == int_value:
            return str(symbol.get("name"))
    return None


def set_bit_names(point: Any, value: int) -> list[str]:
    names = []
    for symbol in point_symbols(point):
        symbol_value = symbol.get("value")
        if isinstance(symbol_value, int) and symbol_value != 0 and (value & symbol_value):
            names.append(str(symbol.get("name")))
    return names


def point_symbols(point: Any) -> list[dict[str, Any]]:
    pdef = getattr(point, "pdef", {}) or {}
    symbols = pdef.get("symbols", [])
    return [symbol for symbol in symbols if isinstance(symbol, dict)] if isinstance(symbols, list) else []


def safe_json_value(value: Any) -> Any:
    if isinstance(value, float) and not math.isfinite(value):
        return str(value)
    if value is None or isinstance(value, (str, int, float, bool)):
        return value
    try:
        numeric = float(value)
        return numeric if math.isfinite(numeric) else str(value)
    except (TypeError, ValueError):
        return str(value)
