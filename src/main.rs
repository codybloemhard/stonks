use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::collections::{ HashMap };
use std::process::Command;

use term_basics_linux as tbl;

const NULL: usize = 0;
const FLOW: usize = 1;
const INTERNAL_FLOW: usize = 2;
const NET: usize = 3;
const NET_NEG: usize = 4;
const NET_POS: usize = 5;
const TRA: usize = 6;
const TRA_NEG: usize = 7;
const TRA_POS: usize = 8;
const YIELD: usize = 9;
const YIELD_NEG: usize = 10;
const YIELD_POS: usize = 11;

fn main() {
    let args = lapp::parse_args("
        Tells you how poor you are.
        -a, --accounts (string...) accounts to graph
        -p, --palette (default \'\') file to read colours from
        -c, --colours (integer...) lines to get colours from (bg, fg, col0, col1, ...)
        <file> (string) transactional \"database\" file
    ");
    let infile = args.get_string("file");
    let contents = fs::read_to_string(infile).expect("Couldn't read sample.");
    let mut namebank = NameBank::new();
    let mut date = Date::default();
    let ts = contents.split('\n').into_iter().map(|line| line.to_string()
        .into_trans(&mut namebank, &mut date)).flatten().collect::<Vec<_>>();
    summary(&namebank, &ts);

    let colours = get_graph_colours(&args);
    let includes = args.get_strings("accounts");
    if !includes.is_empty(){
        graph(&namebank, &ts, &includes.iter().map(|s| s.as_str()).collect::<Vec<_>>(), colours);
    }
}

pub fn get_graph_colours(args: &lapp::Args) -> Vec<String>{
    let mut colours = Vec::new();
    let palette_source = args.get_string("palette");
    if !palette_source.is_empty(){
        let contents = fs::read_to_string(palette_source).expect("Couldn't read palette file.");
        let clines = contents.split('\n').collect::<Vec<_>>();
        let colour_indices = args.get_integers("colours");
        for ind in colour_indices{
            if ind < 0 || ind >= clines.len() as i32{
                continue;
            }
            let mut builder = String::new();
            let mut state = -1;
            for c in clines[ind as usize].chars(){
                if state == -1 {
                    if c == '#' {
                        state = 0;
                        builder.push(c);
                    }
                } else if state < 7 {
                    state += 1;
                    builder.push(c);
                } else {
                    break;
                }
            }
            colours.push(builder);
        }
    }
    let preset = vec!["#2E2440", "#ECEFF4", "#BF616A", "#D08770", "#EBCB8B", "#A3BE8C", "#B48EAD"];
    for (i, colour) in preset.iter().enumerate().take(7){
        if colours.len() < i{
            colours.push(colour.to_owned().to_string());
        }
    }
    colours
}

pub fn graph(state: &NameBank, ts: &[Trans], include: &[&str], colours: Vec<String>){
    let hist = time_hist(state, ts);
    let mut page = String::new();
    // Nord theme used
    // https://www.nordtheme.com/docs/colors-and-paletteshttps://www.nordtheme.com/docs/colors-and-palettes
    let mut carray = String::new();
    carray.push('[');
    for c in colours.iter().skip(2){
        carray.push('\'');
        carray.push_str(c);
        carray.push_str("\', ");
    }
    carray.push(']');
    let head = "
<html>
    <head>
        <script type=\"text/javascript\" src=\"https://www.gstatic.com/charts/loader.js\"></script>
        <script type=\"text/javascript\">
            google.charts.load('current', {'packages':['corechart']});
            google.charts.setOnLoadCallback(drawChart);
            function drawChart() {
                var data = google.visualization.arrayToDataTable([\n";
    let tail = format!("
                ]);
                var options = {{
                    titleColor: '{}',
                    title: 'Net worth',
                    backgroundColor: '{}',
                    lineWidth: 5,
                    legend: {{
                        position: 'bottom',
                        textStyle:{{ color: '{}' }}
                    }},
                    colors:{},
                    hAxis:{{ textStyle:{{ color: '{}' }} }},
                    vAxis:{{ textStyle:{{ color: '{}' }} }},
                }};
                var chart = new google.visualization.LineChart(document.getElementById('line_chart'));
                chart.draw(data, options);
            }}
        </script>
    </head>
    <body style=\"background: {};\">
        <div id=\"line_chart\" style=\"width: 100%; height: 100%; background: {};\"></div>
    </body>
</html>", colours[1], colours[0], colours[1], carray, colours[1], colours[1], colours[0], colours[0]);
//         println!("{}", tail);
    page.push_str(head);
    page.push('[');
    page.push_str("\'Date\',");
    let mut indices = Vec::new();
    (0..state.accounts.next_id).into_iter().for_each(|id| {
        let name = state.account_name(id);
        if include.contains(&&name[..]){
            page.push_str(&format!("\'{}\',", name));
            indices.push(id);
        }
    });
    page.push_str("],\n");
    for ((mm, yy), bs) in hist.into_iter(){
        let format_date = |mm, yy| {
            let m = match mm{
                1 => "Jan",
                2 => "Feb",
                3 => "Mar",
                4 => "Apr",
                5 => "May",
                6 => "Jun",
                7 => "Jul",
                8 => "Aug",
                9 => "Sep",
                10 => "Oct",
                11 => "Nov",
                12 => "Dec",
                _ => "AAA"
            };
            format!("\'{} {}\',", m, yy)
        };
        page.push('[');
        page.push_str(&format_date(mm, yy));
        for ind in &indices{
            page.push_str(&format!("{},", bs[*ind].1.to_string()));
        }
        page.push_str("],\n");
    }
    page.push_str(&tail);
    let mut file = File::create("graph.html").expect("Could not create file!");
    file.write_all(page.as_bytes()).expect("Could not write to file!");
    Command::new("firefox").arg("graph.html").output().expect("Could not open graph in firefox!");
}

pub fn summary(namebank: &NameBank, ts: &[Trans]){
    let mut state = State::new(namebank);
    update(ts, &mut state, None, None);
    let accounts = into_named_accounts(state.accounts.into_balances(), namebank);
    for (name, amount) in &accounts{
        println!("{}: {}", name, amount);
    }
    println!("---------------");
    let amounts = into_named_assets(state.asset_amounts.into_balances(), namebank);
    let prices = into_named_assets(state.asset_prices.into_balances(), namebank);
    let it = amounts.iter().zip(prices.iter());
    let total_assets_worth: f32 = it.fold(0.0, |acc, ((_, a), (_, p))| acc + a * p);
    let mut data_rows = Vec::new();
    for ((name, amount), (_, price)) in amounts.iter().zip(prices.iter()){
        let worth = amount * price;
        data_rows.push((name, amount, worth, price, worth / total_assets_worth));
        println!("{}, {}, {}", name, amount, worth);
    }
    data_rows.sort_by(|(_, _, _, _, sa), (_, _, _, _, sb)|
        sb.partial_cmp(sa).unwrap_or(std::cmp::Ordering::Less));
    println!("Total assets worth: {}", total_assets_worth);
    for (name, amount, worth, price, share) in data_rows{
        println!("{}: {} worth {} priced {} at {}% of total",
            name, amount, worth, price, share * 100.0);
    }
}

pub type Balance = (usize, f32);
pub type NamedBalance = (String, f32);

pub trait IntoBalances{
    fn into_balances(self) -> Vec<Balance>;
}

impl IntoBalances for Vec<f32>{
    fn into_balances(self) -> Vec<Balance>{
        self.into_iter().enumerate().collect::<Vec<_>>()
    }
}

pub fn into_named_accounts(bs: Vec<Balance>, state: &NameBank) -> Vec<NamedBalance>{
    bs.into_iter().map(|(id, val)| (state.account_name(id), val)).collect::<Vec<_>>()
}

pub fn into_named_assets(bs: Vec<Balance>, state: &NameBank) -> Vec<NamedBalance>{
    bs.into_iter().map(|(id, val)| (state.asset_name(id), val)).collect::<Vec<_>>()
}

pub fn time_hist(namebank: &NameBank, ts: &[Trans]) -> Vec<((u8, u16), Vec<Balance>)>{
    let mut hist = Vec::new();
    let mut from = 0;
    let mut date = None;
    let mut state = State::new(namebank);
    let mut i = 0;
    loop{
        let mmyy = (ts[from].date.1, ts[from].date.2);
        let (new_from, new_date) = update(ts, &mut state, Some(from), date);
        hist.push((mmyy, state.accounts.clone().into_balances()));
        if new_from >= ts.len(){
            break;
        }
        from = new_from;
        date = new_date;
        i += 1;
        if i > 10 { break; }
    }
    hist
}

pub fn update(ts: &[Trans], state: &mut State, from: Option<usize>, mut date: Option<(u8, u16)>) -> (usize, Option<(u8, u16)>){
    let skip = if let Some(skip) = from { skip } else { 0 };
    let all = from.is_none();
    for (i, trans) in ts.iter().skip(skip).enumerate(){
        if let Some((m,y)) = date{
            if !all && (trans.date.1 != m || trans.date.2 != y){
                let next = skip + i;
                date = Some((trans.date.1, trans.date.2));
                return (next, date);
            }
        } else {
            date = Some((trans.date.1, trans.date.2));
        }
        match trans.ext{
            TransExt::Set { amount, dst } => {
                if dst != NULL{
                    let diff = amount - state.accounts[dst];
                    state.accounts[NET] += diff;
                    state.accounts[NET_NEG] += diff.min(0.0);
                    state.accounts[NET_POS] += diff.max(0.0);
                    state.accounts[YIELD] += diff;
                    state.accounts[YIELD_NEG] += diff.min(0.0);
                    state.accounts[YIELD_POS] += diff.max(0.0);
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
                } else if src == NULL && dst != NULL{
                    state.accounts[NET] += amount;
                    state.accounts[NET_POS] += amount;
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
                } else if src == NULL && dst != NULL{
                    state.accounts[NET] += add;
                    state.accounts[NET_POS] += add;
                }
            },
            TransExt::Pri { asset, amount, worth } => {
                state.asset_prices[asset] = worth / amount;
            },
            TransExt::Con { src, src_amount, dst, dst_amount } => {
                state.asset_amounts[src] -= src_amount;
                state.asset_amounts[dst] += dst_amount;
            },
        }
    }
    (usize::MAX, date)
}

pub struct State{
    pub accounts: Vec<f32>,
    pub asset_amounts: Vec<f32>,
    pub asset_prices: Vec<f32>,
}

impl State{
    pub fn new(nb: &NameBank) -> Self{
        Self{
            accounts: vec![0.0; nb.accounts.next_id],
            asset_amounts: vec![0.0; nb.assets.next_id],
            asset_prices: vec![0.0; nb.assets.next_id],
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
    Pri{
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
}

pub type Date = (u8, u8, u16);

#[derive(Debug)]
pub struct Trans{
    date: Date,
    tags: Vec<usize>,
    ext: TransExt,
}

trait IntoTrans{
    fn into_trans(self, state: &mut NameBank, date: &mut Date) -> Option<Trans>;
}

impl IntoTrans for String{
    fn into_trans(self, state: &mut NameBank, date: &mut Date) -> Option<Trans>{
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
                    src: state.account_id(splitted[2].to_string()),
                    dst: state.account_id(splitted[3].to_string()),
                    amount: tbl::string_to_value(splitted[4])?,
                }
            },
            "set" => {
                tags_ind = 5;
                TransExt::Set{
                    dst: state.account_id(splitted[2].to_string()),
                    amount: tbl::string_to_value(splitted[3])?,
                }
            },
            "tra" => {
                tags_ind = 7;
                TransExt::Tra{
                    src: state.account_id(splitted[2].to_string()),
                    dst: state.account_id(splitted[3].to_string()),
                    sub: tbl::string_to_value(splitted[4])?,
                    add: tbl::string_to_value(splitted[5])?,
                }
            },
            "pri" => {
                tags_ind = 5;
                TransExt::Pri{
                    asset: state.asset_id(splitted[2].to_string()),
                    amount: tbl::string_to_value(splitted[3])?,
                    worth: tbl::string_to_value(splitted[4])?,
                }
            },
            "con" => {
                tags_ind = 6;
                TransExt::Con{
                    src: state.asset_id(splitted[2].to_string()),
                    src_amount: tbl::string_to_value(splitted[3])?,
                    dst: state.asset_id(splitted[4].to_string()),
                    dst_amount: tbl::string_to_value(splitted[5])?,
                }
            },
            _ => return None,
        };
        let tags = splitted.into_iter().skip(tags_ind).map(|raw_tag| state.tag_id(raw_tag.to_string()))
            .collect::<Vec<_>>();

        Some(Trans{
            date: *date, tags, ext
        })
    }
}
