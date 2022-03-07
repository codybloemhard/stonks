# stonks

Small tool to track personal finance.
Write important events in a transactional-database-ish like plain text file (see `sample.csv`).
All account value related values are meant to be written down in one currency.

Meant to track:
- Worth of accounts, not distribution of assets within accounts.
- Distribution of assets of the portfolio as a whole.
- Statistics such as flow(movement of value), net worth(very negative of course), transaction gain and loss and yield(investments).

Produces:
- A summary containing current account values, asset distribution and some lookahead metrics.
- A graph showing some account values over time.

## Usage

### General

To model debt have an account marked as debt and send a positive value to standard build in account `null`.
`null` is not counted as 'yours' and won't show up in networth.
All other accounts are taken to contribute to your worth.
Don't use accounts starting with `_`.
Special accounts start with `_` and track some statistics: `_flow`, `_internal_flow`, `net`, `net_lost`, `net_gained`, `_tra`, `_tra_lost`, `tra_gained`, `_yield`, `_yield_lost`, `_yield_gained`.

### commands

- `dat`: set date(persistent)
  - dat,date,tags
  - dat,01;01;2021
- `deb`: mark account as debt
  - deb,account,tags
  - deb,mortage
- `ass`: mark account as asset holder
  - ass,account,tags
  - ass,broker-account
- `mov`: move fiat between accounts
  - mov,date,src,dst,amount,tags
  - mov,01;01;2021,payment,saving,100
- `tra`: transaction, move fiat between accounts with a transaction cost
  - tra,date,src,dst,subtract,add,tags
  - tra,01;01;2021,cia0,cia1,1000,985
- `set`: sets the value of investment account, tracking the yield statistics
  - set,date,account,value,tags
  - set,01;01;2021,exchange,2000
- `dec`: declare amount of assets
  - dec,date,asset,amount,tags
  - dec,01;01;2021,ETH,0.1
- `pri`: price an asset
  - pri,date,asset,amount,value,tags
  - pri,01;01;2021,USDC,1000,878
- `pin`: pin an asset to a price, declaring you have x asset worth y
  - pin,date,asset,amount,value,tags
  - pin,01;01;2021,USDC,1000,878
- `con`: convert assets
  - con,date,asset,amount,asset,amount,tags
  - con,01;01;2021,BTC,1,USDC,60000

### cli

Example:
```cargo run ~/git/misc/stonks.csv -g -a 'Payment,Saving,Crypto,Stonks' -p ~/scripts/Xst -c '1,2,4,5,6,7,8,9'```

Will try to read colours in the format `#xxxxxx` on the lines 1,2,4,5,6,7,8,9 of file `~/scripts/Xst` which is a Xresources file with colours for the terminal in my case.
The first two colours are the background and foreground colour and the colours after that will be used to draw the lines for accounts.

By default it has the theme Nord, the colours will override the theme one by one as you give them.
Meaning if you give less than 7 of them some of Nord will still be in there.
