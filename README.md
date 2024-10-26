# stonks

Small tool to track personal finance.
Write important events in a transactional-database-ish like plain text file (see `sample.csv`).
This method of keeping up with your finances is called "plain text accounting" (or "PTA").
I did not know this when starting this project and have thus reinvented the wheel a bit.
Other tools are already available (see: https://plaintextaccounting.org/).
You should probably use them over this (they are already tested in real world applications by many).
I do however keep this alive as I already use it myself.

Meant to track:
- Worth of accounts, not distribution of assets within accounts.
- Distribution of assets of the portfolio as a whole.
- Statistics such as flow(movement of value), net worth, transaction gain and loss, yield(investments), receiving and spending.

Produces:
- A summary containing current account values, asset distribution and some lookahead metrics.
- A graph showing some account values over time.

## Usage

### General

To model debt have an account marked as debt and send a positive value to standard build in account `null`.
`null` is not counted as 'yours' and won't show up in networth.
All other accounts are taken to contribute to your worth.
All account value related values are meant to be written down in one currency.
Don't use accounts starting with `_`.
Special accounts start with `_` and track some statistics: `_flow`, `_internal_flow`, `_net`, `_assets`, `_tra`, `_yield`, `_roi`, `_spending_month`, `_spending_cumulative`, `_receiving_month`, `_receiving_cumulative`.

### commands

- `dat`: set date(persistent)
  - dat,date,tags
  - dat,01/01/2021
- `deb`: mark account as debt
  - deb,account,tags
  - deb,mortage
- `ass`: mark account as asset holder
  - ass,account,tags
  - ass,broker-account
- `stat`: mark account as a statistic (won't show up in net worth etc)
  - stat,account,tags
  - stat,dividends-received
- `mov`: move fiat between accounts
  - mov,date,src,dst,amount,tags
  - mov,01/01/2021,payment,saving,100
- `tra`: transaction, move fiat between accounts with a transaction cost
  - tra,date,src,dst,subtract,add,tags
  - tra,01/01/2021,cia0,cia1,1000,985
- `set`: sets the value of investment account, tracking the yield statistics
  - set,date,account,value,tags
  - set,01/01/2021,exchange,2000
- `dec`: declare amount of assets
  - dec,date,asset,amount,tags
  - dec,01/01/2021,ETH,0.1
- `pri`: price an asset
  - pri,date,asset,amount,value,tags
  - pri,01/01/2021,USDC,1000,878
- `pin`: pin an asset to a price, declaring you have x asset worth y
  - pin,date,asset,amount,value,tags
  - pin,01/01/2021,USDC,1000,878
- `con`: convert assets
  - con,date,asset,amount,asset,amount,tags
  - con,01/01/2021,BTC,1,USDC,60000

### cli

Example:
```
cargo run -- ~/git/misc/stonks.csv -g \
    --summary-accounts 'Payment,Saving,Crypto,Stonks' \
    --graph-accounts '_net,_yield,Payment,Saving,Crypto,Stonks' \
    -p ~/scripts/Xst -c '1,2,4,5,6,7,8,9' --date-year-digits 2
```

Will try to read colours in the format `#xxxxxx` on the lines 1,2,4,5,6,7,8,9 of file `~/scripts/Xst` which is a Xresources file with colours for the terminal in my case.
The first two colours are the background and foreground colour and the colours after that will be used to draw the lines for accounts.

By default it has the theme Nord, the colours will override the theme one by one as you give them.
Meaning if you give less than 7 of them some of Nord will still be in there.

```
 stonks --help
Personal finance tool using a transactional database approach
-r, --redact redact absolute valuations
-g, --graph draw graph
-p, --palette (default '') file to read colours from
-c, --colours (integer...) lines to get colours from (bg, fg, col0, col1, ...)
-b, --browser (default firefox) browser to show graph in
--graph-accounts (string...) accounts to graph
--summary-accounts (string...) accounts to include in the summary account listing
--redact-map (string...) accounts and their redacted name eg. RealName:Stocks0
--date-year-digits (default 4) how many digits to display a date's year with: [0,1,2,3,4]
--date-month-digit use a digit instead of a 3 letter name for a date's month
--value-rounding (default '') whole to round to integers, none to never round
--min-asset-worth (default 1.0) don't list assets worth less
<file> (string) transactional "database" file
```

## License

```
Copyright (C) 2024 Cody Bloemhard

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.
```
