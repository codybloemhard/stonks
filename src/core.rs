use term_basics_linux as tbl;

use std::collections::{ HashMap };

pub const REAL_FIAT: usize = 0;
pub const FIAT: usize = 1;

pub const NULL: usize = 0;
pub const FLOW: usize = 1;
pub const INTERNAL_FLOW: usize = 2;
pub const NET: usize = 3;
pub const NET_NEG: usize = 4;
pub const NET_POS: usize = 5;
pub const TRA: usize = 6;
pub const TRA_NEG: usize = 7;
pub const TRA_POS: usize = 8;
pub const YIELD: usize = 9;
pub const YIELD_NEG: usize = 10;
pub const YIELD_POS: usize = 11;
pub const SPENDING_MONTH: usize = 12;
pub const SPENDING_CUMULATIVE: usize = 13;
pub const RECEIVING_MONTH: usize = 14;
pub const RECEIVING_CUMULATIVE: usize = 15;

pub const NR_BUILDIN_ACCOUNTS: usize = 16;

pub type NamedBalance = (String, f32);

pub fn into_named_accounts(bs: &[f32], state: &NameBank) -> Vec<NamedBalance>{
    bs.iter().copied().enumerate().map(|(id, val)| (state.account_name(id), val)).collect::<Vec<_>>()
}

pub fn into_named_assets(bs: &[f32], state: &NameBank) -> Vec<NamedBalance>{
    bs.iter().copied().enumerate().map(|(id, val)| (state.asset_name(id), val)).collect::<Vec<_>>()
}

pub type MonthDate = (u8, u16);

pub fn hist(state: &mut State, ts: &[Trans]) -> (Vec<Vec<f32>>, MonthDate){
    let mut hist = Vec::new();
    if ts.is_empty() { return (hist, (0, 0)); }
    let mut from = 0;
    let mut date = (ts[0].date.1, ts[0].date.2);
    let start_date = date;
    let mut prev_frame = Vec::new();
    loop{
        let (new_from, new_date) = update(ts, state, Some(from), Some(date));
        // we have a frame for every month, fill in months the data skips
        while (new_date.0 < date.0 + 1 && new_date.1 <= date.1)
                && !(new_date.0 == 1 && new_date.1 == date.1 + 1)
                && (new_date != date){
            hist.push(prev_frame.clone());
            date = if date.0 == 12{
                (1, date.1 + 1)
            } else {
                (date.0 + 1, date.1)
            };
        }
        let frame = state.accounts.clone();
        hist.push(frame.clone());
        if new_from >= ts.len(){
            break;
        }
        from = new_from;
        date = new_date;
        prev_frame = frame;
    }
    (hist, start_date)
}

pub fn update(ts: &[Trans], state: &mut State, from: Option<usize>, from_date: Option<MonthDate>) -> (usize, MonthDate){
    let skip = if let Some(skip) = from { skip } else { 0 };
    let all = from.is_none();
    let mut date = from_date.unwrap_or((0, 0));
    let mut spending_acc = 0.0;
    let mut receiving_acc = 0.0;
    for (i, trans) in ts.iter().skip(skip).enumerate(){
        if !all && (trans.date.1 != date.0 || trans.date.2 != date.1){
            let next = skip + i;
            let date = (trans.date.1, trans.date.2);
            state.accounts[SPENDING_MONTH] = spending_acc;
            state.accounts[SPENDING_CUMULATIVE] += spending_acc;
            state.accounts[RECEIVING_MONTH] = receiving_acc;
            state.accounts[RECEIVING_CUMULATIVE] += receiving_acc;
            return (next, date);
        } else {
            date = (trans.date.1, trans.date.2);
        }

        match trans.ext{
            TransExt::Set { amount, dst } => {
                let diff = amount - state.accounts[dst];
                if dst != NULL{
                    state.accounts[NET] += diff;
                    state.accounts[NET_NEG] += diff.min(0.0);
                    state.accounts[NET_POS] += diff.max(0.0);
                    state.accounts[YIELD] += diff;
                    state.accounts[YIELD_NEG] += diff.min(0.0);
                    state.accounts[YIELD_POS] += diff.max(0.0);
                }
                if state.account_labels[dst] == AccountLabel::Fiat{
                    state.asset_amounts[REAL_FIAT] += diff;
                }
                state.accounts[dst] = amount;
            },
            TransExt::Mov { src, dst, amount } => {
                state.accounts[src] -= amount;
                state.accounts[dst] += amount;
                state.accounts[FLOW] += amount;
                if src != NULL && dst != NULL {
                    state.accounts[INTERNAL_FLOW] += amount;
                } else if src != NULL && dst == NULL{
                    state.accounts[NET] -= amount;
                    state.accounts[NET_NEG] += amount;
                    if state.account_labels[src] != AccountLabel::Debt{
                        spending_acc += amount;
                    }
                } else if src == NULL && dst != NULL{
                    state.accounts[NET] += amount;
                    state.accounts[NET_POS] += amount;
                    if state.account_labels[dst] != AccountLabel::Debt{
                        receiving_acc += amount;
                    }
                }
                let srcl = state.account_labels[src];
                let dstl = state.account_labels[dst];
                if srcl == AccountLabel::Fiat && dstl != AccountLabel::Fiat{
                    state.asset_amounts[REAL_FIAT] -= amount;
                    // When used correctly, this FIAT is converted away to assets
                    if dstl == AccountLabel::Assets{
                        state.asset_amounts[FIAT] += amount;
                    }
                } else if dstl == AccountLabel::Fiat && srcl != AccountLabel::Fiat{
                    state.asset_amounts[REAL_FIAT] += amount;
                    // When used correctly, this FIAT is converted away to assets
                    if srcl == AccountLabel::Assets{
                        state.asset_amounts[FIAT] -= amount;
                    }
                }
            },
            TransExt::Tra { src, dst, sub, add } => {
                state.accounts[src] -= sub;
                state.accounts[dst] += add;
                state.accounts[FLOW] += sub.max(add);
                let diff = add - sub;
                if diff >= 0.0 { state.accounts[TRA_POS] += diff; }
                else if diff < 0.0 { state.accounts[TRA_NEG] -= diff; }
                state.accounts[TRA] += diff;
                if src != NULL && dst != NULL{
                    state.accounts[INTERNAL_FLOW] += sub.max(add);
                    state.accounts[NET] += diff;
                    state.accounts[NET_NEG] += diff.min(0.0);
                    state.accounts[NET_POS] += diff.max(0.0);
                } else if src != NULL && dst == NULL{
                    state.accounts[NET] -= sub;
                    state.accounts[NET_NEG] += sub;
                    if state.account_labels[src] != AccountLabel::Debt{
                        spending_acc += sub;
                    }
                } else if src == NULL && dst != NULL{
                    state.accounts[NET] += add;
                    state.accounts[NET_POS] += add;
                    if state.account_labels[dst] != AccountLabel::Debt{
                        receiving_acc += sub;
                    }
                }
                let srcl = state.account_labels[src];
                let dstl = state.account_labels[dst];
                if srcl == AccountLabel::Fiat && dstl != AccountLabel::Fiat{
                    state.asset_amounts[REAL_FIAT] -= sub;
                    // When used correctly, this FIAT is converted away to assets
                    if dstl == AccountLabel::Assets{
                        state.asset_amounts[FIAT] += add;
                    }
                } else if dstl == AccountLabel::Fiat && srcl != AccountLabel::Fiat{
                    state.asset_amounts[REAL_FIAT] += add;
                    // When used correctly, this FIAT is converted away to assets
                    // convert X assets to ADD fiat, making FIAT 0 again
                    if srcl == AccountLabel::Assets{
                        state.asset_amounts[FIAT] -= add;
                    }
                }
            },
            TransExt::Dec { asset, amount } => {
                state.asset_amounts[asset] = amount;
            },
            TransExt::Pri { asset, amount, worth } => {
                state.asset_prices[asset] = worth / amount;
            },
            TransExt::Pin { asset, amount, worth } => {
                state.asset_prices[asset] = worth / amount;
                state.asset_amounts[asset] = amount;
            },
            TransExt::Con { src, src_amount, dst, dst_amount } => {
                state.asset_amounts[src] -= src_amount;
                state.asset_amounts[dst] += dst_amount;
            },
            TransExt::Ass { account } => {
                state.account_labels[account] = AccountLabel::Assets;
            },
            TransExt::Deb { account } => {
                state.account_labels[account] = AccountLabel::Debt;
            }
        }
    }
    (usize::MAX, date)
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum AccountLabel{
    Null,
    Fiat,
    Assets,
    Debt,
}

pub struct State{
    pub accounts: Vec<f32>,
    pub account_labels: Vec<AccountLabel>,
    pub asset_amounts: Vec<f32>,
    pub asset_prices: Vec<f32>,
}

impl State{
    pub fn new(nb: &NameBank) -> Self{
        let mut account_labels = vec![AccountLabel::Fiat; nb.accounts.next_id];
        account_labels[0] = AccountLabel::Null;
        let mut asset_prices = vec![0.0; nb.assets.next_id];
        asset_prices[0] = 1.0;
        Self{
            accounts: vec![0.0; nb.accounts.next_id],
            account_labels,
            asset_amounts: vec![0.0; nb.assets.next_id],
            asset_prices,
        }
    }
}

#[derive(Default)]
pub struct Ider{
    next_id: usize,
    ids: HashMap<String, usize>,
}

impl Ider{
    pub fn new() -> Self{
        Self{
            next_id: 0,
            ids: HashMap::new(),
        }
    }

    pub fn get_id(&mut self, string: String) -> usize{
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
pub struct NameBank{
    accounts: Ider,
    account_names: HashMap<usize, String>,
    assets: Ider,
    asset_names: HashMap<usize, String>,
    tags: Ider,
}

impl NameBank{
    pub fn new() -> Self{
        let temp = Self{
            accounts: Ider::new(),
            account_names: HashMap::new(),
            assets: Ider::new(),
            asset_names: HashMap::new(),
            tags: Ider::new(),
        };
        temp.set_defaults()
    }

    fn set_defaults(mut self) -> Self{
        self.account_id("null".to_owned());
        self.account_id("_flow".to_owned());
        self.account_id("_internal_flow".to_owned());
        self.account_id("_net".to_owned());
        self.account_id("_net_lost".to_owned());
        self.account_id("_net_gained".to_owned());
        self.account_id("_tra".to_owned());
        self.account_id("_tra_lost".to_owned());
        self.account_id("_tra_gained".to_owned());
        self.account_id("_yield".to_owned());
        self.account_id("_yield_lost".to_owned());
        self.account_id("_yield_gained".to_owned());
        self.account_id("_spending_month".to_owned());
        self.account_id("_spending_cumulative".to_owned());
        self.account_id("_receiving_month".to_owned());
        self.account_id("_receiving_cumulative".to_owned());
        self.asset_id("REAL_FIAT".to_owned());
        self.asset_id("FIAT".to_owned());
        self
    }

    pub fn account_id(&mut self, string: String) -> usize{
        let id = self.accounts.get_id(string.clone());
        self.account_names.insert(id, string);
        id
    }

    pub fn account_name(&self, id: usize) -> String{
        if let Some(name) = self.account_names.get(&id){
            name.to_string()
        } else {
            String::from("unnamed")
        }
    }

    pub fn asset_id(&mut self, string: String) -> usize{
        let id = self.assets.get_id(string.clone());
        self.asset_names.insert(id, string);
        id
    }

    pub fn asset_name(&self, id: usize) -> String{
        if let Some(name) = self.asset_names.get(&id){
            name.to_string()
        } else {
            String::from("unnamed")
        }
    }

    pub fn tag_id(&mut self, string: String) -> usize{
        self.tags.get_id(string)
    }

    pub fn next_account_id(&self) -> usize{
        self.accounts.next_id
    }
}

#[derive(Debug)]
pub enum TransExt{
    Mov{
        src: usize,
        dst: usize,
        amount: f32,
    },
    Set{
        amount: f32,
        dst: usize,
    },
    Tra{
        src: usize,
        dst: usize,
        sub: f32,
        add: f32,
    },
    Dec{
        asset: usize,
        amount: f32,
    },
    Pri{
        asset: usize,
        amount: f32,
        worth: f32,
    },
    Pin{
        asset: usize,
        amount: f32,
        worth: f32,
    },
    Con{
        src: usize,
        dst: usize,
        src_amount: f32,
        dst_amount: f32,
    },
    Ass{
        account: usize,
    },
    Deb{
        account: usize,
    },
}

pub type Date = (u8, u8, u16);

#[derive(Debug)]
pub struct Trans{
    date: Date,
    tags: Vec<usize>,
    ext: TransExt,
}

pub trait IntoTrans{
    fn into_trans(self, state: &mut NameBank, date: &mut Date) -> Option<Trans>;
}

impl IntoTrans for String{
    fn into_trans(self, nb: &mut NameBank, date: &mut Date) -> Option<Trans>{
        if self.is_empty() { return None; }
        if self.starts_with('#') { return None; }
        let splitted = self.split(',').collect::<Vec<_>>();
        if splitted.len() < 2 { return None; }
        let parse_date = |string: &str| {
            let triple = string.split(';').collect::<Vec<_>>();
            if triple.len() != 3 { return None; }
            Some((
                tbl::string_to_value(triple[0])?,
                tbl::string_to_value(triple[1])?,
                tbl::string_to_value(triple[2])?,
            ))
        };
        if splitted[1] != "_"{
            *date = parse_date(splitted[1])?;
        }
        let tags_ind;
        let ext = match splitted[0]{
            "dat" => {
                *date = parse_date(splitted[1])?;
                return None;
            },
            "mov" => {
                tags_ind = 6;
                TransExt::Mov{
                    src: nb.account_id(splitted[2].to_string()),
                    dst: nb.account_id(splitted[3].to_string()),
                    amount: tbl::string_to_value(splitted[4])?,
                }
            },
            "set" => {
                tags_ind = 5;
                TransExt::Set{
                    dst: nb.account_id(splitted[2].to_string()),
                    amount: tbl::string_to_value(splitted[3])?,
                }
            },
            "tra" => {
                tags_ind = 7;
                TransExt::Tra{
                    src: nb.account_id(splitted[2].to_string()),
                    dst: nb.account_id(splitted[3].to_string()),
                    sub: tbl::string_to_value(splitted[4])?,
                    add: tbl::string_to_value(splitted[5])?,
                }
            },
            "dec" => {
                tags_ind = 4;
                TransExt::Dec{
                    asset: nb.asset_id(splitted[2].to_string()),
                    amount: tbl::string_to_value(splitted[3])?,
                }
            },
            "pri" => {
                tags_ind = 5;
                TransExt::Pri{
                    asset: nb.asset_id(splitted[2].to_string()),
                    amount: tbl::string_to_value(splitted[3])?,
                    worth: tbl::string_to_value(splitted[4])?,
                }
            },
            "pin" => {
                tags_ind = 5;
                TransExt::Pin{
                    asset: nb.asset_id(splitted[2].to_string()),
                    amount: tbl::string_to_value(splitted[3])?,
                    worth: tbl::string_to_value(splitted[4])?,
                }
            },
            "con" => {
                tags_ind = 6;
                TransExt::Con{
                    src: nb.asset_id(splitted[2].to_string()),
                    src_amount: tbl::string_to_value(splitted[3])?,
                    dst: nb.asset_id(splitted[4].to_string()),
                    dst_amount: tbl::string_to_value(splitted[5])?,
                }
            },
            "ass" => {
                tags_ind = 3;
                TransExt::Ass{
                    account: nb.account_id(splitted[2].to_string()),
                }
            },
            "deb" => {
                tags_ind = 3;
                TransExt::Deb{
                    account: nb.account_id(splitted[2].to_string()),
                }
            },
            _ => return None,
        };
        let tags = splitted.into_iter().skip(tags_ind).map(|raw_tag| nb.tag_id(raw_tag.to_string()))
            .collect::<Vec<_>>();

        Some(Trans{
            date: *date, tags, ext
        })
    }
}
