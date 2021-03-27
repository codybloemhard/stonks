use std::fs;
use std::collections::{ HashMap, HashSet };

use term_basics_linux as tbl;

fn main() {
    let contents = fs::read_to_string("sample.csv").expect("Couldn't read sample.");
    let mut state = State::new();
    let ts = contents.split('\n').into_iter().map(|line| line.to_string().into_trans(&mut state))
        .flatten().collect::<Vec<_>>();
    println!("{:?}", ts);
}

#[derive(Default)]
pub struct Ider{
    next_id: usize,
    ids: HashMap<String, usize>,
}

impl Ider{
    pub fn new() -> Self{
        Self{
            next_id: 1,
            ids: HashMap::new(),
        }
    }

    pub fn get_id(&mut self, string: String) -> usize{
        if &string == "null"{
            return 0;
        }
        if let Some(id) = self.ids.get(&string){
            *id
        } else {
            self.ids.insert(string, self.next_id);
            self.next_id += 1;
            self.next_id - 1
        }
    }
}

#[derive(Default)]
pub struct State{
    ids: Ider,
    ins: HashSet<usize>,
    tags: Ider,
}

impl State{
    pub fn new() -> Self{
        Self{
            ids: Ider::new(),
            ins: HashSet::new(),
            tags: Ider::new(),
        }
    }

    pub fn account_id(&mut self, string: String, set_in: bool) -> usize{
        let id = self.ids.get_id(string);
        if set_in{
            self.ins.insert(id);
        }
        id
    }

    pub fn tag_id(&mut self, string: String) -> usize{
        self.tags.get_id(string)
    }
}

#[derive(Debug)]
pub struct Trans{
    date: (u8, u8, u16),
    src: usize,
    dst: usize,
    amount: usize,
    comment: String,
    tags: Vec<usize>,
}

trait IntoTrans{
    fn into_trans(self, state: &mut State) -> Option<Trans>;
}

impl IntoTrans for String{
    fn into_trans(self, state: &mut State) -> Option<Trans>{
        let splitted = self.split(',').collect::<Vec<_>>();
        if splitted.len() < 6 { return None; }
        let triple = splitted[0].split(';').collect::<Vec<_>>();
        if triple.len() != 3 { return None; }
        let date: (u8, u8, u16) = (
            tbl::string_to_value(triple[0])?,
            tbl::string_to_value(triple[1])?,
            tbl::string_to_value(triple[2])?,
        );
        let src = state.account_id(splitted[1].to_string(), false);
        let dst = state.account_id(splitted[2].to_string(), true);
        let amount: usize = tbl::string_to_value(splitted[3])?;
        let comment = splitted[4].to_string();
        let tags = splitted.into_iter().skip(5).map(|raw_tag| state.tag_id(raw_tag.to_string()))
            .collect::<Vec<_>>();

        Some(Trans{
            date, src, dst, amount, comment, tags
        })
    }
}
