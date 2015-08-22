use std::cell::RefCell;
use std::rc::Rc;

extern crate bit_vec;
use self::bit_vec::BitVec;

use grammar::{AIdx, Grammar, NIdx, Symbol, SIdx, TIdx};

/// Firsts stores all the first sets for a given grammar.
#[derive(Debug)]
pub struct Firsts {
    // The representation is a contiguous bitfield, of (terms_len * 1) * nonterms_len. Put another
    // way, each nonterminal has (terms_len + 1) bits, where the bit at position terms_len
    // represents epsilon.
    bf: BitVec,
    terms_len: NIdx
}

impl Firsts {
    fn new(nonterms_len: NIdx, terms_len: TIdx) -> Firsts {
        Firsts {
            bf        : BitVec::from_elem((nonterms_len * (terms_len + 1)), false),
            terms_len : terms_len
        }
    }

    /// Returns true if the firsts bit for terminal `tidx` nonterminal `nidx` is set, or
    /// false otherwise. Bit `terms_len` represents epsilon.
    pub fn get(&self, nidx: NIdx, tidx: TIdx) -> bool {
        self.bf[nidx * (self.terms_len + 1) + tidx]
    }

    /// Ensures that the firsts bit for terminal `tidx` nonterminal `nidx` is set. Returns true if
    /// it was already set, or false otherwise. Bit `terms_len` represents epsilon.
    pub fn set(&mut self, nidx: NIdx, tidx: TIdx) -> bool {
        if self.get(nidx, tidx) {
            true
        }
        else {
            self.bf.set((nidx * (self.terms_len + 1) + tidx), true);
            false
        }
    }
}


/// Generates and returns the firsts set for the given grammar.
///
/// # Example
/// Given a grammar `input`:
///
/// ```c
/// S : A "b";
/// A : "a" |;
/// ```
///
/// ```c
/// let ast = parse_yacc(&input);
/// let grm = ast_to_grammar(&ast);
/// let firsts = calc_firsts(&grm);
/// ```
pub fn calc_firsts(grm: &Grammar) -> Firsts {
    let mut firsts = Firsts::new(grm.nonterms_len, grm.terms_len);

    // Loop looking for changes to the firsts set, until we reach a fixed point. In essence, we
    // look at each rule E, and see if any of the nonterminals at the start of its alternatives
    // have new elements in since we last looked. If they do, we'll have to do another round.
    loop {
        let mut changed = false;
        for (rul_i, alts) in grm.rules_alts.iter().enumerate() {
            // For each rule E
            for alt_i in alts.iter() {
                // ...and each alternative A
                let ref alt = grm.alts[*alt_i];
                if alt.len() == 0 {
                    // if it's an empty alternative, ensure this nonterminal's epsilon bit is set.
                    if !firsts.set(rul_i, grm.terms_len) {
                        changed = true;
                    }
                    continue;
                }
                for (sym_i, sym) in alt.iter().enumerate() {
                    match sym {
                        &Symbol::Terminal(term_i) => {
                            // if symbol is a Terminal, add to FIRSTS
                            if !firsts.set(rul_i, term_i) {
                                changed = true;
                            }
                            break;
                        },
                        &Symbol::Nonterminal(nonterm_i) => {
                            // if we're dealing with another Nonterminal, union its FIRSTs together
                            // with the current nonterminals FIRSTs. Note this is (intentionally) a
                            // no-op if the two terminals are one and the same.
                            for bit_i in 0..grm.terms_len {
                                if firsts.get(nonterm_i, bit_i)
                                  && !firsts.set(rul_i, bit_i) {
                                    changed = true;
                                }
                            }

                            // If the epsilon bit in the nonterminal being referenced is set, and
                            // if its the last symbol in the alternative, then add epsilon to
                            // FIRSTs.
                            if firsts.get(nonterm_i, grm.terms_len) && sym_i == alt.len() - 1 {
                                // only add epsilon if the symbol is the last in the production
                                if !firsts.set(rul_i, grm.terms_len) {
                                    changed = true;
                                }
                            }

                            // If FIRST(X) of production R : X Y2 Y3 doesn't contain epsilon, then
                            // don't try and calculate the FIRSTS of Y2 or Y3 (i.e. stop now).
                            if !firsts.get(nonterm_i, grm.terms_len) {
                                break;
                            }
                        },
                    }
                }
            }
        }
        if !changed {
            return firsts;
        }
    }
}

/*
/// Generates and returns the follow set for the given grammar.
///
/// # Example
/// Given a grammar `grm`:
///
/// ```c
/// S : A "b";
/// A : "a" |;
/// ```
///
/// ```c
/// let firsts = calc_firsts(&grm);
/// let follows = calc_follows(&grm, &firsts);
/// println!(follows); // {"S": {}, "A": {"b"}};
/// ```
pub fn calc_follows(grm: &GrammarAST, firsts: &HashMap<String, HashSet<String>>)
                    -> HashMap<String, HashSet<String>> {
    // initialise follow set
    let mut follows: HashMap<String, HashSet<String>> = HashMap::new();
    for rule in grm.rules.values() {
        follows.insert(rule.name.clone(), HashSet::new());
    }

    let mut changed;
    loop {
        changed = false;
        for rule in grm.rules.values() {
            for alt in rule.alternatives.iter() {
                for (sym_i, sym) in alt.iter().enumerate() {
                    match sym {
                        &Symbol::Terminal(_) => continue,
                        &Symbol::Nonterminal(ref name) => {
                            let mut new = HashSet::new();
                            // add FIRSTS(succeeding symbols) to temporary follow set
                            let followers = alt[sym_i+1..].to_vec();
                            let f = get_firsts_from_symbols(firsts, &followers);
                            for e in f.iter() {
                                if e != "" {
                                    new.insert(e.clone());
                                }
                            }
                            // if no symbols are following sym, or FIRST(followers) contains epsilon, then add
                            // FOLLOW(rule.name) to the set as well
                            if followers.len() == 0 || f.contains("") {
                                let rule_follow = follows.get(&rule.name).unwrap();
                                for e in rule_follow {
                                    new.insert(e.clone());
                                }
                            }
                            // add everything from temporary set to current follow set
                            let mut old = follows.get_mut(name).unwrap();
                            for e in new {
                                if !old.contains(&e) {
                                    old.insert(e.clone());
                                    changed = true;
                                }
                            }
                        }
                    }
                }
            }
        }
        if !changed {
            return follows;
        }
    }
}
*/

#[derive(Debug)]
pub struct Closure {
    pub itemset: Rc<Vec<RefCell<ClosureAlt>>>
}

#[derive(Debug)]
pub struct ClosureAlt {
    pub active: BitVec,
    pub dots: BitVec
}

impl Closure {
    pub fn new(grm: &Grammar) -> Closure {
        let mut itemset = Vec::with_capacity(grm.alts.len());
        for alt in grm.alts.iter() { 
            let num_syms = alt.len() + 1;
            itemset.push(RefCell::new(ClosureAlt {
                active: BitVec::from_elem(num_syms, false),
                dots  : BitVec::from_elem(grm.terms_len * num_syms, false)
            }));
        }
        Closure {itemset: Rc::new(itemset)}
    }

    pub fn add(&mut self, grm: &Grammar, aidx: AIdx, dot: SIdx, la: &BitVec) {
        let mut alt_cl = self.itemset[aidx].borrow_mut();
        alt_cl.active.set(dot, true);
        let dots = &mut alt_cl.dots;
        for (i, bit) in la.iter().enumerate() {
            if bit {
                dots.set(dot * grm.nonterms_len + i, true);
            }
        }
    }

    pub fn close(&mut self, grm: &Grammar, firsts: &Firsts) {
        let itemset = &self.itemset;
        let mut new_la = BitVec::from_elem(grm.terms_len, false);
        loop {
            let mut changed = false;
            for i in 0..itemset.len() {
                let alt = &grm.alts[i];
                for dot in 0..alt.len() {
                    if !itemset[i].borrow().active[dot] { continue; }
                    if dot == alt.len() { continue; }
                    if let Symbol::Nonterminal(nonterm_i) = alt[dot] {
                        new_la.clear();
                        let mut nullabled = false;
                        for k in dot + 1..alt.len() {
                            match alt[k] {
                                Symbol::Terminal(term_j) => {
                                    new_la.set(term_j, true);
                                    nullabled = true;
                                    break;
                                },
                                Symbol::Nonterminal(nonterm_j) => {
                                    for l in 0..grm.terms_len {
                                        if firsts.get(nonterm_j, l) {
                                            new_la.set(l, true);
                                        }
                                    }
                                    if !firsts.get(nonterm_j, grm.terms_len) {
                                        nullabled = true;
                                        break;
                                    }
                                }
                            }
                        }
                        if !nullabled {
                            let dots = &itemset[i].borrow().dots;
                            for l in 0..grm.terms_len {
                                if dots[dot * grm.terms_len + l] {
                                    new_la.set(l, true);
                                }
                            }
                        }

                        for alt_i in grm.rules_alts[nonterm_i].iter() {
                            let mut clalt = itemset[*alt_i].borrow_mut();
                            if !clalt.active[0] {
                                clalt.active.set(0, true);
                                changed = true;
                            }
                            for l in 0..grm.terms_len {
                                if new_la[l] && !clalt.dots[l] {
                                    clalt.dots.set(l, true);
                                    changed = true;
                                }
                            }
                        }
                    }
                }
            }
            if !changed { break; }
        }
    }
}

/*
/// Calculates the goto that results from reading past a certain symbol in another set.
pub fn goto1(grm: &GrammarAST, firsts: &HashMap<String, HashSet<String>>,
             state: &HashMap<LR1Item, HashSet<String>>, symbol: &Symbol)
             -> HashMap<LR1Item, HashSet<String>> {
    let mut goto = HashMap::new();

    for (item, la) in state.iter() {
        if item.next() != None {
            if &item.next().unwrap() == symbol {
                // Clone item and insert into new goto set
                let lhs = item.lhs.clone();
                let rhs = item.rhs.clone();
                let dot = item.dot + 1;
                let gotoitm = LR1Item::new(lhs, rhs, dot);
                let gotola = la.clone();
                goto.insert(gotoitm, gotola);
            }
        }
    }
    closure1(grm, firsts, &mut goto);
    goto
}

pub struct StateGraph {
    pub states: Vec<HashMap<LR1Item, HashSet<String>>>,
    pub edges: HashMap<(i32, Symbol), i32>
}

impl StateGraph {
    pub fn contains(&self, state: &HashMap<LR1Item, HashSet<String>>) -> bool {
        self.states.contains(state)
    }
}

/// Builds a `StateGraph` from the given `Grammar`.
pub fn build_graph(grm: &GrammarAST) -> StateGraph {
    let mut states = Vec::new();
    let mut edges = HashMap::new();
    let mut todo = Vec::new();

    // calculate first sets
    let firsts = calc_firsts(&grm);

    // Create first state
    let item = lritem("Start!", vec![nonterminal(&grm.start.clone().unwrap())], 0);
    let mut la = HashSet::new();
    la.insert("$".to_string());
    let mut s0 = HashMap::new();
    s0.insert(item, la);
    closure1(&grm, &firsts, &mut s0);

    // add to list of states as #0
    todo.push(s0);

    let mut current_id = 0;
    let mut unique_id = 1;

    loop {
        if todo.len() == 0 {
            break;
        }
        let state = todo.remove(0);

        let mut symbols_done = HashSet::new();
        for (item, _) in state.iter() {
            let symbol = match item.next() {
                Some(x) => x,
                None => continue
            };
            if symbols_done.contains(&symbol) {
                continue;
            }
            else {
                // Cache processed symbols so that we don't create the same
                // goto set multiple times for different rules with the same
                // next symbol
                symbols_done.insert(symbol.clone());
            }
            let goto = goto1(&grm, &firsts, &state, &symbol);
            // This is slow! We are better off hashing the state HashMaps and making `states` a set
            // instead of a vector.
            // Although on second thought, when we add the weakly_compatible optimisation later we
            // might have to iterate over all states anyway
            match states.iter().position(|s| s == &goto) {
                // If goto is already contained map current_id to its position...
                Some(pos) => {edges.insert((current_id, symbol.clone()), pos as i32); ()},
                // ...otherwise add goto to todo-list and map current_id to its unique_id
                None => {
                    todo.push(goto);
                    edges.insert((current_id, symbol.clone()), unique_id);
                    unique_id += 1;
                    ()
                }
            }
        }

        states.push(state);
        current_id += 1;
    }

    StateGraph {states: states, edges: edges}
}
*/
