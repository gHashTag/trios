# /gardener — IGLA RACE Fleet Gardener

Manage the IGLA RACE training fleet like a garden. All commands run via Railway CLI — zero local CPU load.

## Commands

### status
Show garden overview: Railway services status, best BPB, disk usage, counters.

### prune
Kill stuck/crashed Railway services (status CRASHED or STOPPED >30min).

### water
Restart services that completed early or have stale BPB (>2h no new samples).

### harvest
Collect best BPB values from all service logs and local v2 sweep logs. Print leaderboard.

### fertilize
Boost step counts on top-3 configs and create mirror services with same settings + new seed.

### weed
Remove duplicate or underperforming services (same format/seed/optimizer as a better one).

### trellis
Guide the search: identify promising format zones from BPB data and spawn exploratory services.

### compost
Delete old logs, free disk on Railway volumes. Report savings.

### full
Run prune → water → harvest → fertilize → trellis in sequence.

## Anchor
phi^2 + phi^-2 = 3
