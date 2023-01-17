const WORDLIST: &str = include_str!("../assets/collins_srabble_words_2019.txt");


fn main() {
    let board_base: [String; 25] = "e o l o e e t o o o y k f d e qu f i s e o v e e s"
        .split(" ")
        .map(|s| s.to_string())
        .collect::<Vec<String>>()
        .try_into()
        .unwrap();

    let list: Vec<String> = WORDLIST 
        .split("\n\r")
        .skip(2)// title section of list
        .map(|word| {
            assert!(word.chars().map(|c| c.is_ascii_alphabetic()).all(|x|x), "{word:?}");
            word.to_string()
        }) 
        .collect::<Vec<_>>();

    let list_organized = WordPart::from_collection(&list);
    println!("Created word list");

}

pub fn ch_to_idx(ch: char) -> Option<usize> {
    if ch.is_ascii_alphabetic() {
        Some(ch.to_ascii_uppercase() as usize - 65)
    } else { None }
}

#[test]
fn ch_to_idx_correct_start() {
    assert_eq!(ch_to_idx('a'), Some(0));
}

#[derive(Debug)]
enum WordPart {
    More(Box<[WordPart; 26]>),
    End,
}

impl WordPart {
    pub fn from_collection(words: &Vec<String>) -> [Self; 26] {
        let mut starting_with: [Vec<String>; 26] = Default::default();
        for word in words {
            let mut iter = word.chars();
            if let Some(c) = iter.nth(0) {
                starting_with[ch_to_idx(c).unwrap()].push(iter.collect())
            }
        }
        starting_with.map(|e| if e.is_empty() { Self::End } else { Self::from_collection(&e).into()})
    }
}

impl From<[WordPart; 26]> for WordPart {
    fn from(value: [WordPart; 26]) -> Self {
        Self::More(Box::new(value))
    }
}

