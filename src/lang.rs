pub struct Proj {
    pub name: String,
    pub path: String,
    pub is_ok: bool,
}

pub struct Lang {
    pub name: String,
    pub path: String,
    pub projs: Vec<Proj>,
    pub not_ok: i32,
}

impl Proj {
    pub fn new(name: &str, path: &str) -> Self {
        Proj {
            name: String::from(name),
            path: String::from(path),
            is_ok: true,
        }
    }
}

impl Lang {
    pub fn new(name: &str, path: &str) -> Self {
        Lang {
            name: String::from(name),
            path: String::from(path),
            projs: vec![],
            not_ok: 0,
        }
    }
    pub fn add_proj(&mut self, proj: Proj) {
        self.projs.push(proj)
    }
}
