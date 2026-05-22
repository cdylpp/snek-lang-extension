

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Defn {
    pub name: String, 
    pub params: Vec<String>,
    pub body: Expr,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Prog {
    pub defns: Vec<Defn>,
    pub main: Expr,
}