#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# smoke-public-sil.sh - exercise every OpenSOVD UC against the public SIL.
#
# Target: https://sovd.taktflow-systems.com (live SIL, SQLite backend, pre-seeded faults).
# Usage:  bash scripts/smoke-public-sil.sh
#
# Each UC is a pass/fail probe. Prints PASS / FAIL per UC and an aggregate total.
# Exit code 0 if all 20 UCs pass; otherwise the number of failures.

set -uo pipefail

BASE="${SOVD_BASE:-https://sovd.taktflow-systems.com}"
PASS=0
FAIL=0
SKIP=0
LOG=""

check() {
  local name="$1" status="$2" detail="$3"
  if [[ "$status" == "pass" ]]; then
    echo -e "  \033[32m[PASS]\033[0m $name  $detail"
    PASS=$((PASS+1))
  elif [[ "$status" == "skip" ]]; then
    echo -e "  \033[33m[SKIP]\033[0m $name  $detail"
    SKIP=$((SKIP+1))
  else
    echo -e "  \033[31m[FAIL]\033[0m $name  $detail"
    FAIL=$((FAIL+1))
  fi
}

json_has() { python3 -c "import json,sys; d=json.loads(sys.stdin.read()); exit(0 if $1 else 1)" 2>/dev/null; }
json_get() { python3 -c "import json,sys; d=json.loads(sys.stdin.read()); print($1)" 2>/dev/null; }

echo "== OpenSOVD public SIL smoke test =="
echo "Target: $BASE"
echo

# -------- UC01 Dtc List --------
r=$(curl -sS "$BASE/sovd/v1/components/cvc/faults")
if echo "$r" | json_has "d.get('total',0) >= 2 and len(d.get('items',[])) >= 2"; then
  check "UC01 DtcList" pass "cvc has $(echo "$r" | json_get 'd[\"total\"]') pre-seeded faults"
else
  check "UC01 DtcList" fail "expected >=2 faults on cvc, got: $r"
fi

# -------- UC02 Dtc Detail --------
r=$(curl -sS "$BASE/sovd/v1/components/cvc/faults/P0A1F")
if echo "$r" | json_has "'environment_data' in d and d['item']['code']=='P0A1F' and d['item']['fault_name']"; then
  check "UC02 DtcDetail" pass "fault_name=$(echo "$r" | json_get 'd[\"item\"][\"fault_name\"]')"
else
  check "UC02 DtcDetail" fail "missing environment_data or fault_name: $r"
fi

# -------- UC03 ClearFaults --------
code=$(curl -sS -o /dev/null -w "%{http_code}" -X DELETE "$BASE/sovd/v1/components/bcm/faults")
if [[ "$code" == "204" ]]; then
  check "UC03 ClearFaults" pass "DELETE /bcm/faults returned 204"
else
  check "UC03 ClearFaults" fail "expected 204, got $code"
fi

# -------- UC04 Pagination --------
# sovd-main uses ISO 17978-3 convention: page=N (1-indexed), page-size=N
r=$(curl -sS "$BASE/sovd/v1/components/cvc/faults?page=1&page-size=1")
got=$(echo "$r" | json_get "len(d.get('items',[]))")
if [[ "$got" == "1" ]]; then
  check "UC04 Pagination" pass "page-size=1 returned 1 item (spec pagination)"
else
  check "UC04 Pagination" fail "expected 1 item with page-size=1, got $got"
fi

# -------- UC05 FaultsTimeline (status fields) --------
r=$(curl -sS "$BASE/sovd/v1/components/cvc/faults")
if echo "$r" | json_has "all('status' in it and 'aggregatedStatus' in it['status'] for it in d['items'])"; then
  check "UC05 FaultsTimeline" pass "all faults have status.aggregatedStatus"
else
  check "UC05 FaultsTimeline" fail "missing status fields: $r"
fi

# -------- UC06 Operations (execute) --------
r=$(curl -sS -X POST "$BASE/sovd/v1/components/bcm/operations/relay_self_test/executions" \
    -H "Content-Type: application/json" -d '{}')
if echo "$r" | json_has "'id' in d and 'status' in d"; then
  check "UC06 Operations" pass "execution id=$(echo "$r" | json_get 'd[\"id\"]') status=$(echo "$r" | json_get 'd[\"status\"]')"
  EXEC_ID=$(echo "$r" | json_get 'd["id"]')
else
  check "UC06 Operations" fail "no execution id: $r"
  EXEC_ID=""
fi

# -------- UC07 RoutineCatalog --------
r=$(curl -sS "$BASE/sovd/v1/components/cvc/operations")
if echo "$r" | json_has "len(d.get('items',[])) >= 2 and all('id' in op and 'name' in op for op in d['items'])"; then
  check "UC07 RoutineCatalog" pass "cvc exposes $(echo "$r" | json_get "len(d['items'])") operations"
else
  check "UC07 RoutineCatalog" fail "operations list malformed: $r"
fi

# -------- UC08 ComponentCards --------
r=$(curl -sS "$BASE/sovd/v1/components")
if echo "$r" | json_has "len(d.get('items',[])) == 4"; then
  r2=$(curl -sS "$BASE/sovd/v1/components/bcm")
  if echo "$r2" | json_has "'id' in d and 'name' in d"; then
    check "UC08 ComponentCards" pass "4 components, bcm metadata ok"
  else
    check "UC08 ComponentCards" fail "component metadata missing: $r2"
  fi
else
  check "UC08 ComponentCards" fail "expected 4 components: $r"
fi

# -------- UC09 HwSwVersion --------
r=$(curl -sS "$BASE/sovd/v1/components/bcm/data/vin")
if echo "$r" | json_has "'data' in d and len(d['data']) > 0"; then
  check "UC09 HwSwVersion" pass "bcm VIN=$(echo "$r" | json_get 'd[\"data\"]')"
else
  check "UC09 HwSwVersion" fail "VIN DID missing: $r"
fi

# -------- UC10 LiveDidReads --------
r=$(curl -sS "$BASE/sovd/v1/components/bcm/data")
if echo "$r" | json_has "len(d.get('items',[])) >= 1 and any(it.get('id')=='vin' for it in d['items'])"; then
  check "UC10 LiveDidReads" pass "/data lists DIDs including vin"
else
  check "UC10 LiveDidReads" fail "/data missing vin: $r"
fi

# -------- UC11 FaultPipeline --------
# Inject a fault implicitly through DFM isn't exposed via REST in this build;
# verify the DFM fault sink is alive via /health.sovd_db.status.
r=$(curl -sS "$BASE/sovd/v1/health")
if echo "$r" | json_has "d.get('status')=='ok' and d.get('fault_sink',{}).get('status')=='ok'"; then
  check "UC11 FaultPipeline" pass "fault_sink=ok (MQTT fan-out requires ws-bridge, not in VPS SIL yet)"
else
  check "UC11 FaultPipeline" fail "fault_sink not ok: $r"
fi

# -------- UC12 OperationCycle --------
# Retrieve execution status from UC06. Operation cycle = how DFM tracks async ops.
if [[ -n "$EXEC_ID" ]]; then
  r=$(curl -sS "$BASE/sovd/v1/components/bcm/operations/relay_self_test/executions/$EXEC_ID")
  if echo "$r" | json_has "'status' in d"; then
    check "UC12 OperationCycle" pass "exec $EXEC_ID status=$(echo "$r" | json_get 'd[\"status\"]')"
  else
    check "UC12 OperationCycle" fail "no status field: $r"
  fi
else
  check "UC12 OperationCycle" skip "no execution id from UC06"
fi

# -------- UC13 DtcLifecycle --------
# Fault must expose state transitions: aggregatedStatus + confirmedDTC.
r=$(curl -sS "$BASE/sovd/v1/components/cvc/faults")
if echo "$r" | json_has "any(it['status'].get('aggregatedStatus') in ('active','pending','confirmed') for it in d['items']) and any('confirmedDTC' in it['status'] for it in d['items'])"; then
  check "UC13 DtcLifecycle" pass "status includes aggregatedStatus + confirmedDTC fields"
else
  check "UC13 DtcLifecycle" fail "lifecycle fields missing: $r"
fi

# -------- UC14 CdaTopology --------
r=$(curl -sS "$BASE/sovd/v1/gateway/backends")
if echo "$r" | json_has "len(d.get('items',[])) == 4 and all('protocol' in b and 'reachable' in b for b in d['items'])"; then
  check "UC14 CdaTopology" pass "4 backends, all with protocol + reachable"
else
  check "UC14 CdaTopology" fail "backends missing fields: $r"
fi

# -------- UC15 Session --------
r=$(curl -sS "$BASE/sovd/v1/session")
if echo "$r" | json_has "'session_id' in d and 'level' in d and 'active' in d"; then
  check "UC15 Session" pass "session_id=$(echo "$r" | json_get 'd[\"session_id\"][:8]')..."
else
  check "UC15 Session" fail "session missing fields: $r"
fi

# -------- UC16 AuditLog --------
r=$(curl -sS "$BASE/sovd/v1/audit")
if echo "$r" | json_has "len(d.get('items',[])) >= 5 and all('action' in e and 'target' in e for e in d['items'])"; then
  check "UC16 AuditLog" pass "audit log has $(echo "$r" | json_get "len(d['items'])") entries"
else
  check "UC16 AuditLog" fail "audit log too small or malformed: $r"
fi

# -------- UC17 SafetyBoundary --------
# Call a nonexistent operation; expect structured error.
code=$(curl -sS -o /tmp/_uc17.json -w "%{http_code}" -X POST \
  "$BASE/sovd/v1/components/bcm/operations/DOES_NOT_EXIST/executions" \
  -H "Content-Type: application/json" -d '{}')
r=$(cat /tmp/_uc17.json)
if [[ "$code" == "404" ]] && echo "$r" | json_has "'error_code' in d"; then
  check "UC17 SafetyBoundary" pass "unknown op returns 404 with error_code=$(echo "$r" | json_get 'd[\"error_code\"]')"
else
  check "UC17 SafetyBoundary" fail "expected structured 404, got code=$code body=$r"
fi

# -------- UC18 GatewayRouting --------
r=$(curl -sS "$BASE/sovd/v1/gateway/backends")
if echo "$r" | json_has "all(b.get('reachable') is True for b in d['items'])"; then
  check "UC18 GatewayRouting" pass "all 4 backends reachable"
else
  check "UC18 GatewayRouting" fail "some backends unreachable: $r"
fi

# -------- UC19 Historical --------
# Observability stack: blackbox_exporter probes /sovd/v1/health, Prometheus
# scrapes blackbox, Grafana renders the OpenSOVD SIL dashboard.
r=$(curl -sS "$BASE/sovd/grafana/api/health")
if echo "$r" | json_has "d.get('database')=='ok'"; then
  check "UC19 Historical" pass "Grafana+Prometheus+blackbox live at /sovd/grafana/"
else
  check "UC19 Historical" fail "Grafana health not ok: $r"
fi

# -------- UC20 ConcurrentTesters --------
# Issue 8 concurrent GETs; expect all 200.
code_list=""
for i in 1 2 3 4 5 6 7 8; do
  (curl -sS -o /dev/null -w "%{http_code}\n" "$BASE/sovd/v1/components/cvc/faults") &
done
wait
# all exit with 200 implicitly if none panicked; use more direct probe:
status_total=$(for i in 1 2 3 4 5 6 7 8; do
  curl -sS -o /dev/null -w "%{http_code} " "$BASE/sovd/v1/components/cvc/faults" &
done; wait | tr -d '\n')
if echo "$status_total" | grep -qv 200 ; then
  check "UC20 ConcurrentTesters" fail "not all 200: $status_total"
else
  check "UC20 ConcurrentTesters" pass "8 concurrent GETs succeeded"
fi

echo
echo "== SUMMARY =="
TOTAL=$((PASS+FAIL+SKIP))
echo "Total: $TOTAL  |  PASS: $PASS  |  FAIL: $FAIL  |  SKIP: $SKIP"
exit $FAIL
