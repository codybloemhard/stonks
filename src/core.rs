use term_basics_linux as tbl;

use std::collections::{ HashMap };

pub const REAL_FIAT: usize = 0;
pub const FIAT: usize = 1;

pub const NULL: usize = 0;
pub const FLOW: usize = 1;
pub const INTERNAL_FLOW: usize = 2;
pub const NET: usize = 3;
pub const ASSETS: usize = 4;
pub const TRA: usize = 5;
pub const YIELD: usize = 6;
pub const ROI: usize = 7;
pub const SPENDING_MONTH: usize = 8;
pub const SPENDING_CUMULATIVE: usize = 9;
pub const RECEIVING_MONTH: usize = 10;
pub const RECEIVING_CUMULATIVE: usize = 11;

pub const NR_BUILDIN_ACCOUNTS: usize = 12;

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
    state.accounts[ROI] = 1.0;
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
                    state.accounts[YIELD] += diff;
                    if state.account_labels[dst] == AccountLabel::Assets{
                        let old = state.accounts[ASSETS];
                        state.accounts[ASSETS] += diff;
                        let roi = state.accounts[ASSETS] / old;
                        state.accounts[ROI] *= roi;
                    }
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
                if state.account_labels[src] == AccountLabel::Assets{
                    state.accounts[ASSETS] -= amount;
                }
                if state.account_labels[dst] == AccountLabel::Assets{
                    state.accounts[ASSETS] += amount;
                }
                if src != NULL && dst != NULL {
                    state.accounts[INTERNAL_FLOW] += amount;
                } else if src != NULL && dst == NULL{
                    state.accounts[NET] -= amount;
                    if state.account_labels[src] != AccountLabel::Debt{
                        spending_acc += amount;
                    }
                } else if src == NULL && dst != NULL{
                    state.accounts[NET] += amount;
                    let init = &mut state.account_initialised[dst];
                    if state.account_labels[dst] != AccountLabel::Debt && *init{
                        receiving_acc += amount;
                    }
                    *init = true;
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
                if state.account_labels[src] == AccountLabel::Assets{
                    state.accounts[ASSETS] -= sub;
                }
                if state.account_labels[dst] == AccountLabel::Assets{
                    state.accounts[ASSETS] += add;
                }
                let diff = add - sub;
                state.accounts[TRA] += diff;
                if src != NULL && dst != NULL{
                    state.accounts[INTERNAL_FLOW] += sub.max(add);
                    state.accounts[NET] += diff;
                } else if src != NULL && dst == NULL{
                    state.accounts[NET] -= sub;
                    if state.account_labels[src] != AccountLabel::Debt{
                        spending_acc += sub;
                    }
                } else if src == NULL && dst != NULL{
                    state.accounts[NET] += add;
                    let init = &mut state.account_initialised[dst];
                    if state.account_labels[dst] != AccountLabel::Debt && *init{
                        receiving_acc += sub;
                    }
                    *init = true;
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
    pub account_initialised: Vec<bool>,
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
            account_initialised: vec![false; nb.accounts.next_id],
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
        self.account_id("_assets".to_owned());
        self.account_id("_tra".to_owned());
        self.account_id("_yield".to_owned());
        self.account_id("_roi".to_owned());
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

pub struct Trans{
    date: Date,
    tags: Vec<usize>,
    ext: TransExt,
}

pub type TransRes = Option<Result<Trans, TransErr>>;

#[derive(Debug)]
pub enum TransErr {
    UnknownCommand(String),
    NotEnoughFields(String),
    DateFields,
    ParseError(String, String),
    FloatError(String, String, String),
    MultipleFloats(String),
}

impl std::fmt::Display for TransErr{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result{
        match self{
            TransErr::UnknownCommand(cmd) => write!(f, "Unknown command: {}", cmd),
            TransErr::NotEnoughFields(field) => write!(f, "Not enough fields (comma separated) for {}", field),
            TransErr::DateFields => write!(f, "A date needs 3 fields (day/month/year)"),
            TransErr::ParseError(field, wrong) => write!(f, "Could not parse '{}' in field '{}'", wrong, field),
            TransErr::FloatError(field, wrong, error) => write!(f, "Could not parse '{}' in float '{}': {}", wrong, field, error),
            TransErr::MultipleFloats(field) => write!(f, "Field '{}' returned more than one floating point value", field),
        }
    }
}

pub trait IntoTrans{
    fn into_trans(self, state: &mut NameBank, date: &mut Date) -> TransRes;
}

impl IntoTrans for String{
    fn into_trans(self, nb: &mut NameBank, date: &mut Date) -> TransRes{
        if self.is_empty() { return None; }
        if self.starts_with('#') { return None; }
        let splitted = self.split(',').collect::<Vec<_>>();
        if splitted.len() < 2 { return Some(Err(TransErr::NotEnoughFields("any command".to_string()))); }

        macro_rules! parse_field{
            ($string:expr, $field:expr) => {
                match tbl::string_to_value($string){
                    Some(x) => x,
                    None => return Some(Err(TransErr::ParseError(
                                $field.to_string(),
                                $string.to_string()))),
                }
            }
        }

        macro_rules! parse_float{
            ($string:expr, $field:expr) => {
                match mexprp::eval::<f64>($string){
                    Ok(ans) => match ans{
                        mexprp::Answer::Single(f) => f as f32,
                        _ => return Some(Err(TransErr::MultipleFloats($field.to_string()))),
                    },
                    Err(err) => return Some(Err(TransErr::FloatError(
                                $field.to_string(),
                                $string.to_string(),
                                format!("{}", err)
                    ))),
                }
            }
        }

        macro_rules! parse_date{
            ($output:expr, $string:expr) => {
                let triple = $string.split('/').collect::<Vec<_>>();
                if triple.len() != 3 { return Some(Err(TransErr::DateFields)); }
                $output = (
                    parse_field!(triple[0], "day"),
                    parse_field!(triple[1], "month"),
                    parse_field!(triple[2], "year"),
                );
            }
        }

        macro_rules! check_fields{
            ($nr:expr, $field:expr) => {
                if splitted.len() < $nr{
                    return Some(Err(TransErr::NotEnoughFields($field.to_string())))
                }
            }
        }

        if splitted[1] != "_"{
            parse_date!(*date, splitted[1]);
        }
        let tags_ind;
        let ext = match splitted[0]{
            "dat" => {
                check_fields!(2, "date");
                parse_date!(*date, splitted[1]);
                return None;
            },
            "mov" => {
                tags_ind = 6;
                check_fields!(5, "mov");
                TransExt::Mov{
                    src: nb.account_id(splitted[2].to_string()),
                    dst: nb.account_id(splitted[3].to_string()),
                    amount: parse_float!(splitted[4], "amount"),
                }
            },
            "set" => {
                tags_ind = 5;
                check_fields!(4, "set");
                TransExt::Set{
                    dst: nb.account_id(splitted[2].to_string()),
                    amount: parse_float!(splitted[3], "amount"),
                }
            },
            "tra" => {
                tags_ind = 7;
                check_fields!(6, "tra");
                TransExt::Tra{
                    src: nb.account_id(splitted[2].to_string()),
                    dst: nb.account_id(splitted[3].to_string()),
                    sub: parse_float!(splitted[4], "sub"),
                    add: parse_float!(splitted[5], "add"),
                }
            },
            "dec" => {
                tags_ind = 4;
                check_fields!(4, "dec");
                TransExt::Dec{
                    asset: nb.asset_id(splitted[2].to_string()),
                    amount: parse_float!(splitted[3], "amount"),
                }
            },
            "pri" => {
                tags_ind = 5;
                check_fields!(5, "pri");
                TransExt::Pri{
                    asset: nb.asset_id(splitted[2].to_string()),
                    amount: tbl::string_to_value(splitted[3])?,
                    worth: parse_float!(splitted[4], "worth"),
                }
            },
            "pin" => {
                tags_ind = 5;
                check_fields!(5, "pin");
                TransExt::Pin{
                    asset: nb.asset_id(splitted[2].to_string()),
                    amount: tbl::string_to_value(splitted[3])?,
                    worth: parse_float!(splitted[4], "worth"),
                }
            },
            "con" => {
                tags_ind = 6;
                check_fields!(6, "con");
                TransExt::Con{
                    src: nb.asset_id(splitted[2].to_string()),
                    src_amount: parse_float!(splitted[3], "src_amount"),
                    dst: nb.asset_id(splitted[4].to_string()),
                    dst_amount: parse_float!(splitted[5], "dst_amount"),
                }
            },
            "ass" => {
                tags_ind = 3;
                check_fields!(3, "ass");
                TransExt::Ass{
                    account: nb.account_id(splitted[2].to_string()),
                }
            },
            "deb" => {
                tags_ind = 3;
                check_fields!(3, "deb");
                TransExt::Deb{
                    account: nb.account_id(splitted[2].to_string()),
                }
            },
            _ => return Some(Err(TransErr::UnknownCommand(splitted[0].to_string()))),
        };
        let tags = splitted.into_iter().skip(tags_ind).map(|raw_tag| nb.tag_id(raw_tag.to_string()))
            .collect::<Vec<_>>();

        Some(Ok(Trans{
            date: *date, tags, ext
        }))
    }
}
