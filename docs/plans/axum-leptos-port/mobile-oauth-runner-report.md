# Mobile OAuth runner report

## Runner contract and isolation

- `task api:acceptance` runs the medication read HTTP contract against a disposable PostgreSQL 18 project. The acceptance includes medication forecast, mobile OAuth, and read API cases.
- The runner now puts the Rust API and HTTP test process inside each per-project Compose network. Both use internal port `39998`; `runner.compose.yaml` has no host `ports` mapping. Each run can use a separately validated `CONTRACT_TEST_SUBNET`.
- Independent checks remain `task api:test`, `task api:fmt`, `task api:clippy`, `task contract:fmt`, and `task contract:clippy`.

## Concurrent acceptance proof

Two overlapping `task api:acceptance` runs used distinct, prechecked Docker subnets. Both created healthy database, Rails, and Rust API services, and each completed its HTTP cases successfully.

| Project | Subnet | Disposable fixture path | Rust API container ID |
|---|---|---|---|
| `mtcontract-b8777c0e69024c41` | `192.168.240.0/28` | `run.AGbMpE/fixture.json` | `35b3099ce66a5ebeb9ee11bae7efc646cc52e59769bcaba9f7f7314015849df1` |
| `mtcontract-aab7f2fd9f37467a` | `192.168.240.16/28` | `run.8KFp3e/fixture.json` | `7e9dbb5a2b7155643225ff666d650e45aafcff251185a686db80d3b0eebf2b6d` |

Each run passed forecast `3/3`, mobile OAuth `7/7`, and medication read `9/9` (19/19 total); both Task processes exited 0. The OAuth cases covered list/show grants, invalid grants, audit identity without bearer output, current household visibility, delegated medication grants, foreign household denial with activity refresh, and invalid-filter activity refresh. Fixture files were project-specific. No bearer values were included in this report.

The Docker event records identify both API containers and show their shutdown events at 13:27:49, with the second container destroyed by 13:27:52. The Task logs show both sidecars reaching Healthy while both runs were active. I did not capture a single `docker ps` snapshot with both container IDs at the same instant, so the precise overlap interval cannot be independently established. The runner Compose configuration shows no host port bindings; this is configuration evidence, not a claim derived from the container event records. Network names and configured subnets were observed; network IDs were not captured before cleanup.

Both runs removed their API/Rails/database containers, project networks, project images, volumes, and storage directories. The first pair attempt before the subnet fix failed when Docker's default address pool was exhausted; no API was started for that failed project. An earlier single-project attempt used stale runner inputs and stopped on the three forecast cases because the approved-origin setting was absent; the final pair used the corrected runner and all forecast cases passed.

## Stable-input evidence

- Baseline HEAD and HEAD before and after the final pair: `7bec286ae524c00286d6486d825cb11d7eae936e`.
- The pre-run manifest hashed 43 explicitly listed files (API/contract Rust sources, manifests, the three medication HTTP test files, runner scripts and Compose files, Taskfiles, provisioning code, and Rails fixtures). Its aggregate SHA-256 was `ade1f35080a5ba4b30162844db0f43c1ca5dd4ca6858e91c2d9c2ffdc359ed35`. The same 43-file set after the pair had the identical digest.
- Re-enumeration after the pair found two relevant runner paths omitted from that original explicit list: `rust/contract-tests/runner-subnet.compose.yaml` and `rust/contract-tests/validate_subnet.fish`. The expanded 44-file post-run set, including the Compose override but not the validator, hashed to `d2462080f0283d237eb807481c1740de6eaac4439821f71cd7303b7a568e3bf0`. Because those paths were not included in the pre-run hash, the manifest does not prove a complete byte-for-byte before/after match for them. Writers had frozen runner inputs before the pair; no source edits were reported during the runs.
- The earlier `git diff --binary` digest was not used as runtime evidence because it omits untracked files.

## Runner follow-up

Keep per-project network isolation and explicit subnet validation. The default Docker address pool was exhausted in this environment, so concurrent runs require distinct checked subnet slices until the configured pool provides enough capacity. Do not prune unrelated networks or change daemon settings as part of this acceptance lane.
