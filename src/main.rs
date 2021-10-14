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
    let mut state = NameBank::new();
    let ts = contents.split('\n').into_iter().map(|line| line.to_string().into_trans(&mut state))
        .flatten().collect::<Vec<_>>();
    summary(&state, &ts);

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

    let includes = args.get_strings("accounts");
    if !includes.is_empty(){
        graph(&state, &ts, &includes.iter().map(|s| s.as_str()).collect::<Vec<_>>(), colours);
    }
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

pub fn summary(state: &NameBank, ts: &[Trans]){
    let mut accounts = vec![0f32; state.accounts.next_id];
    update(ts, &mut accounts, None, None);
    let bs = into_nameds(accounts.into_balances(), state);
    for (name, amount) in &bs{
        println!("{}: {}", name, amount);
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

pub fn into_nameds(bs: Vec<Balance>, state: &NameBank) -> Vec<NamedBalance>{
    bs.into_iter().map(|(id, val)| (state.account_name(id), val)).collect::<Vec<_>>()
}

pub fn time_hist(state: &NameBank, ts: &[Trans]) -> Vec<((u8, u16), Vec<Balance>)>{
    let mut hist = Vec::new();
    let mut from = 0;
    let mut date = None;
    let mut accounts = vec![0f32; state.accounts.next_id];
    let mut i = 0;
    loop{
        let mmyy = (ts[from].date.1, ts[from].date.2);
        let (new_from, new_date) = update(ts, &mut accounts, Some(from), date);
        hist.push((mmyy, accounts.clone().into_balances()));
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

pub fn update(ts: &[Trans], accounts: &mut Vec<f32>, from: Option<usize>, mut date: Option<(u8, u16)>) -> (usize, Option<(u8, u16)>){
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
                    let diff = amount - accounts[dst];
                    accounts[NET] += diff;
                    accounts[NET_NEG] += diff.min(0.0);
                    accounts[NET_POS] += diff.max(0.0);
                    accounts[YIELD] += diff;
                    accounts[YIELD_NEG] += diff.min(0.0);
                    accounts[YIELD_POS] += diff.max(0.0);
                }
                accounts[dst] = amount;
            },
            TransExt::Mov { src, dst, amount } => {
                accounts[src] -= amount;
                accounts[dst] += amount;
                accounts[FLOW] += amount;
                if src != NULL && dst != NULL {
                    accounts[INTERNAL_FLOW] += amount;
                } else if src != NULL && dst == NULL{
                    accounts[NET] -= amount;
                    accounts[NET_NEG] += amount;
                } else if src == NULL && dst != NULL{
                    accounts[NET] += amount;
                    accounts[NET_POS] += amount;
                }
            },
            TransExt::Tra { src, dst, sub, add } => {
                accounts[src] -= sub;
                accounts[dst] += add;
                accounts[FLOW] += sub.max(add);
                let diff = add - sub;
                if diff >= 0.0 { accounts[TRA_POS] += diff; }
                else if diff < 0.0 { accounts[TRA_NEG] -= diff; }
                accounts[TRA] += diff;
                if src != NULL && dst != NULL{
                    accounts[INTERNAL_FLOW] += sub.max(add);
                    accounts[NET] += diff;
                    accounts[NET_NEG] += diff.min(0.0);
                    accounts[NET_POS] += diff.max(0.0);
                } else if src != NULL && dst == NULL{
                    accounts[NET] -= sub;
                    accounts[NET_NEG] += sub;
                } else if src == NULL && dst != NULL{
                    accounts[NET] += add;
                    accounts[NET_POS] += add;
                }
            }
            TransExt::Ass { asset, amount, worth } => {

            },
        }
    }
    (usize::MAX, date)
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
    Ass{
        asset: usize,
        amount: f32,
        worth: f32,
    }
}

#[derive(Debug)]
pub struct Trans{
    date: (u8, u8, u16),
    tags: Vec<usize>,
    ext: TransExt,
}

trait IntoTrans{
    fn into_trans(self, state: &mut NameBank) -> Option<Trans>;
}

impl IntoTrans for String{
    fn into_trans(self, state: &mut NameBank) -> Option<Trans>{
        if self.is_empty() { return None; }
        if self.starts_with('#') { return None; }
        let splitted = self.split(',').collect::<Vec<_>>();
        if splitted.len() < 3 { return None; }
        let triple = splitted[1].split(';').collect::<Vec<_>>();
        if triple.len() != 3 { return None; }
        let date: (u8, u8, u16) = (
            tbl::string_to_value(triple[0])?,
            tbl::string_to_value(triple[1])?,
            tbl::string_to_value(triple[2])?,
        );
        let tags_ind;
        let ext = match splitted[0]{
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
            "ass" => {
                tags_ind = 5;
                TransExt::Ass{
                    asset: state.asset_id(splitted[2].to_string()),
                    amount: tbl::string_to_value(splitted[3])?,
                    worth: tbl::string_to_value(splitted[4])?,
                }
            },
            _ => return None,
        };
        let tags = splitted.into_iter().skip(tags_ind).map(|raw_tag| state.tag_id(raw_tag.to_string()))
            .collect::<Vec<_>>();

        Some(Trans{
            date, tags, ext
        })
    }
}
