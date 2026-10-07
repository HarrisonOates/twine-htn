//! Inferred preconditions of compound tasks, for pruning.
//!
//! A fact some later task of a sequence needs, which nothing earlier in the sequence can add,
//! must already hold when the sequence starts. Over a whole decomposition this gives each
//! compound task the facts every one of its refinements needs at the start, and each method, and
//! each tail of a method's subtasks, the same. The search tests them before it looks inside: a
//! task whose inferred precondition fails in the current state has no outcomes there.
//!
//! `mayadd(t)` is everything a refinement of `t` could add: for an action its add list, for a
//! compound task the union over its methods' subtasks, a least fixpoint. `pre(t₁ … tₙ)` is
//! `⋃ᵢ pre(tᵢ) ∖ ⋃_{j<i} mayadd(tⱼ)`, and `pre(t)` of a compound task is the intersection over its
//! methods, a greatest fixpoint from "every fact". It is sound by induction on the height of a
//! refinement: what a refinement really adds is within `mayadd`, so it needs at least `pre`.

use crate::sas::Model;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Bits(Vec<u64>);

impl Bits {
    fn empty(n: usize) -> Bits {
        Bits(vec![0; n.div_ceil(64)])
    }

    fn full(n: usize) -> Bits {
        let mut b = Bits(vec![u64::MAX; n.div_ceil(64)]);
        if !n.is_multiple_of(64) {
            *b.0.last_mut().unwrap() = (1u64 << (n % 64)) - 1;
        }
        b
    }

    fn of(facts: &[u32], n: usize) -> Bits {
        let mut b = Bits::empty(n);
        for &f in facts {
            b.0[f as usize / 64] |= 1 << (f % 64);
        }
        b
    }

    fn union(&mut self, o: &Bits) {
        self.0.iter_mut().zip(&o.0).for_each(|(a, b)| *a |= b);
    }

    fn intersect(&mut self, o: &Bits) {
        self.0.iter_mut().zip(&o.0).for_each(|(a, b)| *a &= b);
    }

    /// `self ∖ o`
    fn minus(&self, o: &Bits) -> Bits {
        Bits(self.0.iter().zip(&o.0).map(|(a, b)| a & !b).collect())
    }

    fn facts(&self) -> Vec<u32> {
        let mut out = Vec::new();
        for (w, &word) in self.0.iter().enumerate() {
            for b in 0..64 {
                if word >> b & 1 == 1 {
                    out.push((w * 64 + b) as u32);
                }
            }
        }
        out
    }
}

pub struct Inferred {
    /// For each task, the facts every refinement needs at its start.
    pub task: Vec<Vec<u32>>,
    /// For each method, the facts its subtasks need at the start, and for each `k`, the facts
    /// `subtasks[k..]` need at their start (`tails[m][0]` is the method's own).
    pub tails: Vec<Vec<Vec<u32>>>,
}

pub fn infer(model: &Model) -> Inferred {
    let n = model.num_state_bits;
    let num_tasks = model.num_tasks;

    let mut mayadd: Vec<Bits> = (0..num_tasks)
        .map(|t| {
            if t < model.num_actions {
                Bits::of(&model.actions[t].add, n)
            } else {
                Bits::empty(n)
            }
        })
        .collect();
    loop {
        let mut changed = false;
        for m in &model.methods {
            let mut add = mayadd[m.task as usize].clone();
            for &s in &m.subtasks {
                add.union(&mayadd[s as usize]);
            }
            if add != mayadd[m.task as usize] {
                mayadd[m.task as usize] = add;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    let sequence = |pre: &[Bits], subtasks: &[u32]| -> Bits {
        let mut need = Bits::empty(n);
        let mut added = Bits::empty(n);
        for &s in subtasks {
            need.union(&pre[s as usize].minus(&added));
            added.union(&mayadd[s as usize]);
        }
        need
    };

    let mut pre: Vec<Bits> = (0..num_tasks)
        .map(|t| {
            if t < model.num_actions {
                Bits::of(&model.actions[t].prec, n)
            } else {
                Bits::full(n)
            }
        })
        .collect();
    loop {
        let mut changed = false;
        for t in model.num_actions..num_tasks {
            let methods = &model.methods_for[t];
            if methods.is_empty() {
                continue;
            }
            let mut p = Bits::full(n);
            for &m in methods {
                p.intersect(&sequence(&pre, &model.methods[m as usize].subtasks));
            }
            if p != pre[t] {
                pre[t] = p;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    let tails = model
        .methods
        .iter()
        .map(|m| (0..m.subtasks.len()).map(|k| sequence(&pre, &m.subtasks[k..]).facts()).collect())
        .collect();
    let task = (0..num_tasks)
        .map(|t| {
            if t >= model.num_actions && model.methods_for[t].is_empty() {
                Vec::new()
            } else {
                pre[t].facts()
            }
        })
        .collect();
    Inferred { task, tails }
}

/// For each task, every fact an action reachable below it reads, adds or deletes: its
/// refinements' outcomes depend on the state through these facts alone, and change nothing else.
pub fn touched(model: &Model) -> Vec<Vec<u32>> {
    let n = model.num_state_bits;
    let mut touch: Vec<Bits> = (0..model.num_tasks)
        .map(|t| {
            if t < model.num_actions {
                let a = &model.actions[t];
                let mut b = Bits::of(&a.prec, n);
                b.union(&Bits::of(&a.add, n));
                b.union(&Bits::of(&a.del, n));
                b
            } else {
                Bits::empty(n)
            }
        })
        .collect();
    loop {
        let mut changed = false;
        for m in &model.methods {
            let mut t = touch[m.task as usize].clone();
            for &s in &m.subtasks {
                t.union(&touch[s as usize]);
            }
            if t != touch[m.task as usize] {
                touch[m.task as usize] = t;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    touch.iter().map(Bits::facts).collect()
}
