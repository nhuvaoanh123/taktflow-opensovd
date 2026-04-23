# P12-SUR-03 Pi Deploy-Path Cleanup Witness

Date: 2026-04-23

Purpose: prove that the Pi surrogate deploy path no longer depends on
hidden workstation-only defaults and can boot from repo-owned deploy
assets plus local placeholder-substituted values.

## Command Shape

Sanitized command shape used for the cleanup proof:

```bash
cd opensovd-core
cp deploy/pi/phase5-full-stack.env.example deploy/pi/phase5-full-stack.env
# fill PI=<pi-user>@<pi-bench-ip> and any optional overrides locally
./deploy/pi/phase5-full-stack.sh
ssh <pi-user>@<pi-bench-ip> "grep -nE '^(User|Group)=' /etc/systemd/system/sovd-main.service"
ssh <pi-user>@<pi-bench-ip> "curl -fsS http://127.0.0.1:21002/sovd/v1/components"
```

## Result

- The checked-in deploy flow ran without a baked-in private Pi target.
- The deploy path accepted the target through `PI=<pi-user>@<pi-bench-ip>`
  or the untracked local env file next to the script.
- The rendered systemd unit installed explicit `User=` / `Group=` values
  instead of a hardcoded bench account name.
- The same cleaned-up path still produced a green
  `GET /sovd/v1/components` on the surrogate target.

## Deploy-Path Evidence

Sanitized witness from the completed deploy:

```text
[phase5-full-stack] using PI service account: <pi-service-user>:<pi-service-group>
[phase5-full-stack] [1/6] preparing /opt/taktflow ...
[phase5-full-stack] [2/6] rsync sovd-main binary ...
[phase5-full-stack] [3/6] rsync sovd-main config ...
[phase5-full-stack] [4/6] installing sovd-main.service ...
[phase5-full-stack] [6/6] verification
[phase5-full-stack] sovd-main answering GET /sovd/v1/components on 127.0.0.1:21002 via SSH - D1 green
[phase5-full-stack] done
```

Rendered unit-file witness after deploy:

```text
User=<pi-service-user>
Group=<pi-service-group>
```

## Response Witness

Sanitized component-set witness after the cleanup deploy:

```json
{
  "items": [
    { "id": "bcm", "href": "/sovd/v1/components/bcm" },
    { "id": "cvc", "href": "/sovd/v1/components/cvc" },
    { "id": "dfm", "href": "/sovd/v1/components/dfm" },
    { "id": "sc", "href": "/sovd/v1/components/sc" }
  ]
}
```

## Scope Note

This witness closes `P12-SUR-03` only. It does not satisfy real `P12`,
does not close `Q-PROD-1` or `Q-PROD-2`, and does not count as M11 or
`G-PROD-1` evidence.
