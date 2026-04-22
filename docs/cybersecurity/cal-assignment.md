# CAL Assignment Matrix

Status: approved 2026-04-22
Owner: Taktflow security lead

This matrix applies the ADR-0036 rules to every currently exposed endpoint
family.

## Matrix

| Surface or endpoint family | Example path | Primary concern | CAL | Rationale |
|---|---|---|---|---|
| Health and local diagnostics smoke | `/sovd/v1/health` | availability only | CAL 2 | Low mutation risk but still bench-reachable |
| Component catalog and discovery | `/sovd/v1/components` | information exposure | CAL 2 | Read-only discovery |
| Data read | `/sovd/v1/components/{id}/data/{did}` | confidentiality, backend abuse | CAL 3 | Sensitive runtime state; no mutation |
| Fault list and fault detail read | `/sovd/v1/components/{id}/faults` | confidentiality | CAL 3 | Reveals safety and operational state |
| Fault clear | `/sovd/v1/components/{id}/faults/{code}` DELETE | integrity of diagnostic state | CAL 4 | Mutates diagnostic evidence |
| Operation start | `/sovd/v1/components/{id}/operations/*/executions` POST | integrity and backend side effects | CAL 4 | Can change ECU behavior |
| Bulk-data and OTA | `/sovd/v1/components/{id}/bulk-data/*` | firmware integrity | CAL 4 | Update path with rollback implications |
| Observer session/audit/backend extras | `/sovd/v1/session`, `/sovd/v1/audit` | sensitive observability | CAL 3 | Reveals live auth and backend state |
| COVESA VSS reads | `/sovd/covesa/vss/*` GET | confidentiality | CAL 3 | Semantic alias for sensitive data |
| COVESA actuator writes | `/sovd/covesa/vss/*` POST | integrity and actuation | CAL 4 | Whitelisted writes still mutate state |
| Extended Vehicle read APIs | `/sovd/v1/extended/vehicle/*` GET | confidentiality | CAL 3 | Vehicle state and fault-log exposure |
| Extended Vehicle subscription create/delete | `/sovd/v1/extended/vehicle/subscriptions` | resource abuse, data exposure | CAL 3 | State fan-out and availability concern |
| Bench-only internal fault seeding | `/__bench/components/{id}/faults` | integrity | CAL 4 | Internal-only but directly mutates visible faults |

## Notes

1. If a route family can both read and mutate, the higher CAL wins.
2. Trusted-ingress auth paths inherit the CAL of the downstream route family.
3. Local-only development routes may be treated as CAL 1 only when they are
   loopback-only and absent from bench or production deploys.
