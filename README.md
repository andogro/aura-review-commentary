# Aura Consensus Protocol Audit Findings Commentary

This document is prepared in response to the draft audit report provided
by Jean-Philippe Aumasson (08/12/17), SHA256 hash of [aura-audit-report.md](aura-audit-report.md)
is 8040fe95d91b2d77f1f0df2af5ba23c85ee487b18dde650dd46cfc82686d0b3d

## Protocol review

### Risks of synchronized time

The report indicates certain risks of [un]synchronized time and related attacks.
We believe these concerns are valid and are mostly the consequences of the
basics of the protocol (it is based on physical time)

### Resilience to malicious nodes

No commentary.

### Denial-of-service attacks

No commentary.

### Finality conditions

We've identified the same case to cause confusion as well (specification
stating `>= 1/2` of the validators, while code requires `> 1/2`).

With regards to `would_be_finalized` calculation, we currently believe
that this code is correct and is expressing the same `> 1/2` requirement,
but is "forecasting" finalization for the next signer.

Recommended course of action:

- [ ] **P1** Consult with Parity maintainers with regards to the original intent
      and actual implementation. Which constraint ("greater" or "stricly greater")
      should in fact be specified and implemented
- [ ] **P2** Further analyze the correctness of `would_be_finalized` definition
- [ ] **P2.1** Provide meaningful in-code commentary on how `build_ancestry_subchain`
               operates

### Finality delay

No commentary at this moment, but we are further researching this concern.

## Code review

### Unsafe code

The report mentions use of unsafe code in `bigint::hash` where it is used
to efficiently compare two byte arrays. As mentioned, this doesn't seem
to be a security risk. However, it is still an instance of unsafe code
which would always raise at least some concern.

At the core of this issue there's a question of potential performance gains.
In order to address it to best of our ability, we've conducted a few comparative
benchmarks. Firstly, we ran simple benchmarks (slice equality vs memcp) both on
rust stable (rustc 1.22.1 (05e2e1c41 2017-11-22)) and rust nighty (rustc 1.22.1 (05e2e1c41 2017-11-22))
and did not see any gains (the results were very similar in both cases):

```
test cmp      ... bench:           4 ns/iter (+/- 1)
test cmp_loop ... bench:       4,251 ns/iter (+/- 215)
test eq       ... bench:           4 ns/iter (+/- 1)
test eq_loop  ... bench:       4,269 ns/iter (+/- 622)
```

The source code for this benchmark is available at [byte-cmp](byte-cmp)

To continue this effort, we further sampled some libraries available
on [crates.io](https://crates.io).

[fastcmp](https://github.com/saschagrunert/fastcmp) was seemingly able
to show different results:

```
test fast_compare_equal    ... bench:          10 ns/iter (+/- 0) = 25600 MB/s
test fast_compare_unequal  ... bench:          10 ns/iter (+/- 0) = 25600 MB/s
test slice_compare_equal   ... bench:          21 ns/iter (+/- 1) = 12190 MB/s
test slice_compare_unequal ... bench:          21 ns/iter (+/- 0) = 12190 MB/s
```

At the core of its implementation method also lies `memcmp`. The reason for the
significant discrepancy in results is being further investigated. At this moment,
we can only state that the optimization method used in parity *may* produce
much more efficient code so its worth keeping around. However, we'd recommend
the following course of action:

- [ ] **C1** Prepare a patch for bigint::hash that explains the necessity of using unsafe
code (and memcmp) in prose to make this code easier to understand for others.

### Step number cast from 64- to 32-bit

The report indicates a cast of a 64-bit number to 32-bit one in AuRa code. The concern
is valid, however, this behaviour will only be triggered 100-500 years later. That said,
the following course of action was taken:

- [x] **C2** Avoid casting down to 32-bit by dropping the need to use `Duration` type
      for representing step duration (which is typically single or double digits seconds)
      https://github.com/oraclesorg/parity/commit/fa2ddce949d8a227b7135b300003f0a5bceddc0f
- [X] **C2.1** Prepare a PR and get it merged into the main repository (or get resolved in
      any other way) https://github.com/paritytech/parity/pull/7282

### Potential integer overflow

The report highlights that step increment function might overflow the counter, especially
when parity is compiled for a 32-bit system. This event is also quite far in the future,
however, the following course of action was taken:

- [x] **C3** Shutdown parity when such event occurs as there's not much that can be done
      at that point https://github.com/oraclesorg/parity/commit/fa2ddce949d8a227b7135b300003f0a5bceddc0f
- [X] **C3.1** Prepare a PR and get it merged into the main repository (or get resolved in
      any other way) https://github.com/paritytech/parity/pull/7282


### Potential division by zero

The report shows that Step calibration function might panic on division by zero. This is a
valid concern. If somebody will configure Parity with a step duration of 0 seconds, it'll
quickly panic, however, the error won't necessarily be very informative. This case highlights
the lack of refinement types in Rust. Following course of action was taken:

- [x] **C4** Guard step duration parameter to be positive and crash Parity otherwise
     https://github.com/oraclesorg/parity/commit/fa2ddce949d8a227b7135b300003f0a5bceddc0f
- [X] **C4.1** Prepare a PR and get it merged into the main repository (or get resolved in
      any other way) https://github.com/paritytech/parity/pull/7282

### Other possible improvements

The report suggests few other improvements. First one is to improve the measurement of timing.
There is no indication at this point that this is a concern. Course of action:

- [ ] **C5** Try out https://github.com/jedisct1/rust-coarsetime to see how much more
      performant it is
- [ ] **C6** Research the importance of measurement timing

The other suggested improvement is erroneous as it suggests that the code doesn't check
of RNG creation failure, while in fact it does.
