//! Symbolic stack words: counted predicates never unroll a source INT.
use super::wires::Slice;
use super::*;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Node {
    Empty,
    Wire(Slice),
    Concat(Vec<Word>),
    Repeat(Word, BigUint),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Word {
    node: Rc<Node>,
    len: BigUint,
}
impl Default for Word {
    fn default() -> Self {
        Self {
            node: Rc::new(Node::Empty),
            len: BigUint::zero(),
        }
    }
}
impl Word {
    fn wire(w: Slice) -> Self {
        Self {
            node: Rc::new(Node::Wire(w)),
            len: 1u32.into(),
        }
    }
    fn cat(parts: Vec<Self>) -> Self {
        let mut flat = vec![];
        for part in parts {
            match part.node.as_ref() {
                Node::Empty => (),
                Node::Concat(items) => flat.extend(items.iter().cloned()),
                _ => flat.push(part),
            }
        }
        let mut merged: Vec<Self> = vec![];
        for word in flat {
            let (base, count) = match word.node.as_ref() {
                Node::Repeat(base, n) => (base.clone(), n.clone()),
                _ => (word.clone(), BigUint::from(1u32)),
            };
            if let Some(last) = merged.last_mut() {
                let (previous, copies) = match last.node.as_ref() {
                    Node::Repeat(base, n) => (base.clone(), n.clone()),
                    _ => (last.clone(), BigUint::from(1u32)),
                };
                if base == previous {
                    *last = Self::repeat(base, count + copies);
                    continue;
                }
            }
            merged.push(word);
        }
        let mut flat = merged;
        if flat.is_empty() {
            return Self::default();
        }
        if flat.len() == 1 {
            return flat.pop().expect("one word");
        }
        // Canonicalise repeated finite patterns, including repeated pushes of
        // the same wire, before representing large counted repetitions.
        for period in 1..=flat.len() / 2 {
            if flat.len() % period == 0
                && flat.iter().enumerate().all(|(i, w)| w == &flat[i % period])
            {
                return Self::repeat(
                    Self::cat(flat[..period].to_vec()),
                    BigUint::from(flat.len() / period),
                );
            }
        }
        let len = flat.iter().map(|w| &w.len).sum();
        Self {
            node: Rc::new(Node::Concat(flat)),
            len,
        }
    }
    fn repeat(word: Self, count: BigUint) -> Self {
        if count.is_zero() || word.len.is_zero() {
            return Self::default();
        }
        if count == 1u32.into() {
            return word;
        }
        if let Node::Repeat(inner, n) = word.node.as_ref() {
            return Self::repeat(inner.clone(), n * count);
        }
        let len = &word.len * &count;
        Self {
            node: Rc::new(Node::Repeat(word, count)),
            len,
        }
    }
    fn split(&self, n: &BigUint) -> (Self, Self) {
        assert!(n <= &self.len, "word split is within its symbolic length");
        if n.is_zero() {
            return (Self::default(), self.clone());
        }
        if n == &self.len {
            return (self.clone(), Self::default());
        }
        match self.node.as_ref() {
            Node::Concat(parts) => {
                let mut left = vec![];
                let mut right = vec![];
                let mut remain = n.clone();
                for part in parts {
                    if remain >= part.len {
                        left.push(part.clone());
                        remain -= &part.len;
                    } else {
                        let (a, b) = part.split(&remain);
                        left.push(a);
                        right.push(b);
                        remain = BigUint::zero();
                    }
                }
                (Self::cat(left), Self::cat(right))
            }
            Node::Repeat(inner, count) => {
                let copies = n / &inner.len;
                let tail = n % &inner.len;
                let (a, b) = inner.split(&tail);
                let remainder = if tail.is_zero() {
                    count - &copies
                } else {
                    count - &copies - 1u32
                };
                let right = if tail.is_zero() {
                    Self::repeat(inner.clone(), remainder)
                } else {
                    Self::cat(vec![b, Self::repeat(inner.clone(), remainder)])
                };
                (
                    Self::cat(vec![Self::repeat(inner.clone(), copies), a]),
                    right,
                )
            }
            _ => unreachable!("a one-wire word cannot be split internally"),
        }
    }
    fn equal(a: &Self, b: &Self) -> bool {
        if a.len != b.len {
            return false;
        }
        let mut work = vec![(a.clone(), b.clone())];
        while let Some((a, b)) = work.pop() {
            if a == b {
                continue;
            }
            match (a.node.as_ref(), b.node.as_ref()) {
                (Node::Wire(x), Node::Wire(y)) => {
                    if x != y {
                        return false;
                    }
                }
                _ => {
                    let length = match (a.node.as_ref(), b.node.as_ref()) {
                        (Node::Concat(p), _) => p[0].len.clone(),
                        (_, Node::Concat(p)) => p[0].len.clone(),
                        (Node::Repeat(p, _), Node::Repeat(q, _)) => {
                            std::cmp::min(p.len.clone(), q.len.clone())
                        }
                        _ => return false,
                    };
                    let (ap, ar) = a.split(&length);
                    let (bp, br) = b.split(&length);
                    work.push((ar, br));
                    work.push((ap, bp));
                }
            }
        }
        true
    }
    fn cancel(a: &Self, b: &Self) -> Result<(Self, Self), String> {
        let n = std::cmp::min(&a.len, &b.len);
        let (ap, ar) = a.split(n);
        let (bp, br) = b.split(n);
        require(
            Self::equal(&ap, &bp),
            "§3.5",
            "predicate pop does not match push",
        )?;
        Ok((ar, br))
    }
    pub fn push(&mut self, w: Slice) {
        *self = Self::cat(vec![Self::wire(w), self.clone()]);
    }
    pub fn pop(&mut self, w: &Slice) -> Result<(), String> {
        require(!self.len.is_zero(), "§3.5", "predicate stack underflow")?;
        let (top, rest) = self.split(&BigUint::from(1u32));
        require(
            Self::equal(&top, &Self::wire(w.clone())),
            "§3.5",
            "predicate pop does not match push",
        )?;
        *self = rest;
        Ok(())
    }
    pub fn is_empty(&self) -> bool {
        self.len.is_zero()
    }
    pub fn agrees(&self, other: &Self) -> bool {
        Self::equal(self, other)
    }
    pub fn wires(&self) -> Vec<&Slice> {
        let mut todo = vec![self];
        let mut result = vec![];
        while let Some(word) = todo.pop() {
            match word.node.as_ref() {
                Node::Wire(w) => result.push(w),
                Node::Concat(parts) => todo.extend(parts),
                Node::Repeat(inner, _) => todo.push(inner),
                Node::Empty => (),
            }
        }
        result
    }
}
#[derive(Clone, Default)]
pub(super) struct Effect {
    pop: Word,
    push: Word,
}
impl Effect {
    pub fn instruction(row: &Row, wire: Slice) -> Self {
        match row.code {
            Code::Pred => Self {
                pop: Word::default(),
                push: Word::wire(wire),
            },
            Code::Unpred => Self {
                pop: Word::wire(wire),
                push: Word::default(),
            },
            _ => unreachable!("only predicate instructions have stack effects"),
        }
    }
    pub fn then(&mut self, next: &Self) -> Result<(), String> {
        let (push, pop) = Word::cancel(&self.push, &next.pop)?;
        self.pop = Word::cat(vec![self.pop.clone(), pop]);
        self.push = Word::cat(vec![next.push.clone(), push]);
        Ok(())
    }
    pub fn counted(&self, count: &BigUint) -> Result<Self, String> {
        if count.is_zero() {
            return Ok(Self::default());
        }
        if count == &1u32.into() {
            return Ok(self.clone());
        }
        let (push, pop) = Word::cancel(&self.push, &self.pop)?;
        let extra = count - 1u32;
        Ok(Self {
            pop: Word::cat(vec![self.pop.clone(), Word::repeat(pop, extra.clone())]),
            push: Word::cat(vec![self.push.clone(), Word::repeat(push, extra)]),
        })
    }
    pub fn apply(&self, stack: &Word) -> Result<Word, String> {
        require(
            stack.len >= self.pop.len,
            "§3.5",
            "predicate stack underflow in counted loop",
        )?;
        let (pop, rest) = stack.split(&self.pop.len);
        require(
            Word::equal(&pop, &self.pop),
            "§3.5",
            "counted-loop predicate pop mismatch",
        )?;
        Ok(Word::cat(vec![self.push.clone(), rest]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn wire(name: &str) -> Slice {
        Slice {
            name: name.into(),
            start: BigUint::zero(),
            width: 1u32.into(),
        }
    }
    #[test]
    fn repeated_stack_effects_match_literal_push_pop_execution() {
        // Finite exhaustive words provide a separate stack-machine oracle.
        for length in 0..=5 {
            for encoding in 0usize..4usize.pow(length) {
                let mut code = encoding;
                let mut effect = Effect::default();
                let mut valid = true;
                let mut ops = vec![];
                for _ in 0..length {
                    let op = code % 4;
                    code /= 4;
                    let w = wire(if op % 2 == 0 { "a" } else { "b" });
                    let step = if op < 2 {
                        Effect {
                            pop: Word::default(),
                            push: Word::wire(w.clone()),
                        }
                    } else {
                        Effect {
                            pop: Word::wire(w.clone()),
                            push: Word::default(),
                        }
                    };
                    if effect.then(&step).is_err() {
                        valid = false;
                        break;
                    }
                    ops.push((op < 2, w));
                }
                if !valid {
                    continue;
                }
                for repeats in 0..=4 {
                    let initial = vec![wire("a"), wire("b"), wire("a"), wire("b")];
                    let mut actual = initial.clone();
                    let mut success = true;
                    'visits: for _ in 0..repeats {
                        for (push, w) in &ops {
                            if *push {
                                actual.push(w.clone());
                            } else if actual.pop().as_ref() != Some(w) {
                                success = false;
                                break 'visits;
                            }
                        }
                    }
                    let mut stack = Word::default();
                    for w in initial {
                        stack.push(w);
                    }
                    let result = effect
                        .counted(&BigUint::from(repeats as u32))
                        .and_then(|e| e.apply(&stack));
                    assert_eq!(result.is_ok(), success, "{length}/{encoding}/{repeats}");
                    if let Ok(mut word) = result {
                        while let Some(w) = actual.pop() {
                            word.pop(&w).unwrap();
                        }
                        assert!(word.is_empty());
                    }
                }
            }
        }
    }
    #[test]
    fn nested_huge_counts_keep_the_stack_symbolic() {
        let count = BigUint::from(10u32).pow(60);
        let mut push = Effect::default();
        for name in ["a", "b", "a", "b"] {
            push.then(&Effect {
                pop: Word::default(),
                push: Word::wire(wire(name)),
            })
            .unwrap();
        }
        let repeated = push.counted(&count).unwrap().counted(&count).unwrap();
        let mut pop = Effect::default();
        for name in ["b", "a", "b", "a"] {
            pop.then(&Effect {
                pop: Word::wire(wire(name)),
                push: Word::default(),
            })
            .unwrap();
        }
        let undo = pop.counted(&(&count * &count)).unwrap();
        let stack = repeated.apply(&Word::default()).unwrap();
        assert!(undo.apply(&stack).unwrap().is_empty());
    }
}
