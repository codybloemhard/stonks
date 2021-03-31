use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::collections::{ HashMap, HashSet };
use std::process::Command;

use term_basics_linux as tbl;

fn main() {
    let contents = fs::read_to_string("sample.csv").expect("Couldn't read sample.");
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
    let mut accounts = vec![0i64; state.ids.next_id];
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
        bs.into_iter().enumerate().for_each(|(i, (_, v))| accounts[i] += v);
        accounts.iter().skip(null_skip).for_each(|v| page.push_str(&format!("{},", v.to_string())));
        page.push_str("],\n");
    }
    page.push_str(tail);
    let mut file = File::create("graph.html").expect("Could not create file!");
    file.write_all(page.as_bytes()).expect("Could not write to file!");
    Command::new("firefox").arg("graph.html").output().expect("Could not open graph in firefox!");
}

pub fn summary(state: &State, ts: &[Trans]){
    let (_, bs) = gradient(state, ts, None);
    let bs = into_nameds(bs, state);
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

pub fn into_nameds(bs: Vec<Balance>, state: &State) -> Vec<NamedBalance>{
    bs.into_iter().map(|(id, val)| (state.name(id), val)).collect::<Vec<_>>()
}

pub fn time_hist(state: &State, ts: &[Trans]) -> Vec<((u8, u16), Vec<Balance>)>{
    let mut hist = Vec::new();
    let mut from = 0;
    loop{
        let mmyy = (ts[from].date.1, ts[from].date.2);
        let (new_from, bs) = gradient(state, ts, Some(from));
        hist.push((mmyy, bs));
        if from == ts.len() - 1 {
            break;
        }
        from = new_from;
    }
    hist
}

pub fn gradient(state: &State, ts: &[Trans], from: Option<usize>) -> (usize, Vec<Balance>){
    let mut accounts = vec![0i64; state.ids.next_id];
    let mut date = None;
    let skip = if let Some(skip) = from { skip } else { 0 };
    let mut next = 0;
    for (i, trans) in ts.iter().skip(skip).enumerate(){
        if let Some((_,m,y)) = date{
            if trans.date.1 != m || trans.date.2 != y{
                if from.is_some(){
                    next = skip + i;
                }
                break;
            }
        } else if from.is_some(){
            date = Some(trans.date);
        }
        accounts[trans.src] -= trans.amount as i64;
        accounts[trans.dst] += trans.amount as i64;
    }
    (next, accounts.into_iter().enumerate().collect::<Vec<_>>())
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
