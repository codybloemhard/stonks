# stonks
## Usage
For example:
```cargo run ~/git/misc/stonks.csv -a 'Payment,Saving,Crypto,Stonks' -p ~/scripts/Xst -c '1,2,4,5,6,7,8,9'```

Will try to read colours in the format `#xxxxxx` on the lines 1,2,4,5,6,7,8,9 of file `~/scripts/Xst` which is a Xresources file with colours for the terminal in my case.
The first two colours are the background and foreground colour and the colours after that will be used to draw the lines for accounts.

By default it has the theme Nord, the colours will override the theme one by one as you give them.
Meaning if you give less than 7 of them some of Nord will still be in there.
