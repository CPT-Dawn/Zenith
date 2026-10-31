# Stay on GTK4 despite the "lightest bar" promise

Zenith's selling point is low resource use, which on its face argues for a custom renderer on smithay-client-toolkit (~5–15 MB). We kept GTK4 and optimise it instead. A rewrite would have meant building text input, popovers and a calendar ourselves, which is months of work and a risk to visual quality. So we pitch Zenith as the lightest *beautiful* bar and publish measured numbers rather than claim to be the lightest bar overall.

## Consequences

- A GTK4 process has a memory floor of tens of megabytes that no amount of tuning removes. Performance budgets are set by measurement, not aspiration.
- We revisit this decision only if the benchmark harness shows Zenith losing to comparable rich bars.
