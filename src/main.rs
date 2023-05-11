#[macro_use] extern crate log;
use std::collections::{HashSet, HashMap};

fn main() {
    pretty_env_logger::init_timed();
    //TODO: properly handle QU
    let board = Board::from_str(
        //"e o l o e e t o o o y k f d e qu f i s e o v e e s"
        std::env::args().nth(1).expect("Pass the board (space seperated tile content) as the first argument")
            .to_ascii_lowercase()
            .as_str(),
    )
    .unwrap();

    info!("Board:\n{}", board.display_str());

    let list_raw = std::fs::read_to_string(std::env::var("WORDPATH").unwrap()).unwrap();

    info!("loaded word list");

    let list_vec = match std::env::var("FORMAT").unwrap().as_str() {
        "bnc" => {
            // parsing for https://ucrel.lancs.ac.uk/bncfreq/flists.html (list 1.1 complete no cut off)
            list_raw
                .split('\n')
                .skip(1)
                .filter_map(|line| {
                    if line.is_empty() {
                        return None;
                    }
                    //println!("line {line:?}");
                    let [
            // the word
            ref word @ ..,
            // part of speech
            part_of_speech,
            // variant
            // if ":", it is just a normal word,
            // if "%", it is what appears to be a variation,
            // if <other> it is a variation of the last one with "%" as its tag.
            //     note that for variations, `word` and `part_of_speech` are "@"
            variant,
            // rounded frequency per million word tokens (down to min of 10)
            freq,
            // number of sectors of the corpus (max 100) in which the word occurs
            range,
            // Dispersion value (Juilland's D) from a minimum of 0.00 to a maximum of 1.00
            dispersion
        ] = line.split('\t').collect::<Vec<_>>()[..] else { panic!("bad formatting") };
                    //TODO: some things (see "in" in the corpus) have * or ~ after the word, which
                    //indicates certain things
                    let word = word.iter().copied().collect::<String>();
                    let freq: f64 = freq.parse().unwrap();
                    let range: f64 = range.parse().unwrap();
                    let dispersion: f64 = dispersion.parse().unwrap();
                    // -- filters --
                    // freq cutoff, boggle does not allow 2 letter words
                    if (freq < 50.0 && range < 30.0) || word.len() <= 2 {
                        None?
                    }
                    Some((word, part_of_speech, variant, freq, range, dispersion))
                })
                .filter_map(|(word, _, _, _, _, _)| {
                    if word != "@" && word.chars().all(|c| c.is_ascii_alphabetic()) {
                        Some(word)
                    } else {
                        // if word != "@" {
                        //     println!("{word:?}");
                        // }
                        None
                    }
                })
                .collect::<Vec<String>>()
        }
        "plain" => {
            list_raw
                .to_ascii_lowercase() // IMPORTANT
                .split("\n")
                .filter_map(|word| {
                    let word = word.trim();
                    // assert!(
                    //     word.chars().map(|c| c.is_ascii_alphabetic()).all(|x| x),
                    //     "{word:?}"
                    // );
                    if !word.chars().all(|c| c.is_ascii_alphabetic()) {
                        println!("skip {word:?} reason NONALPHABETIC");
                        return None
                    }
                    Some(word.to_string())
                })
                .collect::<Vec<_>>()
        }
        _ => panic!("unknown format"),
    };

    info!("parsed word list");

    let list = WordPart::from_collection(&list_vec, false);

    #[allow(unused)]
    fn dbg_wl(part: &'static WordPart, prev: String) {
        for (i, next) in part.next[..=26].iter().enumerate() {
            if let Some(next) = next {
                let s = prev.clone() + Tile(i).to_ch().unwrap().to_string().as_str();
                if next.completes_word {
                    println!("word {s}");
                }
                dbg_wl(*next, s);
            }
        }
    }
    //dbg_wl(list, "".into());

    info!("generated word tree");

    let mut found = HashSet::new();

    for x in 1..=BOARD_SIZE {
        for y in 1..=BOARD_SIZE {
            debug!("run for ({x}, {y})");
            fn iter(
                x: usize,
                y: usize,
                board: Board,
                list: &'static WordPart,
                previous: String,
                previous_coords: Vec<(usize, usize)>,
                n: usize,
                found: &mut HashSet<String>,
            ) {
                let indent = std::iter::repeat("  ").take(n).collect::<String>();
                trace!(
                    "{indent}  iter x {x} y {y} prev {previous:?} valid next {:?}",
                    list.next[..=26]
                        .iter()
                        .enumerate()
                        .filter_map(|(i, x)| {
                            (*x)?;
                            Some(Tile(i).to_ch().unwrap())
                        })
                        .collect::<String>()
                );
                let (tiles, parts) = unsafe { asdf_nosimd(&board, list, (x, y)) };
                for (i, (tile, part)) in tiles.iter().zip(parts.iter()).enumerate() {
                    if *tile == Tile::invalid() {
                        trace!("{indent}    tile #{i} INVALID");
                        continue;
                    }
                    trace!("{indent}    tile #{i} char '{}'", tile.to_ch().unwrap());
                    let this = previous.clone() + tile.to_ch().unwrap().to_string().as_str();
                    let x2 = x
                        .checked_add_signed(match i {
                            0 | 3 | 5 => -1,
                            1 | 6 => 0,
                            2 | 4 | 7 => 1,
                            _ => unreachable!(),
                        })
                        .unwrap();
                    let y2 = y
                        .checked_add_signed(match i {
                            0 | 1 | 2 => -1,
                            3 | 4 => 0,
                            5 | 6 | 7 => 1,
                            _ => unreachable!(),
                        })
                        .unwrap();
                    let mut board2 = board.clone();
                    board2.0[x2 + y2 * BOARD_SIZE_PADDED] = Tile::invalid();
                    let mut previous_coords2 = previous_coords.clone();
                    previous_coords2.push((x2, y2));
                    if let Some(part) = part {
                        if part.completes_word {
                            let again = if !found.insert(this.clone()) { "[repeat]" } else { "[new   ]" };
                            if log_enabled!(log::Level::Trace) {
                                debug!("{indent}      found word {this:?}\t{again} path {previous_coords2:?}");
                            } else {
                                debug!("  found word {this:?}\t{again} path {previous_coords2:?}");
                            }
                        }
                        iter(x2, y2, board2, part, this, previous_coords2, n + 2, found);
                    }
                }
            }
            let at = board.0[x + y * BOARD_SIZE_PADDED];
            if let Some(list_at) = list.next[at.to_idx()] {
                iter(
                    x,
                    y,
                    board.clone(),
                    list_at,
                    at.to_ch().unwrap().to_string(),
                    vec![(x, y)],
                    0,
                    &mut found,
                );
            }
        }
    }
    info!("found words {found:?}");
    // -- count #letter words --
    let mut by_letters: HashMap<usize, usize> = Default::default();
    for word in found {
    by_letters.entry(word.len()).and_modify(|x| *x += 1).or_insert(1);
}
let mut by_letters_vec = by_letters.into_iter().collect::<Vec<_>>();
by_letters_vec.sort_by_key(|x| x.0);
for (letters, ammnt) in &by_letters_vec {
    info!("found {ammnt}\t{letters} letter words");
}
// -- calculate score --
let mut score = 0usize;
for (letters, amnt) in &by_letters_vec {
    score += *amnt * (match letters {
        0 | 1 | 2 => 0,
        3 | 4 => 1,
        5 => 2,
        6 => 3,
        7 => 5,
        8 | _ => 11,
    });
}
info!("score {score}");
}

//#[no_mangle]
pub unsafe fn asdf_nosimd(
    board: &Board,
    cwl: &'static WordPart,
    idx: (usize, usize),
) -> ([Tile; 8], [Option<&'static WordPart>; 8]) {
    const B: isize = BOARD_SIZE_PADDED as isize;
    const OFFSETS: [isize; 8] = [-B - 1, -B, -B + 1, -1, 1, B - 1, B, B + 1];
    debug_assert!(
        idx.0 > 0 && idx.0 < BOARD_SIZE_PADDED && idx.1 > 0 && idx.1 < BOARD_SIZE_PADDED,
        "out of range!"
    );
    let mut tiles = [Tile::invalid(); 8];
    let mut words = [None; 8];
    for (n, i) in OFFSETS.into_iter().enumerate() {
        let i = (idx.0 + idx.1 * BOARD_SIZE_PADDED)
            .checked_add_signed(i)
            .unwrap_unchecked();
        let tile = board.0[i];
        tiles[n] = tile;
        words[n] = *cwl.as_slice().get(tile.to_idx()).unwrap_unchecked();
    }
    (tiles, words)
}

#[repr(transparent)]
#[derive(Debug, Clone)]
pub struct Board([Tile; BOARD_SIZE_PADDED * BOARD_SIZE_PADDED]);
const BOARD_SIZE: usize = 5;
const BOARD_SIZE_PADDED: usize = BOARD_SIZE + 2;

impl Board {
    pub fn from_str(s: &str) -> Option<Board> {
        let parts: [Tile; BOARD_SIZE * BOARD_SIZE] = s
            .split(" ")
            .map(|s| match s.len() {
                1 => Tile::from_char(s.chars().nth(0).unwrap()),
                2 if s == "qu" => Tile::from_char('q'),
                _ => None,
            })
            .collect::<Option<Vec<Tile>>>()?
            .try_into()
            .ok()?;
        let mut board = [Tile::invalid(); BOARD_SIZE_PADDED * BOARD_SIZE_PADDED];
        for y in 0..BOARD_SIZE {
            for x in 0..BOARD_SIZE {
                board[(y + 1) * BOARD_SIZE_PADDED + (x + 1)] = parts[y * BOARD_SIZE + x];
            }
        }
        Some(Board(board))
    }

    pub fn display_str(&self) -> String {
        const Q_IDX: Tile = if let Some(q) = Tile::from_char('q') {
            q
        } else {
            panic!()
        };
        let mut out = String::new();
        for y in 0..BOARD_SIZE {
            if y != 0 {
                out.push_str("---------------\n");
            }
            for x in 0..BOARD_SIZE {
                if x != 0 {
                    out.push_str("|");
                }
                let idx = (y + 1) * BOARD_SIZE_PADDED + (x + 1);
                let c = self.0[idx];
                if c != Q_IDX {
                    out.push_str(&format!("{},{idx}", c.to_ch().unwrap_or('-')));
                } else {
                    out.push_str("QU");
                }
            }
            out.push_str("\n")
        }
        out
    }
}

#[repr(transparent)]
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Tile(pub usize);

impl Tile {
    #[inline]
    pub const fn from_char(ch: char) -> Option<Self> {
        if ch.is_ascii_alphabetic() {
            Some(Self(ch.to_ascii_uppercase() as usize - 65))
        } else {
            None
        }
    }

    #[inline]
    pub const fn invalid() -> Self {
        Tile(26) //entry 27 in WordPart, allways None
    }

    #[inline]
    pub fn to_ch(self) -> Option<char> {
        char::try_from((self.0 as u32).saturating_add(65)).ok()
    }

    #[inline]
    pub fn to_idx(self) -> usize {
        self.0
    }
}

#[test]
fn ch_to_idx_correct_start() {
    assert_eq!(Tile::from_char('a'), Some(Tile(0)));
}

#[derive(Debug)]
pub struct WordPart {
    next: [Option<&'static WordPart>; 27], /* 27th entry is for out-of-bounds, allways None*/
    // if this completes a word
    completes_word: bool,
}
// #[derive(Debug)]
// pub enum WordPart {
//     More(Box<[WordPart; 26]>),
//     End,
// }

impl WordPart {
    pub fn from_collection(words: &[String], completes_word: bool) -> &'static Self {
        let mut starting_with: [(Vec<String>, bool); 27] = Default::default();
        for word in words {
            let mut iter = word.chars();
            if let Some(c) = iter.next() {
                starting_with[Tile::from_char(c).unwrap().to_idx()]
                    .0
                    .push(iter.collect());
                if word.len() == 1 {
                    starting_with[Tile::from_char(c).unwrap().to_idx()].1 |= true;
                }
            }
        }
        Box::leak(Box::new(Self {
            next: starting_with.map(|e| {
                if e.0.is_empty() {
                    None
                } else {
                    Some(Self::from_collection(&e.0, e.1))
                }
            }),
            completes_word,
        }))
    }

    pub fn as_slice(&self) -> &[Option<&'static Self>] {
        &self.next
    }
}
