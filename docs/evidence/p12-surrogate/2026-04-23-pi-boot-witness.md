# P12-SUR-02 Pi Surrogate Boot Witness

Date: 2026-04-23

Purpose: prove that the repo-owned Pi deploy path can refresh `sovd-main`
on the existing Pi-class surrogate target and that the local SOVD surface
answers `GET /sovd/v1/components` afterward.

## Command Shape

Sanitized command shape used for the deploy and proof:

```bash
cd opensovd-core
PI=<pi-user>@<pi-bench-ip> ./deploy/pi/phase5-full-stack.sh
ssh <pi-user>@<pi-bench-ip> "systemctl is-active sovd-main"
ssh <pi-user>@<pi-bench-ip> "curl -fsS http://127.0.0.1:21002/sovd/v1/components"
ssh <pi-user>@<pi-bench-ip> "journalctl -u sovd-main -n 20 --no-pager"
```

## Result

- The repo deploy flow refreshed the Pi-side `sovd-main` service.
- `systemctl is-active sovd-main` returned `active` after deploy.
- The Pi-local loopback endpoint answered `GET /sovd/v1/components`.
- The active component set after deploy was `bcm`, `cvc`, `dfm`, `sc`.

## Response Witness

```json
{
  "items": [
    {
      "id": "bcm",
      "name": "Body Control Module",
      "href": "/sovd/v1/components/bcm"
    },
    {
      "id": "cvc",
      "name": "Central Vehicle Controller",
      "href": "/sovd/v1/components/cvc"
    },
    {
      "id": "dfm",
      "name": "dfm",
      "href": "/sovd/v1/components/dfm"
    },
    {
      "id": "sc",
      "name": "Safety Controller",
      "href": "/sovd/v1/components/sc"
    }
  ]
}
```

## Journal Witness

Sanitized journal excerpt from the Pi after the deploy-triggered restart:

```text
... Stopping sovd-main.service ...
... Started sovd-main.service ...
Loading configuration from /opt/taktflow/sovd-main/opensovd.toml
... Booting InMemoryServer with configured local demo surface and forwards ...
... OpenSOVD core listening on 127.0.0.1:21002 transport="http"
```

## Scope Note

This witness closes `P12-SUR-02` only. It does not satisfy real `P12`,
does not close `Q-PROD-1` or `Q-PROD-2`, and does not count as M11 or
`G-PROD-1` evidence.
