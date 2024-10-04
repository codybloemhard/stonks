use crate::core::*;

use std::process::Command;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::collections::HashMap;

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
    // Nord theme used
    // https://www.nordtheme.com/docs/colors-and-paletteshttps://www.nordtheme.com/docs/colors-and-palettes
    let preset = ["#2E2440", "#ECEFF4", "#BF616A", "#D08770", "#EBCB8B", "#A3BE8C", "#B48EAD"];
    for (i, colour) in preset.iter().enumerate().take(7){
        if colours.len() < i{
            colours.push(colour.to_owned().to_string());
        }
    }
    colours
}

#[allow(clippy::too_many_arguments)]
pub fn graph(
    norm_fac: f32, nb: &NameBank, ts: &[Trans], include: &[&str],
    redact_map: &HashMap<String, String>, colours: Vec<String>,
    browser: &str, year_digits: u16, use_month_names: bool)
{
    let mut state = State::new(nb);
    let (hist, start_date) = hist(&mut state, ts);
    let mut page = String::new();
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
    (0..nb.next_account_id()).for_each(|id| {
        let name = nb.account_name(id);
        if include.contains(&&name[..]){
            let name = if let Some(redacted) = redact_map.get(&name){
                redacted.to_string()
            } else {
                name
            };
            page.push_str(&format!("\'{}\',", name));
            indices.push(id);
        }
    });
    page.push_str("],\n");
    let mut date = start_date;
    for bs in hist.into_iter(){
        let format_date = |mm, yy| {
            let m = if use_month_names{
                match mm{
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
                }.to_string()
            } else {
                format!("{}", mm)
            };
            let y = format!("{}", yy).chars().rev().take(year_digits as usize).collect::<String>().chars().rev().collect::<String>();
            format!("\'{} {}\',", m, y)
        };
        page.push('[');
        page.push_str(&format_date(date.0, date.1));
        date = if date.0 == 12{
            (1, date.1 + 1)
        } else {
            (date.0 + 1, date.1)
        };
        for ind in &indices{
            page.push_str(&format!("{},", (bs[*ind] / norm_fac)));
        }
        page.push_str("],\n");
    }
    page.push_str(&tail);
    let mut file = File::create("graph.html").expect("Could not create file!");
    file.write_all(page.as_bytes()).expect("Could not write to file!");
    Command::new(browser).arg("graph.html").output().unwrap_or_else(|_| panic!("Could not open graph in {}!", browser));
}

