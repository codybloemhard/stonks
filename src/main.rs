use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::collections::{ HashMap, HashSet };
use std::process::Command;

use term_basics_linux as tbl;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let contents = fs::read_to_string(&args[1]).expect("Couldn't read sample.");
    let mut state = State::new();
    let ts = contents.split('\n').into_iter().map(|line| line.to_string().into_trans(&mut state))
        .flatten().collect::<Vec<_>>();
    summary(&state, &ts);
    graph(&state, &ts, true);
}

pub fn graph(state: &State, ts: &[Trans], skip_null: bool){
    let null_skip = if skip_null { 1 } else { 0 };
    let hist = time_hist(state, ts);
    let mut page = String::new();
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
                        titleColor: '#FFF',
                        title: 'Net worth',
                        backgroundColor: '#444',
                        lineWidth: 5,
                        legend: {
                            position: 'bottom',
                            textStyle:{ color: '#FFF' }
                        },
                        colors:['#F00', '#0F0', '#00F' ],
                        hAxis:{ textStyle:{ color: '#FFF' } },
                        vAxis:{ textStyle:{ color: '#FFF' } },
                    };
                    var chart = new google.visualization.LineChart(document.getElementById('line_chart'));
                    chart.draw(data, options);
                }
            </script>
        </head>
        <body style=\"background: #222;\">
            <div id=\"line_chart\" style=\"width: 100%; height: 500px; background: #222;\"></div>
        </body>
    </html>";
    page.push_str(head);
    page.push('[');
    page.push_str("\'Date\',");
    (null_skip..state.ids.next_id).into_iter().for_each(|id| page.push_str(&format!("\'{}\',", state.name(id))));
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
        bs.iter().skip(null_skip).for_each(|v| page.push_str(&format!("{},", v.1.to_string())));
        page.push_str("],\n");
    }
    page.push_str(tail);
    let mut file = File::create("graph.html").expect("Could not create file!");
    file.write_all(page.as_bytes()).expect("Could not write to file!");
    Command::new("firefox").arg("graph.html").output().expect("Could not open graph in firefox!");
}

pub fn summary(state: &State, ts: &[Trans]){
    let mut accounts = vec![0i64; state.ids.next_id];
    update(ts, &mut accounts, None, None);
    let bs = into_nameds(accounts.into_balances(), state);
    for (name, amount) in &bs{
        println!("{}: {}", name, amount);
    }
    println!("Your life is worth {} EUR.", bs.sum());
}

pub type Balance = (usize, i64);
pub type NamedBalance = (String, i64);

pub trait Sumable{
    fn sum(&self) -> i64;
}

impl Sumable for Vec<Balance>{
    fn sum(&self) -> i64{
        self.iter().filter(|(id, _)| id != &0).fold(0, |sum, (_, amount)| sum + amount)
    }
}

impl Sumable for Vec<NamedBalance>{
    fn sum(&self) -> i64{
        self.iter().filter(|(name, _)| name != "null").fold(0, |sum, (_, amount)| sum + amount)
    }
}

pub trait IntoBalances{
    fn into_balances(self) -> Vec<Balance>;
}

impl IntoBalances for Vec<i64>{
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
    let mut accounts = vec![0i64; state.ids.next_id];
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

pub fn update(ts: &[Trans], accounts: &mut Vec<i64>, from: Option<usize>, mut date: Option<(u8, u16)>) -> (usize, Option<(u8, u16)>){
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
                accounts[trans.dst] = amount as i64;
            },
            TransExt::Mov { src, amount } => {
                accounts[src] -= amount as i64;
                accounts[trans.dst] += amount as i64;
            },
            TransExt::Tra { src, sub, add } => {
                accounts[src] -= sub as i64;
                accounts[trans.dst] += add as i64;
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
    names: HashMap<usize, String>,
    ins: HashSet<usize>,
    tags: Ider,
}

impl State{
    pub fn new() -> Self{
        Self{
            ids: Ider::new(),
            names: HashMap::new(),
            ins: HashSet::new(),
            tags: Ider::new(),
        }
    }

    pub fn account_id(&mut self, string: String, set_in: bool) -> usize{
        let id = self.ids.get_id(string.clone());
        if set_in{
            self.ins.insert(id);
        }
        self.names.insert(id, string);
        id
    }

    pub fn name(&self, id: usize) -> String{
        if id == 0 {
            String::from("null")
        } else if let Some(name) = self.names.get(&id){
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
        amount: usize,
    },
    Set{
        amount: usize,
    },
    Tra{
        src: usize,
        sub: usize,
        add: usize,
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
                    src: state.account_id(splitted[2].to_string(), false),
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
                    src: state.account_id(splitted[2].to_string(), false),
                    sub: tbl::string_to_value(splitted[4])?,
                    add: tbl::string_to_value(splitted[5])?,
                }
            }
            _ => return None,
        };
        let dst = state.account_id(splitted[indices.0].to_string(), true);
        let comment = splitted[indices.1].to_string();
        let tags = splitted.into_iter().skip(indices.2).map(|raw_tag| state.tag_id(raw_tag.to_string()))
            .collect::<Vec<_>>();

        Some(Trans{
            date, dst, comment, tags, ext
        })
    }
}
