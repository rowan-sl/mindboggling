# mindboggling
blazingly fast and memory safe* boggle solver

## Use
To run, some env vars must be set:
WORDPATH: path to the .txt file of the word list to use
FORMAT: the format of the word list, either
- bnc: designed to work with the [british national text corpus](https://ucrel.lancs.ac.uk/bncfreq/flists.html), spacifically [list 1.1 full list (download)](https://ucrel.lancs.ac.uk/bncfreq/lists/1_1_all_fullalpha.zip)
- plain (RECOMMENDED): just a list of words, seperated by newlines
    - [this one](http://www.mieliestronk.com/corncob_lowercase.txt) seems like a good list to use
RUST_LOG: the log level to use. set to info for normal use

And the boggle board should be provided as a string as the first argument, as shown:

`"s r n d e h u s m a m v c x i n e i i t r l p u k"`

Here, each cell of the board (5x5 is currently configured) is a single letter, seperated by spaces

## TODO:
- fix `qu` tiles
- find a better word list?
