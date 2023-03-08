mod core;
mod summary;
mod graph;

use crate::core::*;
use crate::summary::*;
use crate::graph::*;

use std::fs;
use std::collections::HashMap;

fn main() {
    let args = lapp::parse_args("
        Personal finance tool using a transactional database approach
        -r, --redact redact absolute valuations
        -g, --graph draw graph
        -p, --palette (default \'\') file to read colours from
        -c, --colours (integer...) lines to get colours from (bg, fg, col0, col1, ...)
        -b, --browser (default firefox) browser to show graph in
        --graph-accounts (string...) accounts to graph
        --summary-accounts (string...) accounts to include in the summary account listing
        --redact-map (string...) accounts and their redacted name eg. RealName:Stocks0
        --date-year-digits (default 4) how many digits to display a date's year with: [0,1,2,3,4]
        --date-month-digit use a digit instead of a 3 letter name for a date's month
        --value-rounding (default \'\') whole to round to integers, none to never round
        <file> (string) transactional \"database\" file
    ");
    let infile = args.get_string("file");
    let redact = args.get_bool("redact");
    let draw_graph = args.get_bool("graph");
    let contents = fs::read_to_string(infile).expect("Couldn't read sample.");
    let browser = args.get_string("browser");
    let year_digits = args.get_integer("date-year-digits").clamp(0, 4) as u16;
    let use_month_name = !args.get_bool("date-month-digit");
    let redact_list = args.get_strings("redact-map");
    let value_rounding = args.get_string("value-rounding");
    let mut redact_map = HashMap::new();
    for element in redact_list{
        let split = element.split(':').into_iter().collect::<Vec<_>>();
        if split.len() < 2 { continue; }
        redact_map.insert(split[0].to_string(), split[1].to_string());
    }

    let mut namebank = NameBank::new();
    let mut date = Date::default();
    let ts_res = contents.split('\n').into_iter().map(|line| line.to_string()
        .into_trans(&mut namebank, &mut date)).enumerate().collect::<Vec<_>>();
    let mut ts = Vec::new();
    let mut errs = Vec::new();
    for (line, tr) in ts_res{
        match tr {
            Some(Err(e)) => errs.push((line + 1, e)), // lines start at 1, indices at 0
            Some(Ok(t)) => ts.push(t),
            _ => {},
        }
    }
    if !errs.is_empty(){
        println!("The following errors have been found while parsing:");
        for (line, err) in errs{
            println!("  {}:\t{}", line, err);
        }
        return;
    }

    let mut state = State::new(&namebank);
    let (hist, _date) = hist(&mut state, &ts);
    let norm_fac = summary(
        &namebank, &state, &hist, redact, &redact_map,
        &args.get_strings("summary-accounts"), &value_rounding
    );

    if draw_graph{
        let colours = get_graph_colours(&args);
        let includes = args.get_strings("graph-accounts");
        if !includes.is_empty(){
            let includes = includes.iter().map(|s| s.as_str()).collect::<Vec<_>>();
            graph(
                norm_fac, &namebank, &ts, &includes, &redact_map, colours, &browser,
                year_digits, use_month_name
            );
        }
    }
}

