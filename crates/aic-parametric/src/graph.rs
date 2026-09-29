use crate::expr::{Expr, ParseError};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

/// Fully-qualified parameter key, e.g. `#12.width`.
pub type ParamKey = String;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ParamError {
    #[error("parse error: {0}")]
    Parse(#[from] ParseError),
    #[error("dependency cycle: {}", path.join(" -> "))]
    Cycle { path: Vec<ParamKey> },
    #[error("unknown reference '{0}'")]
    Unresolved(String),
    #[error("{key}: {message}")]
    Eval { key: ParamKey, message: String },
    #[error("constraint {code} violated: {message}")]
    Constraint { code: String, message: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParamState {
    pub key: ParamKey,
    /// Text as entered by the user (e.g. `= cabinet.inner_width`).
    pub source: String,
    /// Expression with references resolved to absolute keys.
    pub expr: Expr,
    pub deps: Vec<ParamKey>,
    pub value: Result<f64, String>,
}

impl ParamState {
    pub fn is_expression(&self) -> bool {
        !self.expr.is_literal()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Constraint {
    pub id: String,
    /// Resolved boolean expression (non-zero == satisfied).
    #[serde(skip, default = "default_expr")]
    pub expr: Expr,
    pub source: String,
    pub code: String,
    pub message: String,
}

fn default_expr() -> Expr {
    Expr::Num(1.0)
}

#[derive(Debug, Clone, Default)]
pub struct ParamGraph {
    params: BTreeMap<ParamKey, ParamState>,
    /// Reverse edges: key -> params that reference it (may reference missing keys).
    dependents: HashMap<ParamKey, BTreeSet<ParamKey>>,
    constraints: BTreeMap<String, (Constraint, Vec<ParamKey>)>,
    /// Number of expression evaluations performed (for incremental-recompute tests).
    pub eval_count: u64,
}

impl ParamGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.params.len()
    }

    pub fn is_empty(&self) -> bool {
        self.params.is_empty()
    }

    pub fn get(&self, key: &str) -> Option<&ParamState> {
        self.params.get(key)
    }

    pub fn value(&self, key: &str) -> Option<f64> {
        self.params.get(key).and_then(|p| p.value.as_ref().ok().copied())
    }

    pub fn contains(&self, key: &str) -> bool {
        self.params.contains_key(key)
    }

    pub fn iter(&self) -> impl Iterator<Item = &ParamState> {
        self.params.values()
    }

    pub fn keys_with_prefix<'a>(&'a self, prefix: &'a str) -> impl Iterator<Item = &'a ParamKey> + 'a {
        self.params.range(prefix.to_string()..).map(|(k, _)| k).take_while(move |k| k.starts_with(prefix))
    }

    pub fn constraints(&self) -> impl Iterator<Item = &Constraint> {
        self.constraints.values().map(|(c, _)| c)
    }

    fn deps_of(expr: &Expr) -> Vec<ParamKey> {
        let mut v = Vec::new();
        expr.refs(&mut v);
        v.sort();
        v.dedup();
        v
    }

    /// Returns a cycle path if making `key` depend on `deps` would close a loop.
    fn find_cycle(&self, key: &str, deps: &[ParamKey], pending: &HashMap<&str, &[ParamKey]>) -> Option<Vec<ParamKey>> {
        // DFS from each dep following dependency edges; reaching `key` is a cycle.
        let mut stack: Vec<(ParamKey, Vec<ParamKey>)> =
            deps.iter().map(|d| (d.clone(), vec![key.to_string(), d.clone()])).collect();
        let mut seen = BTreeSet::new();
        while let Some((cur, path)) = stack.pop() {
            if cur == key {
                return Some(path);
            }
            if !seen.insert(cur.clone()) {
                continue;
            }
            let next: &[ParamKey] = match pending.get(cur.as_str()) {
                Some(d) => d,
                None => match self.params.get(&cur) {
                    Some(p) => &p.deps,
                    None => &[],
                },
            };
            for n in next {
                let mut p = path.clone();
                p.push(n.clone());
                stack.push((n.clone(), p));
            }
        }
        None
    }

    fn unlink(&mut self, key: &str) {
        if let Some(old) = self.params.get(key) {
            for d in old.deps.clone() {
                if let Some(set) = self.dependents.get_mut(&d) {
                    set.remove(key);
                    if set.is_empty() {
                        self.dependents.remove(&d);
                    }
                }
            }
        }
    }

    fn link(&mut self, key: &str, deps: &[ParamKey]) {
        for d in deps {
            self.dependents.entry(d.clone()).or_default().insert(key.to_string());
        }
    }

    /// Define or redefine one parameter. See [`ParamGraph::set_many`].
    pub fn set(&mut self, key: &str, source: &str, expr: Expr) -> Result<Vec<ParamKey>, ParamError> {
        self.set_many(vec![(key.to_string(), source.to_string(), expr)])
    }

    /// Transactionally define several parameters and incrementally recompute
    /// every downstream value. On cycle, evaluation error of a (re)defined key,
    /// newly broken dependents or violated constraints, everything is rolled back.
    /// Returns the keys whose value changed.
    pub fn set_many(&mut self, defs: Vec<(ParamKey, String, Expr)>) -> Result<Vec<ParamKey>, ParamError> {
        let resolved: Vec<(ParamKey, String, Expr, Vec<ParamKey>)> =
            defs.into_iter().map(|(k, s, e)| {
                let d = Self::deps_of(&e);
                (k, s, e, d)
            }).collect();
        {
            let pending: HashMap<&str, &[ParamKey]> =
                resolved.iter().map(|(k, _, _, d)| (k.as_str(), d.as_slice())).collect();
            for (k, _, _, d) in &resolved {
                if let Some(path) = self.find_cycle(k, d, &pending) {
                    return Err(ParamError::Cycle { path });
                }
            }
        }
        let roots: Vec<ParamKey> = resolved.iter().map(|r| r.0.clone()).collect();
        let backup: Vec<(ParamKey, Option<ParamState>)> =
            roots.iter().map(|k| (k.clone(), self.params.get(k).cloned())).collect();

        for (k, s, e, d) in resolved {
            self.unlink(&k);
            self.link(&k, &d);
            self.params.insert(k.clone(), ParamState { key: k, source: s, expr: e, deps: d, value: Err("pending".into()) });
        }
        let dirty = self.downstream(&roots);
        let prev: Vec<(ParamKey, Result<f64, String>)> = dirty
            .iter()
            .filter(|k| !roots.contains(k))
            .filter_map(|k| self.params.get(k).map(|p| (k.clone(), p.value.clone())))
            .collect();
        let changed_before: HashMap<ParamKey, Result<f64, String>> = backup
            .iter()
            .map(|(k, s)| (k.clone(), s.as_ref().map(|s| s.value.clone()).unwrap_or(Err("new".into()))))
            .chain(prev.iter().cloned())
            .collect();

        self.recompute(&dirty);

        // Validate: redefined keys must evaluate; previously-good dependents must stay good.
        let mut failure = None;
        for k in &roots {
            if let Some(Err(m)) = self.params.get(k).map(|p| &p.value) {
                failure = Some(ParamError::Eval { key: k.clone(), message: m.clone() });
                break;
            }
        }
        if failure.is_none() {
            for (k, old) in &prev {
                if old.is_ok() {
                    if let Some(Err(m)) = self.params.get(k).map(|p| &p.value) {
                        failure = Some(ParamError::Eval { key: k.clone(), message: m.clone() });
                        break;
                    }
                }
            }
        }
        if failure.is_none() {
            let touched: BTreeSet<&ParamKey> = dirty.iter().collect();
            failure = self.check_constraints(|deps| deps.iter().any(|d| touched.contains(d))).err();
        }
        if let Some(err) = failure {
            for (k, old) in backup {
                self.unlink(&k);
                match old {
                    Some(o) => {
                        let d = o.deps.clone();
                        self.link(&k, &d);
                        self.params.insert(k, o);
                    }
                    None => {
                        self.params.remove(&k);
                    }
                }
            }
            self.recompute(&dirty);
            return Err(err);
        }

        Ok(dirty
            .into_iter()
            .filter(|k| match (changed_before.get(k), self.params.get(k)) {
                (Some(old), Some(new)) => old != &new.value,
                _ => true,
            })
            .collect())
    }

    /// Remove parameters (e.g. when an object is deleted). Dependents are
    /// recomputed and will report an unresolved reference.
    pub fn remove(&mut self, keys: &[ParamKey]) -> Vec<ParamKey> {
        for k in keys {
            self.unlink(k);
            self.params.remove(k);
        }
        let dirty: Vec<ParamKey> = self.downstream(keys).into_iter().filter(|k| self.params.contains_key(k)).collect();
        self.recompute(&dirty);
        dirty
    }

    /// Topologically ordered closure of `roots` and all their dependents.
    fn downstream(&self, roots: &[ParamKey]) -> Vec<ParamKey> {
        let mut set: BTreeSet<ParamKey> = BTreeSet::new();
        let mut queue: VecDeque<ParamKey> = roots.iter().cloned().collect();
        while let Some(k) = queue.pop_front() {
            if !set.insert(k.clone()) {
                continue;
            }
            if let Some(ds) = self.dependents.get(&k) {
                queue.extend(ds.iter().cloned());
            }
        }
        // Kahn's algorithm restricted to `set`.
        let mut indeg: HashMap<&ParamKey, usize> = set.iter().map(|k| (k, 0)).collect();
        for k in &set {
            if let Some(p) = self.params.get(k) {
                for d in &p.deps {
                    if set.contains(d) {
                        *indeg.get_mut(k).unwrap() += 1;
                    }
                }
            }
        }
        let mut ready: VecDeque<&ParamKey> = indeg.iter().filter(|(_, n)| **n == 0).map(|(k, _)| *k).collect();
        let mut ready_sorted: Vec<&ParamKey> = ready.drain(..).collect();
        ready_sorted.sort();
        ready.extend(ready_sorted);
        let mut order = Vec::with_capacity(set.len());
        while let Some(k) = ready.pop_front() {
            order.push(k.clone());
            if let Some(ds) = self.dependents.get(k) {
                for d in ds {
                    if let Some(n) = indeg.get_mut(d) {
                        *n -= 1;
                        if *n == 0 {
                            ready.push_back(d);
                        }
                    }
                }
            }
        }
        order
    }

    fn recompute(&mut self, order: &[ParamKey]) {
        for k in order {
            let Some(p) = self.params.get(k) else { continue };
            let expr = p.expr.clone();
            let params = &self.params;
            let v = expr
                .eval(&|r: &str| match params.get(r) {
                    Some(p) => p.value.clone().map_err(|_| format!("'{r}' has no value")),
                    None => Err(format!("unknown reference '{r}'")),
                })
                .and_then(|v| if v.is_finite() { Ok(v) } else { Err("non-finite result".into()) });
            self.eval_count += 1;
            if let Some(p) = self.params.get_mut(k) {
                p.value = v;
            }
        }
    }

    pub fn add_constraint(&mut self, c: Constraint) {
        let deps = Self::deps_of(&c.expr);
        self.constraints.insert(c.id.clone(), (c, deps));
    }

    pub fn remove_constraints_with_prefix(&mut self, prefix: &str) {
        self.constraints.retain(|id, _| !id.starts_with(prefix));
    }

    fn check_constraints(&self, mut relevant: impl FnMut(&[ParamKey]) -> bool) -> Result<(), ParamError> {
        for (c, deps) in self.constraints.values() {
            if !relevant(deps) {
                continue;
            }
            let v = c.expr.eval(&|r: &str| self.value(r).ok_or_else(|| format!("unknown '{r}'")));
            match v {
                Ok(x) if x != 0.0 => {}
                // A constraint whose inputs don't exist (e.g. no shelves) is vacuous.
                Err(_) => {}
                Ok(_) => return Err(ParamError::Constraint { code: c.code.clone(), message: c.message.clone() }),
            }
        }
        Ok(())
    }

    /// Verify all constraints (e.g. after load).
    pub fn check_all_constraints(&self) -> Result<(), ParamError> {
        self.check_constraints(|_| true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;

    fn set(g: &mut ParamGraph, k: &str, s: &str) -> Result<Vec<ParamKey>, ParamError> {
        g.set(k, s, parse(s).unwrap())
    }

    #[test]
    fn dependent_dimensions_update() {
        let mut g = ParamGraph::new();
        set(&mut g, "cabinet.width", "800").unwrap();
        set(&mut g, "left.thickness", "18").unwrap();
        set(&mut g, "right.thickness", "18").unwrap();
        set(&mut g, "inner.width", "cabinet.width - left.thickness - right.thickness").unwrap();
        assert_eq!(g.value("inner.width"), Some(764.0));

        let changed = set(&mut g, "cabinet.width", "900").unwrap();
        assert_eq!(g.value("inner.width"), Some(864.0));
        assert!(changed.contains(&"inner.width".to_string()));
        assert!(!changed.contains(&"left.thickness".to_string()));
    }

    #[test]
    fn incremental_only_touches_downstream() {
        let mut g = ParamGraph::new();
        for i in 0..100 {
            set(&mut g, &format!("p{i}"), "1").unwrap();
        }
        set(&mut g, "a", "p1 * 2").unwrap();
        g.eval_count = 0;
        set(&mut g, "p1", "5").unwrap();
        assert_eq!(g.eval_count, 2, "only p1 and a are re-evaluated");
        assert_eq!(g.value("a"), Some(10.0));
    }

    #[test]
    fn detects_cycles_and_rolls_back() {
        let mut g = ParamGraph::new();
        set(&mut g, "a", "1").unwrap();
        set(&mut g, "b", "a + 1").unwrap();
        set(&mut g, "c", "b + 1").unwrap();
        let err = set(&mut g, "a", "c + 1").unwrap_err();
        assert!(matches!(err, ParamError::Cycle { .. }));
        assert_eq!(g.value("a"), Some(1.0));
        assert_eq!(g.value("c"), Some(3.0));
        assert!(matches!(set(&mut g, "x", "x + 1").unwrap_err(), ParamError::Cycle { .. }));
    }

    #[test]
    fn eval_errors_and_constraints_roll_back() {
        let mut g = ParamGraph::new();
        set(&mut g, "w", "800").unwrap();
        set(&mut g, "t", "18").unwrap();
        set(&mut g, "inv", "1 / (w - 800 + 1)").unwrap();
        g.add_constraint(Constraint {
            id: "c1".into(),
            expr: parse("w > 2 * t").unwrap(),
            source: "w > 2 * t".into(),
            code: "WIDTH_LESS_THAN_SIDES".into(),
            message: "too narrow".into(),
        });
        let e = set(&mut g, "w", "30").unwrap_err();
        assert!(matches!(e, ParamError::Constraint { .. }), "{e:?}");
        assert_eq!(g.value("w"), Some(800.0));
        // Would break a dependent (division by zero) -> rejected.
        assert!(set(&mut g, "w", "799").is_err());
        assert_eq!(g.value("inv"), Some(1.0));
    }

    #[test]
    fn missing_reference_resolves_later() {
        let mut g = ParamGraph::new();
        // Defining something that references a missing key is an error for that key.
        assert!(set(&mut g, "a", "b * 2").is_err());
        set(&mut g, "b", "2").unwrap();
        set(&mut g, "a", "b * 2").unwrap();
        g.remove(&["b".to_string()]);
        assert_eq!(g.value("a"), None);
    }
}
