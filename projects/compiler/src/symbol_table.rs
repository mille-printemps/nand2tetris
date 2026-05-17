use collections::hashmap::HashMap;
use collections::Empty;

use crate::ast::VarKind;

#[derive(Clone, Debug)]
pub struct Symbol {
    pub typ: String,
    pub kind: VarKind,
    pub index: u16,
}

#[derive(Clone)]
pub struct SymbolTable {
    class_scope: HashMap<String, Symbol>,
    subroutine_scope: HashMap<String, Symbol>,
    pub static_count: u16,
    pub field_count: u16,
    pub arg_count: u16,
    pub local_count: u16,
}

impl SymbolTable {
    pub fn new() -> Self {
        SymbolTable {
            class_scope: HashMap::empty(),
            subroutine_scope: HashMap::empty(),
            static_count: 0,
            field_count: 0,
            arg_count: 0,
            local_count: 0,
        }
    }

    // Returns a new table with the subroutine scope cleared, preserving class-level symbols.
    pub fn reset_subroutine(&self) -> Self {
        SymbolTable {
            class_scope: self.class_scope.clone(),
            subroutine_scope: HashMap::empty(),
            static_count: self.static_count,
            field_count: self.field_count,
            arg_count: 0,
            local_count: 0,
        }
    }

    /// Returns a new table with the given symbol added.
    pub fn define(&self, name: String, typ: String, kind: VarKind) -> Self {
        match kind {
            VarKind::Static => {
                let symbol = Symbol {
                    typ,
                    kind,
                    index: self.static_count,
                };
                SymbolTable {
                    class_scope: self.class_scope.insert(name, symbol),
                    subroutine_scope: self.subroutine_scope.clone(),
                    static_count: self.static_count + 1,
                    field_count: self.field_count,
                    arg_count: self.arg_count,
                    local_count: self.local_count,
                }
            }
            VarKind::Field => {
                let symbol = Symbol {
                    typ,
                    kind,
                    index: self.field_count,
                };
                SymbolTable {
                    class_scope: self.class_scope.insert(name, symbol),
                    subroutine_scope: self.subroutine_scope.clone(),
                    static_count: self.static_count,
                    field_count: self.field_count + 1,
                    arg_count: self.arg_count,
                    local_count: self.local_count,
                }
            }
            VarKind::Arg => {
                let symbol = Symbol {
                    typ,
                    kind,
                    index: self.arg_count,
                };
                SymbolTable {
                    class_scope: self.class_scope.clone(),
                    subroutine_scope: self.subroutine_scope.insert(name, symbol),
                    static_count: self.static_count,
                    field_count: self.field_count,
                    arg_count: self.arg_count + 1,
                    local_count: self.local_count,
                }
            }
            VarKind::Var => {
                let symbol = Symbol {
                    typ,
                    kind,
                    index: self.local_count,
                };
                SymbolTable {
                    class_scope: self.class_scope.clone(),
                    subroutine_scope: self.subroutine_scope.insert(name, symbol),
                    static_count: self.static_count,
                    field_count: self.field_count,
                    arg_count: self.arg_count,
                    local_count: self.local_count + 1,
                }
            }
        }
    }

    // Looks up a name: subroutine scope first, then class scope.
    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        let key = name.to_string();
        self.subroutine_scope
            .get(&key)
            .or_else(|| self.class_scope.get(&key))
    }
}
