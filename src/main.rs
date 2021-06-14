use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::collections::{ HashMap };
use std::process::Command;

use term_basics_linux as tbl;

const NULL: usize = 0;
const FLOW: usize = 1;
const NET: usize = 2;
const NET_LOST: usize = 3;
const NET_GAINED: usize = 4;
const TRA: usize = 5;
const TRA_LOST: usize = 6;
const TRA_GAINED: usize = 7;
const YIELD: usize = 8;
const YIELD_LOST: usize = 9;
const YIELD_GAINED: usize = 10;
const INTERNAL_FLOW: usize = 11;

fn main() {
    let args = lapp::parse_args("
        Tells you how poor you are.
        -a, --accounts (string...) accounts to graph
        <file> (string) transactional \"database\" file
    ");
    let infile = args.get_string("file");
    let contents = fs::read_to_string(infile).expect("Couldn't read sample.");
    let mut state = State::new();
    let ts = contents.split('\n').into_iter().map(|line| line.to_string().into_trans(&mut state))
        .flatten().collect::<Vec<_>>();
    summary(&state, &ts);

    let includes = args.get_strings("accounts");
    if !includes.is_empty(){
        graph(&state, &ts, &includes.iter().map(|s| s.as_str()).collect::<Vec<_>>());
    }
}

pub fn graph(state: &State, ts: &[Trans], include: &[&str]){
    let hist = time_hist(state, ts);
    let mut page = String::new();
    // Nord theme used
    // https://www.nordtheme.com/docs/colors-and-paletteshttps://www.nordtheme.com/docs/colors-and-palettes
    let head = "
<html>
    <head>
        <script type=\"text/javascript\" src=\"https://www.gstatic.com/charts/loader.js\"></script>
        <script type=\"text/javascript\">
            google.charts.load('current', {'packages':['corechart']});
            google.charts.setOnLoadCallback(drawChart);
            function drawChart() {
                var data = google.visualization.arrayToDataTable([\n";
    let tail = "
                ]);
                var options = {
                    titleColor: '#ECEFF4',
                    title: 'Net worth',
                    backgroundColor: '#2E3440',
                    lineWidth: 5,
                    legend: {
                        position: 'bottom',
                        textStyle:{ color: '#ECEFF4' }
                    },
                    colors:['#BF616A', '#D08770', '#EBCB8B', '#A3BE8C', '#B48EAD'],
                    hAxis:{ textStyle:{ color: '#ECEFF4' } },
                    vAxis:{ textStyle:{ color: '#ECEFF4' } },
                };
                var chart = new google.visualization.LineChart(document.getElementById('line_chart'));
                chart.draw(data, options);
            }
        </script>
    </head>
    <body style=\"background: #2E3440;\">
        <div id=\"line_chart\" style=\"width: 100%; height: 100%; background: #2E3440;\"></div>
    </body>
</html>";
    page.push_str(head);
    page.push('[');
    page.push_str("\'Date\',");
    let mut indices = Vec::new();
    (0..state.ids.next_id).into_iter().for_each(|id| {
        let name = state.name(id);
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
    page.push_str(tail);
    let mut file = File::create("graph.html").expect("Could not create file!");
    file.write_all(page.as_bytes()).expect("Could not write to file!");
    Command::new("firefox").arg("graph.html").output().expect("Could not open graph in firefox!");
}

pub fn summary(state: &State, ts: &[Trans]){
    let mut accounts = vec![0f32; state.ids.next_id];
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

pub fn into_nameds(bs: Vec<Balance>, state: &State) -> Vec<NamedBalance>{
    bs.into_iter().map(|(id, val)| (state.name(id), val)).collect::<Vec<_>>()
}

pub fn time_hist(state: &State, ts: &[Trans]) -> Vec<((u8, u16), Vec<Balance>)>{
    let mut hist = Vec::new();
    let mut from = 0;
    let mut date = None;
    let mut accounts = vec![0f32; state.ids.next_id];
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
            TransExt::Set { amount } => {
                if trans.dst != NULL{
                    let diff = amount - accounts[trans.dst];
                    accounts[NET] += diff;
                    accounts[NET_LOST] += amount.min(0.0);
                    accounts[NET_GAINED] += amount.max(0.0);
                    accounts[YIELD] += diff;
                    accounts[YIELD_LOST] += diff.min(0.0);
                    accounts[YIELD_GAINED] += diff.max(0.0);
                }
                accounts[trans.dst] = amount;
            },
            TransExt::Mov { src, amount } => {
                accounts[src] -= amount;
                accounts[trans.dst] += amount;
                accounts[FLOW] += amount;
                if src != NULL && trans.dst != NULL {
                    accounts[INTERNAL_FLOW] += amount;
                } else if src != NULL && trans.dst == NULL{
                    accounts[NET] -= amount;
                    accounts[NET_LOST] += amount;
                } else if src == NULL && trans.dst != NULL{
                    accounts[NET] += amount;
                    accounts[NET_GAINED] += amount;
                }
            },
            TransExt::Tra { src, sub, add } => {
                accounts[src] -= sub;
                accounts[trans.dst] += add;
                accounts[FLOW] += sub.max(add);
                let diff = add - sub;
                if diff >= 0.0 { accounts[TRA_GAINED] += diff; }
                else if diff < 0.0 { accounts[TRA_LOST] -= diff; }
                accounts[TRA] += diff;
                if src != NULL && trans.dst != NULL{
                    accounts[INTERNAL_FLOW] += sub.max(add);
                    accounts[NET] += diff;
                    accounts[NET_LOST] += diff.min(0.0);
                    accounts[NET_GAINED] += diff.max(0.0);
                } else if src != NULL && trans.dst == NULL{
                    accounts[NET] -= sub;
                    accounts[NET_LOST] += sub;
                } else if src == NULL && trans.dst != NULL{
                    accounts[NET] += add;
                    accounts[NET_GAINED] += add;
                }
            }
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
pub struct State{
    ids: Ider,
    names: HashMap<usize, String>,
    tags: Ider,
}

impl State{
    pub fn new() -> Self{
        let temp = Self{
            ids: Ider::new(),
            names: HashMap::new(),
            tags: Ider::new(),
        };
        temp.set_defaults()
    }

    fn set_defaults(mut self) -> Self{
        self.account_id("null".to_owned());
        self.account_id("_flow".to_owned());
        self.account_id("_net".to_owned());
        self.account_id("_net_lost".to_owned());
        self.account_id("_net_gained".to_owned());
        self.account_id("_tra".to_owned());
        self.account_id("_tra_lost".to_owned());
        self.account_id("_tra_gained".to_owned());
        self.account_id("_yield".to_owned());
        self.account_id("_yield_lost".to_owned());
        self.account_id("_yield_gained".to_owned());
        self.account_id("_internal_flow".to_owned());
        self
    }

    pub fn account_id(&mut self, string: String) -> usize{
        let id = self.ids.get_id(string.clone());
        self.names.insert(id, string);
        id
    }

    pub fn name(&self, id: usize) -> String{
        if let Some(name) = self.names.get(&id){
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
        amount: f32,
    },
    Set{
        amount: f32,
    },
    Tra{
        src: usize,
        sub: f32,
        add: f32,
    }
}

#[derive(Debug)]
pub struct Trans{
    date: (u8, u8, u16),
    dst: usize,
    comment: String,
    tags: Vec<usize>,
    ext: TransExt,
}

trait IntoTrans{
    fn into_trans(self, state: &mut State) -> Option<Trans>;
}

impl IntoTrans for String{
    fn into_trans(self, state: &mut State) -> Option<Trans>{
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
        let indices;
        let ext = match splitted[0]{
            "mov" => {
                indices = (3, 5, 6);
                TransExt::Mov{
                    src: state.account_id(splitted[2].to_string()),
                    amount: tbl::string_to_value(splitted[4])?,
                }
            },
            "set" => {
                indices = (2, 4, 5);
                TransExt::Set{
                    amount: tbl::string_to_value(splitted[3])?,
                }
            },
            "tra" => {
                indices = (3, 6, 7);
                TransExt::Tra{
                    src: state.account_id(splitted[2].to_string()),
                    sub: tbl::string_to_value(splitted[4])?,
                    add: tbl::string_to_value(splitted[5])?,
                }
            }
            _ => return None,
        };
        let dst = state.account_id(splitted[indices.0].to_string());
        let comment = splitted[indices.1].to_string();
        let tags = splitted.into_iter().skip(indices.2).map(|raw_tag| state.tag_id(raw_tag.to_string()))
            .collect::<Vec<_>>();

        Some(Trans{
            date, dst, comment, tags, ext
        })
    }
}
