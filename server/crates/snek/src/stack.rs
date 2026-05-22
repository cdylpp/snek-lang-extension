use im::HashMap;

pub struct Stack {
    size: i64,
    map: HashMap<String, i64>,
}

impl Stack {
    /// Creates an empty compiler stack environment.
    pub fn new() -> Self {
        Stack {
            size: 0,
            map: HashMap::new(),
        }
    }

    /// Creates a stack environment for function parameters.
    ///
    /// Parameters are mapped to caller-frame slots starting at `-2`, matching
    /// the compiler's `[rbp - 8*slot]` operand convention for positive and
    /// negative slot indices.
    pub fn params(vars: &Vec<String>) -> Self {
        let mut map = HashMap::new();
        let mut i = 2;
        for v in vars {
            map.insert(v.to_string(), -i);
            i += 1;
        }
        Stack { size: 0, map }
    }

    /// Returns a new stack environment with `x` bound to the next local slot.
    ///
    /// The returned `i64` is the assigned slot number.
    pub fn push(&self, x: String) -> (i64, Stack) {
        let size = self.size + 1;
        let map = self.map.update(x, size);

        (size, Self { size, map })
    }

    /// Looks up the stack slot associated with `x`.
    pub fn get(&self, x: &String) -> Option<i64> {
        self.map.get(x).copied()
    }

    /// Returns whether `x` is bound in this stack environment.
    pub fn contains(&self, x: &String) -> bool {
        // return true if the stack contains the element
        self.map.contains_key(x)
    }

    /// Returns a copy of this stack environment.
    pub fn clone(&self) -> Stack {
        Stack {
            size: self.size,
            map: self.map.clone(),
        }
    }

    /// Returns a new stack environment with `k` bound to slot `v`.
    pub fn update(&self, k: String, v: i64) -> Stack {
        Stack {
            size: self.size,
            map: self.map.update(k, v),
        }
    }
}
