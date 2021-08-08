# stonks
Small tool to track personal finance.
Write important events in a transactional-database-ish like plain text file (see `sample.csv`).
Everything is meant to be written down in one currency.
Meant to track worth among accounts not distribution of assets inside accounts.
Supports moving value(`mov`), transacting value with possible loss or gain(`tra`) and setting the current value(`set`) which is used to track investments gain and loss.
Tracks flow(movement of value), net worth(very negative of course), transaction gain and loss and yield(investments).
Can display simple graph of all your accounts in the browser.
## Usage
### General
To model debt have an account send a positive value to standard build in account `null`.
`null` is not counted as 'yours' and won't show up in networth.
All other accounts are taken to contribute to your worth.
Don't use accounts starting with `_`.
Special accounts start with `_` and track some statistics: `_flow`, `_internal_flow`, `net`, `net_lost`, `net_gained`, `_tra`, `_tra_lost`, `tra_gained`, `_yield`, `_yield_lost`, `_yield_gained`.
### cli
Example:
```cargo run ~/git/misc/stonks.csv -a 'Payment,Saving,Crypto,Stonks' -p ~/scripts/Xst -c '1,2,4,5,6,7,8,9'```

Will try to read colours in the format `#xxxxxx` on the lines 1,2,4,5,6,7,8,9 of file `~/scripts/Xst` which is a Xresources file with colours for the terminal in my case.
The first two colours are the background and foreground colour and the colours after that will be used to draw the lines for accounts.

By default it has the theme Nord, the colours will override the theme one by one as you give them.
Meaning if you give less than 7 of them some of Nord will still be in there.
