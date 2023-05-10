use packed_simd::{cptrx8, isizex8, m8x8, msizex8, u64x8, usizex8, Cast};
use std::{cell::UnsafeCell, hint::black_box, mem, time::Instant};

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
    let list = WordPart::from_collection(&list_raw);

    println!("Created word list");

    unsafe {
        for i in 0..10_000 {
            black_box((
                //asdf(&board, list, (2, 2)),
                asdf_nosimd(&board, list, (2, 2)),
            ));
        }
        let before1 = Instant::now();
        // for i in 0..10_000 {
        //     let next = asdf(&board, list, (2, 2));
        //     black_box(next);
        // }
        let time1 = before1.elapsed();
        let before2 = Instant::now();
        for i in 0..10_000 {
            let next2 = asdf_nosimd(&board, list, (2, 2));
            black_box(next2);
        }
        let time2 = before2.elapsed();
        let before3 = Instant::now();
        for i in 0..10_000 {
            let next3 = asdf2(&board, list, (2, 2));
            black_box(next3);
        }
        let time3 = before3.elapsed();
        println!("simd: {time1:?}\nno simd: {time2:?}\nnew simd:{time3:?}");
        //assert_eq!(next.0, next2.0);
        // let mut b = board;
        // let mut word: Vec<char> = vec![];
        // loop {
        //     b.0[2*BOARD_SIZE_PADDED+2+1] = Tile(usize::MAX);
        //     // next = asdf(&b, *list.1[next_idx], next_coord);
        // }
        // println!("{:?}", next.0.map(|i| i.to_ch()));
    }
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

#[no_mangle]
pub unsafe fn asdf2(
    board: &Board,
    cwl: &'static WordPart,
    idx: (usize, usize),
) -> ([Tile; 8], [Option<&'static WordPart>; 8]) {
    const B: isize = BOARD_SIZE_PADDED as isize;
    const OFFSETS: isizex8 = isizex8::new(-B - 1, -B, -B + 1, -1, 1, B - 1, B, B + 1);
    debug_assert!(
        idx.0 > 0 && idx.0 < BOARD_SIZE_PADDED && idx.1 > 0 && idx.1 < BOARD_SIZE_PADDED,
        "out of range!"
    );
    let m = msizex8::splat(true);
    let z = u64x8::splat(0);
    let tiles = cptrx8::splat(
        board
            .0
            .as_ptr()
            .add(idx.0 + idx.1 * BOARD_SIZE_PADDED)
            .cast(),
    )
    .offset(OFFSETS)
    .read(m, z);

    // let mut tiles = [0u64; 8];
    // let tiles_ptr = i.read(msizex8::splat(true), u64x8::splat(0));
    // tiles_ptr.write_to_slice_unaligned_unchecked(&mut tiles);

    let words = cptrx8::splat(cwl.as_slice().as_ptr().cast())
        .add(tiles.cast())
        .read(m, z);

    (mem::transmute(tiles), mem::transmute(words))
    // let mut tiles = [Tile::invalid(); 8];
    // let mut words = [None; 8];
    // for (n, i) in OFFSETS.into_iter().enumerate() {
    //     let i = (idx.0 + idx.1 * BOARD_SIZE_PADDED).checked_add_signed(i).unwrap_unchecked();
    //     let tile = board.0[i];
    //     tiles[n] = tile;
    //     words[n] = *cwl.as_slice().get(tile.to_idx()).unwrap_unchecked();
    // }
    // (tiles, words)
}

//#[no_mangle]
pub unsafe fn asdf(
    board: &Board,
    cwl: &WordPart,
    idx: (usize, usize),
) -> ([Tile; 8], [*mut WordPart; 8]) {
    const B: isize = BOARD_SIZE_PADDED as isize;
    const OFFSETS: isizex8 = isizex8::new(-B - 1, -B, -B + 1, -1, 1, B - 1, B, B + 1);
    debug_assert!(
        idx.0 > 0 && idx.0 < BOARD_SIZE_PADDED && idx.1 > 0 && idx.1 < BOARD_SIZE_PADDED,
        "out of range!"
    );
    // if an idx is off the side of the board, it will fall in the gutter area (Tiles set to usize::MAX)
    // this can be detected later
    let idxs: usizex8 =
        (isizex8::splat((idx.0 + idx.1 * BOARD_SIZE_PADDED) as isize) + OFFSETS).cast();

    #[inline]
    unsafe fn gather_or(from: &[usize], idxs: usizex8, or: usize) -> usizex8 {
        //saftey check
        //assert_eq!(mem::size_of::<usize>(), mem::size_of::<u64>());
        //this does not work (the cptrx8::read) with usize elements (https://github.com/rust-lang/packed_simd/issues/237)
        let base = from.as_ptr().cast::<u64>();
        let mask: msizex8 = idxs.lt(usizex8::splat(from.len()));
        let alt = u64x8::splat(mem::transmute::<usize, u64>(or));
        cptrx8::splat(base).add(idxs).read(mask, alt).cast()
    }

    #[inline]
    unsafe fn to_array(val: usizex8) -> [usize; 8] {
        let mut slice = [0usize; 8];
        val.write_to_slice_unaligned_unchecked(&mut slice);
        slice
    }

    let letter_idxs = gather_or(
        mem::transmute::<&[Tile], &[usize]>(&board.0),
        idxs,
        usize::MAX,
    );
    let next_letters = gather_or(
        mem::transmute::<&[Option<&'static WordPart>], &[usize]>(cwl.as_slice()),
        letter_idxs,
        0,
    );
    (
        mem::transmute(to_array(letter_idxs)),
        mem::transmute(to_array(next_letters)),
    )
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

#[repr(transparent)]
#[derive(Debug)]
pub struct WordPart(
    [Option<&'static WordPart>; 27], /* 27th entry is for out-of-bounds, allways None*/
);
// #[derive(Debug)]
// pub enum WordPart {
//     More(Box<[WordPart; 26]>),
//     End,
// }

impl WordPart {
    pub fn from_collection(words: &[String]) -> &'static Self {
        let mut starting_with: [Vec<String>; 27] = Default::default();
        for word in words {
            let mut iter = word.chars();
            if let Some(c) = iter.nth(0) {
                starting_with[Tile::from_char(c).unwrap().to_idx()].push(iter.collect())
            }
        }
        Box::leak(Box::new(Self(starting_with.map(|e| {
            if e.is_empty() {
                None
            } else {
                Some(Self::from_collection(&e).into())
            }
        }))))
    }

    pub fn as_slice(&self) -> &[Option<&'static Self>] {
        &self.0
    }
}
