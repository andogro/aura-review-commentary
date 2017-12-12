% Audit of the Aura consensus protocol
% Jean-Philippe Aumasson
% 08/12/17

Authority Round (a.k.a. Aura) is a proof-of-authority consensus engine in the [Parity](https://github.com/paritytech/parity/) Ethereum client, implemented in [/ethcore/src/engines/authority_round](https://github.com/paritytech/parity/tree/master/ethcore/src/engines/authority_round) in files [mod.rs](https://github.com/paritytech/parity/blob/master/ethcore/src/engines/authority_round/mod.rs) and [finality.rs](https://github.com/paritytech/parity/blob/master/ethcore/src/engines/authority_round/finality.rs) (about 850 lines or Rust, including approximately 250 lines of tests).

The goal of this audit is to assess the security strength of the Aura and its implementation, and to find any security shortcoming, in particular related to finality conditions.
The version of the source code reviewed is that at the commit `b1517654e1212588238c989d00dd92128ea040fe`.

The first part *Protocol review* is about the protocol logic, as understood from the documentation and as implemented. 
The second part *Code review* is about logic and software bugs in the implementation that are not directly related to the protocol's logic.


# Protocol review

## Risks of synchronized time

Aura critically relies on time synchronization of the validators for successful execution of the protocol.
This increases two risks:

1. **Inconsistency** of the blockchain's state, if clocks of validators are accidentally desynchronized, for example due to clock drift. In this case, different validators would compute different a different step index, and therefore a different lead validator.

2. **Denial-of-service** and possiblity other attacks: Many machines would get time from an NTP server through ntpd, adjusting their local clock to compensate any skew. However,
    - The NTP server(s) have then to be trusted---which also reduces the decentralization of the protocol
    - The NTP traffic is unauthenticated by default, thus a network attacker could transmit invalid time to the validators.

The risk of inconsistency can be minimized by choosing a long enough duration, and by synchronizing with a trusted time server from a local network (for example getting its time from GPS).

The risk of denial-of-service can be minimized by authenticating any NTP requests, using the pre-shared key authentication mode or the protocol proposed in a [recent Internet Draft](https://tools.ietf.org/html/draft-ietf-ntp-using-nts-for-ntp-10).

Even when NTP is not used, an attacker may have other means of influencing the local clock skew, for example through [CPU heating](http://sec.cs.ucl.ac.uk/users/smurdoch/papers/ccs06hotornot.pdf).


## Resilience to malicious nodes

Aura claims to tolerate up to 50% malicious nodes, as per its documentation.
The goals of a collusion of malicious nodes would be to seal blocks that should not be sealed, or merely to influence the selection of a validator.

Based on my understanding of the protocol, I believe these claims are correct. The bound 50% is tight, and any collusion of more than 50% of the validators could disrupt the sound execution of the protocol.

## Denial-of-service attacks

Even if connections between validators are encrypted and authenticated, an attacker that blocks from/to one or more validators can prevent the protocol from validating incoming blocks.

Solutions to this problem are non-trivial, and seem to require a trade-off between resiliency to network disruptions and security against collusions of malicious nodes.

## Finality conditions

At the time of writing the [documentation](https://github.com/paritytech/parity/wiki/Aura) defines finality condition as `|SIG_SET(C[K..])| >= n/2`, where `n` is the number of validators, which suggests finalization as soon as 50% of validators have signed a given step.
However a strict majority seems to be the desired condition, so this condition should instead be `|SIG_SET(C[K..])| > n/2`.

The condition implemented in finality.rs seems even stricter in `build_ancestry_subchain()`:

```
    let would_be_finalized = (current_signed + 1) * 2 > self.signers.len();
```

However this condition seems inconsistent with that in `push_hash()`, which simply requires a strict majority:

```
		while self.sign_count.len() * 2 > self.signers.len() {
```

If this understanding is correct, the finality condition should be consistently implemented and specified.


## Finality delay

A minimum of `n_v/2 + 1` validations being required, with `n_v` the number of validators.
At least `2(n_v/2 + 1) = n_v + 2` message round trips are therefore necessary before a block is finalized by all validators.
In the worst case, after exactly `n_v` validations, the delay will instead be of `2n_v + 2`.
If multiple blocks are proposed for validation, the voting will add at least `2n_v` round trips.

The average delay may be estimated empirically, based on the number of validations of a block or series thereof.


# Code review

This section reports on potential security risks in the implementation of Aura.
We mainly reviewed the mod.rs and finality.rs files, and did not comprehensively review their dependencies.

## Unsafe code

The files implementing Aura, mod.rs and finality.rs, do not unsafe code blocks such as raw pointer dereferences, nor recursions that would create a risk of stack overflow.

However, there is an unsafe blocks in the `bigint::hash` module that is executed in the Aura code, namely the implementation of the PartialEq trait:

```
		impl PartialEq for $from {
			fn eq(&self, other: &Self) -> bool {
				unsafe { memcmp(self.0.as_ptr() as *const c_void, other.0.as_ptr() as *const c_void, $size) == 0 }
			}
		}
```
This is for example executed in Aura's `is_epoch_end()` function in the line 

```
	.map(|h| if h == chain_head.hash() {
```
This does not create a security, however, since the buffer size (32 bytes) is hardcoded and can't be controlled by an attacher.


## Step number cast from 64- to 32-bit 

In `duration_remaining()` a Step's `inner` attribute obtained from `self.load()` is cast from `usize` to `u32`, thus truncating any 64-bit value when running on a 64-bit system (where `usize` is 64-bit). This number is then multiplied to a `Duration` object to estimate the remaining time for this step:

```
	fn load(&self) -> usize { self.inner.load(AtomicOrdering::SeqCst) }
	fn duration_remaining(&self) -> Duration {
		let now = unix_now();
		let step_end = self.duration * (self.load() as u32 + 1);
```

Rust does not allow a `Duration` to be multiplied by an `u64`, however. The safest fix seems therefore to check that `inner` is smaller than `u32::MAX`.

## Potential integer overflow

The `AtomicUsize::fetch_add()` function will wrap around on overflow, which may occur on 32-bit systems (where `usize` is 32-bit) and lead to unsafe behavior of the protocol if not detected: c
```
	fn increment(&self) {
		self.inner.fetch_add(1, AtomicOrdering::SeqCst);
	}
```

A fix is to check that `inner` is smaller than `u32::MAX`.


## Potential division by zero

If `self.duration` is less than a second, then a division by zero will happen in:

```
		if self.calibrate {
			let new_step = unix_now().as_secs() / self.duration.as_secs();
			self.inner.store(new_step as usize, AtomicOrdering::SeqCst);
		}
```

A fix is to ensure that `self.duration` is greater than one second.

## Other possible improvements

### Faster timings

If speed of timing measurements is critical, Aura may consider using [rust-coarsetime](https://github.com/jedisct1/rust-coarsetime), "a partial replacement for the Time and Duration structures from the standard library".
The timings obtained are slightly less accurate, though.

### Safer PRNG

Aura doesn't rely on a pseudorandom generator (`random()` is only used for testing), yet we observed that in hash.rs the `randomize()` function will not check that an instance of OsRng was successfully created:

```
pub fn randomize(&mut self) {
	let mut rng = OsRng::new().unwrap();
	*self= $from::rand(&mut rng);
}
```

It would be safer to properly check the OsRng instantiation, by doing the following (as [copied from SO](https://stackoverflow.com/questions/27362523/generating-secure-random-numbers-in-rust))

```
    let mut rng = match OsRng::new() {
        Ok(g) => g,
        Err(e) => panic!("Failed to obtain OS RNG: {}", e)
    };
```






