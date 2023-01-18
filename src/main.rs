#![feature(portable_simd)]

use std::mem;
use std::simd::{isizex8, Simd};

const WORDLIST: &str = include_str!("../assets/collins_srabble_words_2019.txt");

fn main() {
    let board = Board::from_str("e o l o e e t o o o y k f d e qu f i s e o v e e s").unwrap();

    println!("Board:\n{}", board.display_str());

    let list_raw = WORDLIST
        .split("\n\r")
        .skip(2) // title section of list
        .map(|word| {
            assert!(
                word.chars().map(|c| c.is_ascii_alphabetic()).all(|x| x),
                "{word:?}"
            );
            word.to_string()
        })
        .collect::<Vec<_>>();
    let list = WordList::from_list(&list_raw);

    println!("Created word list");

    unsafe {
        let next = asdf(&board, list.0.into(), (2, 2));
        println!("{:?}", next.0.map(|i| idx_to_ch(i)));
    }
}

pub unsafe fn asdf(
    board: &Board,
    cwl: WordPart,
    idx: (usize, usize),
) -> ([usize; 8], [Option<Box<WordPart>>; 8]) {
    const B: isize = BOARD_SIZE as isize;
    const OFFSETS: isizex8 = Simd::from_array([-B - 1, -B, -B + 1, -1, 1, B - 1, B, B + 1]);
    // negative values (where idx + offset < 0) are wrapped to VERY large usize values,
    // which does the appropreate
    let mut idxs = (Simd::splat((idx.0 + idx.0 * BOARD_SIZE) as isize) + OFFSETS).cast::<usize>();
    if idx.0 == 0 {
        idxs &= Simd::from_array([
            0,
            usize::MAX,
            usize::MAX,
            0,
            usize::MAX,
            0,
            usize::MAX,
            usize::MAX,
        ]);
    } else if idx.0 == BOARD_SIZE - 1 {
        idxs &= Simd::from_array([
            usize::MAX,
            usize::MAX,
            0,
            usize::MAX,
            0,
            usize::MAX,
            usize::MAX,
            0,
        ]);
    }
    let letter_idxs = Simd::gather_or(&board.0, idxs, Simd::splat(32usize));
    let next_letters = Simd::gather_or(
        mem::transmute::<&[WordPart], &[usize]>(cwl.as_slice()),
        letter_idxs,
        Simd::splat(0),
    )
    .to_array();
    (
        letter_idxs.to_array(),
        // Null ptr indicates no word in that direction
        mem::transmute::<[usize; 8], [*mut WordPart; 8]>(next_letters).map(|ptr| {
            if ptr.is_null() {
                None
            } else {
                Some(Box::from_raw(ptr))
            }
        }),
    )
}

#[repr(transparent)]
#[derive(Debug, Clone)]
pub struct Board([usize; 25]);
const BOARD_SIZE: usize = 5;

impl Board {
    pub fn from_str(s: &str) -> Option<Board> {
        Some(Board(
            s.split(" ")
                .map(|s| match s.len() {
                    1 => ch_to_idx(s.chars().nth(0).unwrap()),
                    2 if s == "qu" => ch_to_idx('q'),
                    _ => None,
                })
                .collect::<Option<Vec<usize>>>()?
                .try_into()
                .ok()?,
        ))
    }

    pub fn display_str(&self) -> String {
        const Q_IDX: usize = if let Some(q) = ch_to_idx('q') {
            q
        } else {
            panic!()
        };
        let mut out = String::new();
        for (i, x) in self.0.chunks(5).enumerate() {
            if i != 0 {
                out.push_str("---------------\n");
            }
            for (i, c) in x.iter().enumerate() {
                if i != 0 {
                    out.push_str("|");
                }
                if *c != Q_IDX {
                    out.push_str(&format!("{} ", idx_to_ch(*c).unwrap()))
                } else {
                    out.push_str("QU")
                }
            }
            out.push_str("\n")
        }
        out
    }
}

pub const fn ch_to_idx(ch: char) -> Option<usize> {
    if ch.is_ascii_alphabetic() {
        Some(ch.to_ascii_uppercase() as usize - 65)
    } else {
        None
    }
}

pub fn idx_to_ch(idx: usize) -> Option<char> {
    if idx >= 26 {
        return None;
    }
    char::try_from(idx as u32 + 65).ok()
}

#[test]
fn ch_to_idx_correct_start() {
    assert_eq!(ch_to_idx('a'), Some(0));
}

#[derive(Debug)]
pub struct WordList(pub [WordPart; 26]);

impl WordList {
    pub fn from_list(list: &[String]) -> Self {
        Self(WordPart::from_collection(list))
    }
}

#[repr(transparent)]
#[derive(Debug)]
pub struct WordPart(*mut [WordPart; 26]);
// #[derive(Debug)]
// pub enum WordPart {
//     More(Box<[WordPart; 26]>),
//     End,
// }

impl WordPart {
    pub fn from_collection(words: &[String]) -> [Self; 26] {
        let mut starting_with: [Vec<String>; 26] = Default::default();
        for word in words {
            let mut iter = word.chars();
            if let Some(c) = iter.nth(0) {
                starting_with[ch_to_idx(c).unwrap()].push(iter.collect())
            }
        }
        starting_with.map(|e| {
            if e.is_empty() {
                Self(std::ptr::null_mut())
            } else {
                Self::from_collection(&e).into()
            }
        })
    }

    pub fn as_slice(&self) -> &[WordPart] {
        unsafe { &*self.0 }
    }
}

impl From<[WordPart; 26]> for WordPart {
    fn from(value: [WordPart; 26]) -> Self {
        Self(Box::into_raw(Box::new(value)))
    }
}

impl Drop for WordPart {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { drop(Box::from_raw(self.0)) }
        }
    }
}
