use crate::core::*;

use term_basics_linux::UC;

pub fn summary(namebank: &NameBank, ts: &[Trans], redact: bool, includes: &[String]) -> f32{
    let mut state = State::new(namebank);
    let _hist = hist(&mut state, ts);
    let spending = spending(ts, &mut state);
    let accounts = into_named_accounts(state.accounts.into_balances(), namebank);
    let pos_sum: f32 = accounts.iter().skip(12).map(|(_, x)| if *x > 0.0 { *x } else { 0.0 }).sum();
    let norm_fac = if redact { pos_sum } else { 1.0 };
    let amounts = into_named_assets(state.asset_amounts.into_balances(), namebank);
    let prices = into_named_assets(state.asset_prices.into_balances(), namebank);
    let it = amounts.iter().zip(prices.iter());
    let total_holdings_worth: f32 = it.fold(0.0, |acc, ((_, a), (_, p))| acc + a * p);
    let min_sum = pos_sum.min(total_holdings_worth);
    let net = accounts[NET].1;
    let debt = net - min_sum;
    let r#yield = accounts[YIELD].1;
    let sum_holding_error = pos_sum - total_holdings_worth;
    let real_fiat = amounts[0].1;
    let shadowrealm_fiat = amounts[1].1;
    let fiat_split = real_fiat / total_holdings_worth * 100.0;

    let (textc, infoc, namec, posc, negc, fracc) = (UC::Std, UC::Magenta, UC::Blue, UC::Green, UC::Red, UC::Yellow);
    let pncol = |v: f32| if v < 0.0 { negc } else { posc };
    println!("{}General:", infoc);
    println!("{}Net: {}{}{}.", textc, pncol(net), net, textc);
    println!("{}Debt: {}{}{}.", textc, pncol(debt), debt, textc);
    println!("{}Yield: {}{}{}.", textc, pncol(r#yield), r#yield, textc);
    println!("{}Positive owned sum: {}{}", textc, posc, if redact { 1.0 } else { pos_sum });
    println!("{}Total holdings worth: {}{}",
             textc, posc, total_holdings_worth / norm_fac);
    println!("{}Positive owned sum / holdings error: {}{}{} which is {}{}{}%.",
             textc, pncol(sum_holding_error), sum_holding_error / norm_fac, textc, posc, sum_holding_error.abs() / min_sum * 100.0, textc);

    println!("{}Accounts:", infoc);
    let include_not_everything = !includes.is_empty();
    for (name, amount) in &accounts{
        if include_not_everything && !includes.contains(name){ continue; }
        let val = *amount / norm_fac;
        println!("{}{}: {}{}", namec, name, pncol(val), val);
    }

    println!("{}Distribution:", infoc);
    println!("{t}With a split of {f}{a}{t}% assets and {f}{b}{t}% fiat",
             t = textc, f = fracc, a = 100.0 - fiat_split, b = fiat_split);
    println!("{t}A total of {c}{f}{t} fiat is stuck in the shadowrealm",
             t = textc, c = pncol(shadowrealm_fiat), f = shadowrealm_fiat / norm_fac);

    let mut data_rows = Vec::new();
    for ((name, amount), (_, price)) in amounts.iter().zip(prices.iter()){
        if *price == 0.0 { continue; }
        if *amount < 0.000001 { continue; }
        let worth = amount * price;
        data_rows.push((name, amount, worth, price, worth / total_holdings_worth));
    }
    data_rows.sort_by(|(_, _, _, _, sa), (_, _, _, _, sb)|
        sb.partial_cmp(sa).unwrap_or(std::cmp::Ordering::Less));
    for (name, amount, worth, price, share) in data_rows{
        if !redact{
            println!("{nc}{name}{tc}: {ac}{amount}{tc} worth {wc}{worth}{tc} priced {pc}{price}{tc} at {sc}{share}{tc}% of total",
                tc = textc, nc = namec, name = name, ac = pncol(*amount), amount = amount, wc = pncol(worth), worth = worth,
                pc = pncol(*price), price = price, sc = fracc, share = share * 100.0);
        } else {
            println!("{nc}{name}{tc} at {sc}{share}{tc}% of total",
                tc = textc, nc = namec, name = name, sc = fracc, share = share * 100.0);
        }
    }
    println!("{}Metrics:", infoc);
    let past_12m: f32 = spending.iter().rev().take(12).map(|(v, _)| v).sum();
    println!("{}You spent {}{}{} the past year.",
             textc, pncol(past_12m), past_12m / norm_fac, textc);
    let time_flat = net / past_12m * 12.0;
    let moy = |x: f32| if x.abs() > 24.0 { x / 12.0 } else { x }; // months or years
    let moy_label = |x: f32| if x.abs() > 24.0 { "years" } else { "months" };
    println!("{}Your net worth is {}{}{} {} (no Inflation and ROI)",
        textc, pncol(time_flat), moy(time_flat), textc, moy_label(time_flat));

    let print_time_exp = |inflation_rate: f32, roi_rate: f32|{
        let inflation = (1.0 + (inflation_rate * 0.01)).powf(1.0 / 12.0);
        let roi = (1.0 + (roi_rate * 0.01)).powf(1.0 / 12.0);
        let infc = if inflation > 1.0 { negc } else { posc };
        let roic = if roi > 1.0 { posc } else { negc };
        let mut month_cost = past_12m / 12.0;
        let mut assets = min_sum - real_fiat;
        let mut total = min_sum;
        let mut months = 0.0;
        loop{
            if months >= 1200.0{
                println!("{}Your assets are worth {}100+{} years ({}% Infl., {}% ROI)", textc, posc, textc, inflation_rate, roi_rate);
                return;
            }
            if total > month_cost{
                total -= month_cost;
                month_cost *= inflation;
                months += 1.0;
                assets = assets.min(total);
                total -= assets;
                assets *= roi;
                total += assets;
            } else {
                months += total / month_cost;
                println!("{}Your assets are worth {}{}{} {} ({}{}{}% Infl., {}{}{}% ROI)",
                    textc, pncol(months), moy(months), textc, moy_label(months), infc, inflation_rate, textc, roic, roi_rate, textc);
                return;
            }
        }
    };

    print_time_exp(5.0, -5.0);
    print_time_exp(5.0, 0.0);
    print_time_exp(5.0, 5.0);
    print_time_exp(5.0, 6.0);
    print_time_exp(5.0, 7.0);
    norm_fac
}
