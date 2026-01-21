# Pattern System: Strudel Parity Proposal

This document outlines the changes needed to achieve parity with Strudel's pattern system, enabling us to port Strudel's test suite to validate our implementation.

## Overview

The Rust pattern system in `crates/modular_core/src/pattern_system/` is a port of Strudel's pattern system. To verify correctness, we need to port Strudel's tests from `strudel/packages/core/test/pattern.test.mjs`. However, several features are missing or have API differences that need to be addressed first.

## 1. Missing Alignment Modes

### Current State

The `OperatorVariant` enum in `operators.rs` currently has:
- `Default` — use operator-specific default behavior
- `In` — structure from primary/left pattern (`appLeft`)
- `Out` — structure from argument/right pattern (`appRight`)
- `Squeeze` — squeeze argument into primary events (`squeezeJoin`)
- `Mix` — intersection structure (`appBoth`)

### Missing Modes

Strudel supports these additional alignment modes:
- `Reset` — retrigger inner pattern at outer onsets, aligning cycle position
- `Restart` — retrigger inner pattern at outer onsets, aligning from cycle zero
- `SqueezeOut` — inverse of Squeeze: squeeze primary into argument events

(`Poly` is also in Strudel but deferred as it's less commonly tested)

### Implementation Plan

#### Step 1: Add enum variants

In `crates/modular_core/src/pattern_system/operators.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OperatorVariant {
    #[default]
    Default,
    In,
    Out,
    Squeeze,
    Mix,
    Reset,      // NEW
    Restart,    // NEW
    SqueezeOut, // NEW
}
```

#### Step 2: Implement `reset_join` and `restart_join`

In `crates/modular_core/src/pattern_system/monadic.rs`, add:

```rust
/// Flatten Pattern<Pattern<T>> by retriggering inner patterns at outer onsets.
/// 
/// - `reset_join`: Aligns inner pattern's cycle position with outer hap start
/// - `restart_join`: Aligns inner pattern's cycle zero with outer hap start
/// 
/// Ported from Strudel's `resetJoin(restart: bool)`.
pub fn reset_join<U, F>(&self, f: F, restart: bool) -> Pattern<U>
where
    U: Clone + Send + Sync + 'static,
    F: Fn(&T) -> Pattern<U> + Send + Sync + 'static,
{
    let outer = self.clone();
    let f = Arc::new(f);

    Pattern::new(move |state: &State| {
        // Only consider discrete (onset) events from outer
        let outer_haps: Vec<_> = outer
            .query(state)
            .into_iter()
            .filter(|h| h.is_discrete() && h.has_onset())
            .collect();

        let mut result = Vec::new();

        for outer_hap in outer_haps {
            let inner_pat = f(&outer_hap.value);
            
            // Calculate shift amount:
            // - reset: align cycle position (use whole.begin.cycle_pos())
            // - restart: align from zero (use whole.begin directly)
            let shift = if restart {
                outer_hap.whole_or_part().begin.clone()
            } else {
                outer_hap.whole_or_part().begin.cycle_pos()
            };
            
            // Shift inner pattern later by the calculated amount
            let shifted = inner_pat.late(shift);
            
            // Query shifted pattern
            let inner_haps = shifted.query(state);
            
            for inner_hap in inner_haps {
                // Intersect with outer hap's timespan
                let outer_whole = outer_hap.whole_or_part();
                
                // For inner whole: intersect if discrete, else None
                let new_whole = inner_hap.whole.as_ref().and_then(|w| {
                    w.intersection(outer_whole)
                });
                
                // For inner part: must intersect
                if let Some(new_part) = inner_hap.part.intersection(&outer_hap.part) {
                    let combined_context = outer_hap.combine_context(&inner_hap);
                    result.push(Hap::new_with_context(
                        new_whole,
                        new_part,
                        inner_hap.value.clone(),
                        combined_context,
                    ));
                }
            }
        }

        result
    })
}

pub fn restart_join<U, F>(&self, f: F) -> Pattern<U>
where
    U: Clone + Send + Sync + 'static,
    F: Fn(&T) -> Pattern<U> + Send + Sync + 'static,
{
    self.reset_join(f, true)
}
```

#### Step 3: Implement `squeeze_out_join`

In `crates/modular_core/src/pattern_system/monadic.rs`:

```rust
/// Squeeze the outer pattern into the inner pattern's events.
/// Inverse of squeeze_join: structure comes from the inner (argument) pattern.
///
/// For `a.squeeze_out(b)`:
/// - Structure comes from `b`
/// - For each event in `b`, squeeze `a` into that event's duration
pub fn squeeze_out_join<U, F>(&self, f: F) -> Pattern<U>
where
    U: Clone + Send + Sync + 'static,
    F: Fn(&T) -> Pattern<U> + Send + Sync + 'static,
{
    // squeeze_out(a, b) = b.fmap(|b_val| a.fmap(|a_val| f(a_val, b_val))).squeeze_join()
    // But here we need to flip the roles
    let outer = self.clone();
    let f = Arc::new(f);

    Pattern::new(move |state: &State| {
        // Query outer for structure
        let outer_haps: Vec<_> = outer
            .query(state)
            .into_iter()
            .filter(|h| h.has_onset())
            .collect();

        let mut result = Vec::new();

        for outer_hap in outer_haps {
            let inner_pat = f(&outer_hap.value);
            
            // Focus the inner pattern to fit within the outer event's duration
            let squeeze_span = outer_hap.whole_or_part();
            let focused = inner_pat.focus_span(squeeze_span);
            
            let inner_haps = focused.query(state);
            
            for inner_hap in inner_haps {
                if let Some(new_part) = inner_hap.part.intersection(&outer_hap.part) {
                    let new_whole = inner_hap.whole.as_ref().and_then(|w| {
                        w.intersection(outer_hap.whole_or_part())
                    });
                    
                    let combined_context = outer_hap.combine_context(&inner_hap);
                    result.push(Hap::new_with_context(
                        new_whole,
                        new_part,
                        inner_hap.value.clone(),
                        combined_context,
                    ));
                }
            }
        }

        result
    })
}
```

**Note:** The actual implementation of `squeeze_out` flips which pattern provides structure:
- `a.squeeze(b)`: `a` provides structure, `b` squeezed into `a`'s events
- `a.squeeze_out(b)`: `b` provides structure, `a` squeezed into `b`'s events

This can be implemented as:
```rust
// _opSqueezeOut in Strudel:
// return otherPat.fmap((a) => thisPat.fmap((b) => func(b)(a))).squeezeJoin();
```

#### Step 4: Add operator helper methods

Add convenience methods for operators in `applicative.rs` or a new file:

```rust
impl<T: Clone + Send + Sync + 'static> Pattern<T> {
    /// Apply binary operation with Reset alignment
    pub fn op_reset<U, V, F>(&self, other: &Pattern<U>, f: F) -> Pattern<V>
    where
        U: Clone + Send + Sync + 'static,
        V: Clone + Send + Sync + 'static,
        F: Fn(&T, &U) -> V + Send + Sync + Clone + 'static,
    {
        let this = self.clone();
        let f = f.clone();
        other.reset_join(move |b| {
            this.fmap(move |a| f(a, b))
        }, false)
    }

    /// Apply binary operation with Restart alignment
    pub fn op_restart<U, V, F>(&self, other: &Pattern<U>, f: F) -> Pattern<V>
    where
        U: Clone + Send + Sync + 'static,
        V: Clone + Send + Sync + 'static,
        F: Fn(&T, &U) -> V + Send + Sync + Clone + 'static,
    {
        let this = self.clone();
        let f = f.clone();
        other.reset_join(move |b| {
            this.fmap(move |a| f(a, b))
        }, true)
    }

    /// Apply binary operation with SqueezeOut alignment
    pub fn op_squeeze_out<U, V, F>(&self, other: &Pattern<U>, f: F) -> Pattern<V>
    where
        U: Clone + Send + Sync + 'static,
        V: Clone + Send + Sync + 'static,
        F: Fn(&T, &U) -> V + Send + Sync + Clone + 'static,
    {
        let this = self.clone();
        let f = f.clone();
        other.squeeze_join(move |b| {
            this.fmap(move |a| f(a, b))
        })
    }
}
```

#### Step 5: Wire into operator dispatch

Update all operators in `operators.rs` to handle the new variants:

```rust
// In AddOperator, MulOperator, etc:
Ok(match variant {
    OperatorVariant::Default | OperatorVariant::In => {
        pattern.app_left(&arg_pattern, |a, b| a + b)
    }
    OperatorVariant::Out => pattern.app_right(&arg_pattern, |a, b| a + b),
    OperatorVariant::Mix => pattern.app_both(&arg_pattern, |a, b| a + b),
    OperatorVariant::Squeeze => pattern.op_squeeze(&arg_pattern, |a, b| a + b),
    OperatorVariant::SqueezeOut => pattern.op_squeeze_out(&arg_pattern, |a, b| a + b),
    OperatorVariant::Reset => pattern.op_reset(&arg_pattern, |a, b| a + b),
    OperatorVariant::Restart => pattern.op_restart(&arg_pattern, |a, b| a + b),
})
```

---

## 2. Generic Patterned Parameters

### Current State

Temporal methods like `fast`, `slow`, `early`, `late` currently take constant `Fraction` values:

```rust
pub fn fast(&self, factor: Fraction) -> Pattern<T>
pub fn slow(&self, factor: Fraction) -> Pattern<T>
pub fn early(&self, offset: Fraction) -> Pattern<T>
pub fn late(&self, offset: Fraction) -> Pattern<T>
```

### Desired State

Strudel allows pattern arguments via its `patternify` mechanism:

```javascript
// These are equivalent in Strudel:
pattern.fast(2)
pattern.fast(sequence(1, 4))  // Patterned factor!
```

### Implementation Plan

#### Step 1: Create `IntoPattern` trait

In `crates/modular_core/src/pattern_system/mod.rs`:

```rust
/// Trait for types that can be converted into a Pattern.
/// Enables generic methods that accept both values and patterns.
pub trait IntoPattern<T>: Send + Sync {
    fn into_pattern(self) -> Pattern<T>;
}

// Direct value -> pure pattern
impl<T: Clone + Send + Sync + 'static> IntoPattern<T> for T {
    fn into_pattern(self) -> Pattern<T> {
        pure(self)
    }
}

// Pattern -> identity
impl<T: Clone + Send + Sync + 'static> IntoPattern<T> for Pattern<T> {
    fn into_pattern(self) -> Pattern<T> {
        self
    }
}

// Reference to pattern -> clone
impl<T: Clone + Send + Sync + 'static> IntoPattern<T> for &Pattern<T> {
    fn into_pattern(self) -> Pattern<T> {
        self.clone()
    }
}
```

#### Step 2: Update temporal methods with generics

In `crates/modular_core/src/pattern_system/temporal.rs`:

```rust
/// Speed up the pattern by a factor.
/// Accepts both constant values and patterns.
///
/// # Examples
/// ```
/// pattern.fast(Fraction::from(2))           // Constant: 2x speed
/// pattern.fast(sequence(vec![1, 4]))        // Pattern: alternates 1x and 4x
/// ```
pub fn fast<F: IntoPattern<Fraction>>(&self, factor: F) -> Pattern<T> {
    let factor_pat = factor.into_pattern();
    let pat = self.clone();
    
    factor_pat.inner_join(move |f| {
        pat._fast(f.clone())
    })
}

/// Internal constant-factor fast (no pattern overhead).
fn _fast(&self, factor: Fraction) -> Pattern<T> {
    if factor.is_zero() {
        return silence();
    }

    let query = self.query.clone();
    let factor_clone = factor.clone();

    Pattern::new(move |state: &State| {
        let new_span = state.span.with_time(|t| t * &factor_clone);
        let haps = query(&state.set_span(new_span));

        haps.into_iter()
            .map(|hap| hap.with_span_transform(|span| span.with_time(|t| t / &factor_clone)))
            .collect()
    })
}

/// Slow down the pattern by a factor.
pub fn slow<F: IntoPattern<Fraction>>(&self, factor: F) -> Pattern<T> {
    let factor_pat = factor.into_pattern();
    let pat = self.clone();
    
    factor_pat.inner_join(move |f| {
        pat._slow(f.clone())
    })
}

fn _slow(&self, factor: Fraction) -> Pattern<T> {
    if factor.is_zero() {
        return silence();
    }
    self._fast(Fraction::from(1) / factor)
}

/// Shift pattern earlier in time.
pub fn early<F: IntoPattern<Fraction>>(&self, offset: F) -> Pattern<T> {
    let offset_pat = offset.into_pattern();
    let pat = self.clone();
    
    offset_pat.inner_join(move |o| {
        pat._early(o.clone())
    })
}

fn _early(&self, offset: Fraction) -> Pattern<T> {
    self.with_query_time(move |t| t + &offset)
        .with_hap_time(move |t| t - &offset)
}

/// Shift pattern later in time.
pub fn late<F: IntoPattern<Fraction>>(&self, offset: F) -> Pattern<T> {
    let offset_pat = offset.into_pattern();
    let pat = self.clone();
    
    offset_pat.inner_join(move |o| {
        pat._late(o.clone())
    })
}

fn _late(&self, offset: Fraction) -> Pattern<T> {
    self._early(-offset)
}
```

#### Step 3: Add `segment` method

```rust
/// Discretize a continuous signal by sampling it n times per cycle.
/// Essential for converting signals like `saw()` to discrete events.
pub fn segment<N: IntoPattern<i64>>(&self, n: N) -> Pattern<T> {
    let n_pat = n.into_pattern();
    let pat = self.clone();
    
    n_pat.inner_join(move |n| {
        pat._segment(*n)
    })
}

fn _segment(&self, n: i64) -> Pattern<T> {
    if n <= 0 {
        return silence();
    }
    
    // Create n evenly-spaced sample points per cycle
    let pat = self.clone();
    let frac_n = Fraction::from(n);
    
    Pattern::new(move |state: &State| {
        let mut result = Vec::new();
        
        for span in state.span.span_cycles() {
            let cycle_start = span.begin.sam();
            
            for i in 0..n {
                let frac_i = Fraction::from(i);
                let event_start = &cycle_start + &frac_i / &frac_n;
                let event_end = &cycle_start + (&frac_i + Fraction::from(1)) / &frac_n;
                let event_span = TimeSpan::new(event_start.clone(), event_end.clone());
                
                // Check if this event intersects the query span
                if let Some(part) = event_span.intersection(&span) {
                    // Sample the pattern at the event start
                    let sample_state = state.set_span(TimeSpan::new(
                        event_start.clone(),
                        event_start.clone(),
                    ));
                    
                    if let Some(hap) = pat.query(&sample_state).into_iter().next() {
                        result.push(Hap::new(
                            Some(event_span),
                            part,
                            hap.value.clone(),
                        ));
                    }
                }
            }
        }
        
        result
    })
}
```

---

## 3. Tests to Port from Strudel

Once the above features are implemented, the following test categories from `pattern.test.mjs` can be ported:

### Core Tests (Already Portable)

| Strudel Test | Rust Equivalent | Status |
|--------------|-----------------|--------|
| `TimeSpan.equals()` | `timespan.rs` tests | ✅ Exists |
| `TimeSpan.splitCycles` | `span_cycles()` | ✅ Exists |
| `TimeSpan.intersection_e` | `intersection()` | ✅ Exists |
| `Hap.hasOnset()` | `has_onset()` | ✅ Exists |
| `Hap.spanEquals` | Need to add | 🔲 TODO |
| `Hap.wholeOrPart()` | `whole_or_part()` | ✅ Exists |
| `Pattern.pure` | `pure()` | ✅ Exists |
| `Pattern.fmap()` | `fmap()` | ✅ Exists |
| `Pattern.stack()` | `stack()` | ✅ Exists |
| `Pattern.fastcat()` | `fastcat()` | ✅ Exists |
| `Pattern.slowcat()` | `slowcat()` | ✅ Exists |
| `Pattern.sequence()` | `sequence()` | ✅ Exists |
| `Pattern.rev()` | `rev()` | ✅ Exists |

### Tests Requiring New Alignment Modes

| Strudel Test | Required Feature |
|--------------|------------------|
| `add.reset()` | `Reset` alignment mode |
| `add.restart()` | `Restart` alignment mode |
| `add.squeezeout()` | `SqueezeOut` alignment mode |
| `keep.reset()` | `Reset` alignment mode |
| `keep.restart()` | `Restart` alignment mode |
| `keep.squeezeout()` | `SqueezeOut` alignment mode |
| `keepif.reset()` | `Reset` alignment mode |
| `keepif.restart()` | `Restart` alignment mode |
| `keepif.squeezeout()` | `SqueezeOut` alignment mode |

### Tests Requiring Patterned Parameters

| Strudel Test | Required Feature |
|--------------|------------------|
| `fast(sequence(1, 4))` | Generic `fast<F: IntoPattern>` |
| `fast(1.5, 2)` variadic | Generic `fast` + sequence helper |
| `slow()` with patterns | Generic `slow<F: IntoPattern>` |
| `early()` with patterns | Generic `early<F: IntoPattern>` |

### Tests to Skip (Out of Scope)

| Strudel Test | Reason |
|--------------|--------|
| `set()`, `setOut()`, `setSqueeze()` | Object value merging - not applicable to typed Rust |
| `firstOf()`, `every()` | Function patterns - defer |
| `when()` | Conditional with function - defer |
| `jux()`, `juxBy()` | Stereo/spatial - DSL feature |
| `chop()`, `striate()`, `slice()`, `splice()` | Sample slicing - audio-specific |
| `inhabit()`, `pick()` | Named pattern lookup - DSL feature |

---

## 4. Implementation Order

1. **Phase 1: Alignment Modes**
   - Add `OperatorVariant` enum variants
   - Implement `reset_join`, `restart_join`
   - Implement `squeeze_out_join`
   - Wire into operator dispatch
   - Add tests

2. **Phase 2: Generic Parameters**
   - Create `IntoPattern` trait
   - Update `fast`, `slow`, `early`, `late`
   - Add `segment` method
   - Add tests

3. **Phase 3: Port Strudel Tests**
   - Create test helper utilities (`ts`, `st`, `hap`, `same_first`)
   - Port `TimeSpan` tests
   - Port `Hap` tests
   - Port `Pattern` core tests
   - Port alignment mode tests
   - Port patterned parameter tests

---

## 5. Open Questions

1. **Optimization for pure values**: The generic approach means `fast(2)` goes through `pure(2).inner_join()`. Should we add runtime detection for pure patterns to skip the join overhead?

2. **API naming**: Should we keep internal `_fast()` methods public for cases where users explicitly want the non-patterned version for performance?

3. **`late()` dependency**: The alignment modes require `late()`. Verify it exists or implement as `early(-offset)`.

4. **Test framework**: Should tests use the existing Rust test framework or create a more Strudel-like test DSL for easier porting?
