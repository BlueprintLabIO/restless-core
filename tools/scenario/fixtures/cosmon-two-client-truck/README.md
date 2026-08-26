# Cosmon two-client truck fixture

This is an isolated, test-world technical walking skeleton. It starts one Godot ENet server and two
separate Godot clients through a local deterministic UDP delay/loss proxy. The server, rather than a
client narrative, records the crate-through-truck sequence.

Run it only in a Company Runtime with the Sprint 19 image:

```sh
restless-scenario doctor .
restless-scenario run . --output /company/outputs/cosmon-two-client-truck-001 --seed s19
```

Open `review.html` in the output directory after a mechanically verified run. It is a prepared
technical state review, not a game-feel, art, Steam, player-demand, or commercial-readiness verdict.
